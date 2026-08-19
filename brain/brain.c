/*
 * brain.c — Мозг MangoGeneration (C11)
 *
 * Локальный генератор и преобразователь изображений + анализатор файлов.
 * Работает как автономный исполняемый файл, вызываемый из Rust-ядра:
 *
 *   brain generate --type gradient|pattern|dark [--width W] [--height H] [--output FILE.png]
 *   brain analyze <путь_к_файлу_или_папке>
 *   brain convert <input> <output> [--format PNG|JPEG|BMP|TGA]
 *
 * Выводит JSON на stdout. Собран с публичными заголовками stb (public domain)
 * для декодирования/кодирования изображений.
 */

/* Должно быть определено ДО любых системных заголовков (для realpath/strcasecmp) */
#ifndef _WIN32
#define _POSIX_C_SOURCE 200809L
#endif

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>
#include <stdarg.h>
#include <time.h>

/* Кроссплатформенная работа с файловой системой */
#ifdef _WIN32
#include <windows.h>
#include <direct.h>
#define strcasecmp _stricmp
#else
#include <strings.h>
#include <dirent.h>
#include <sys/stat.h>
#include <unistd.h>
#endif

/* Один исходный файл объявляет реализацию stb */
#define STB_IMAGE_IMPLEMENTATION
#define STB_IMAGE_WRITE_IMPLEMENTATION
#include "vendor/stb_image.h"
#include "vendor/stb_image_write.h"

/* ============================================================
 *  Динамическая строка (простой буфер для сборки JSON)
 * ============================================================ */

typedef struct {
    char *data;
    size_t len;
    size_t cap;
} StrBuf;

static void sb_init(StrBuf *sb) {
    sb->cap = 256;
    sb->len = 0;
    sb->data = (char *)malloc(sb->cap);
    if (sb->data) sb->data[0] = '\0';
}

static void sb_reserve(StrBuf *sb, size_t extra) {
    if (sb->len + extra + 1 > sb->cap) {
        size_t ncap = sb->cap ? sb->cap : 256;
        while (ncap < sb->len + extra + 1) ncap *= 2;
        sb->data = (char *)realloc(sb->data, ncap);
        sb->cap = ncap;
    }
}

static void sb_append(StrBuf *sb, const char *s) {
    size_t l = strlen(s);
    sb_reserve(sb, l);
    memcpy(sb->data + sb->len, s, l);
    sb->len += l;
    sb->data[sb->len] = '\0';
}

static void sb_append_char(StrBuf *sb, char c) {
    sb_reserve(sb, 1);
    sb->data[sb->len++] = c;
    sb->data[sb->len] = '\0';
}

static void sb_printf(StrBuf *sb, const char *fmt, ...) {
    char tmp[4096];
    va_list ap;
    va_start(ap, fmt);
    int n = vsnprintf(tmp, sizeof(tmp), fmt, ap);
    va_end(ap);
    if (n <= 0) return;
    sb_reserve(sb, (size_t)n);
    memcpy(sb->data + sb->len, tmp, (size_t)n);
    sb->len += (size_t)n;
    sb->data[sb->len] = '\0';
}

/* JSON-экранирование строки (без UTF-8 транскодирования) */
static void sb_append_json_string(StrBuf *sb, const char *s) {
    sb_append_char(sb, '"');
    for (const unsigned char *p = (const unsigned char *)s; *p; p++) {
        switch (*p) {
            case '"':  sb_append(sb, "\\\""); break;
            case '\\': sb_append(sb, "\\\\"); break;
            case '\n': sb_append(sb, "\\n");  break;
            case '\r': sb_append(sb, "\\r");  break;
            case '\t': sb_append(sb, "\\t");  break;
            default:
                if (*p < 0x20) sb_printf(sb, "\\u%04x", *p);
                else sb_append_char(sb, (char)*p);
        }
    }
    sb_append_char(sb, '"');
}

/* ============================================================
 *  Случайный генератор (xorshift64 — быстрый и детерминированный)
 * ============================================================ */

static uint64_t rng_state = 0x9E3779B97F4A7C15ULL;

static uint32_t brain_rand(void) {
    rng_state ^= rng_state >> 12;
    rng_state ^= rng_state << 25;
    rng_state ^= rng_state >> 27;
    return (uint32_t)((rng_state * 0x2545F4914F6CDD1DULL) >> 32);
}

static int brain_rand_range(int lo, int hi) {
    if (hi <= lo) return lo;
    return lo + (int)(brain_rand() % (uint32_t)(hi - lo + 1));
}

/* ============================================================
 *  Генерация изображений (RGB-буфер -> PNG через stb)
 * ============================================================ */

static unsigned char *make_gradient(int w, int h, int c1[3], int c2[3],
                                    const char *direction) {
    unsigned char *img = (unsigned char *)malloc((size_t)w * h * 3);
    if (!img) return NULL;

    for (int y = 0; y < h; y++) {
        for (int x = 0; x < w; x++) {
            double t;
            if (strcmp(direction, "horizontal") == 0)
                t = (double)x / (double)w;
            else if (strcmp(direction, "diagonal") == 0)
                t = ((double)x / (double)w + (double)y / (double)h) / 2.0;
            else /* vertical */
                t = (double)y / (double)h;

            size_t i = ((size_t)y * w + x) * 3;
            img[i + 0] = (unsigned char)(c1[0] + (int)((c2[0] - c1[0]) * t));
            img[i + 1] = (unsigned char)(c1[1] + (int)((c2[1] - c1[1]) * t));
            img[i + 2] = (unsigned char)(c1[2] + (int)((c2[2] - c1[2]) * t));
        }
    }
    return img;
}

static unsigned char *make_pattern(int w, int h, int seed) {
    rng_state = (uint64_t)seed * 0x9E3779B97F4A7C15ULL;
    if (seed == 0) rng_state ^= (uint64_t)time(NULL);

    unsigned char *img = (unsigned char *)malloc((size_t)w * h * 3);
    if (!img) return NULL;

    /* Тёмная подложка */
    for (size_t i = 0; i < (size_t)w * h * 3; i++) img[i] = 12;

    int circles = brain_rand_range(20, 60);
    for (int c = 0; c < circles; c++) {
        int cx = brain_rand_range(-100, w + 100);
        int cy = brain_rand_range(-100, h + 100);
        int r = brain_rand_range(50, 300);
        int cr = brain_rand_range(0, 255);
        int cg = brain_rand_range(0, 255);
        int cb = brain_rand_range(0, 255);
        int alpha = brain_rand_range(30, 100);

        int x0 = cx - r; if (x0 < 0) x0 = 0;
        int x1 = cx + r; if (x1 > w) x1 = w;
        int y0 = cy - r; if (y0 < 0) y0 = 0;
        int y1 = cy + r; if (y1 > h) y1 = h;

        for (int y = y0; y < y1; y++) {
            for (int x = x0; x < x1; x++) {
                int dx = x - cx, dy = y - cy;
                if (dx * dx + dy * dy <= r * r) {
                    size_t i = ((size_t)y * w + x) * 3;
                    img[i + 0] = (unsigned char)((img[i + 0] * (255 - alpha) + cr * alpha) / 255);
                    img[i + 1] = (unsigned char)((img[i + 1] * (255 - alpha) + cg * alpha) / 255);
                    img[i + 2] = (unsigned char)((img[i + 2] * (255 - alpha) + cb * alpha) / 255);
                }
            }
        }
    }
    return img;
}

static int cmd_generate(int argc, char **argv) {
    const char *type = "gradient";
    const char *output = "wallpaper.png";
    int w = 1920, h = 1080;

    for (int i = 0; i < argc; i++) {
        if (strcmp(argv[i], "--type") == 0 && i + 1 < argc) type = argv[++i];
        else if (strcmp(argv[i], "--output") == 0 && i + 1 < argc) output = argv[++i];
        else if (strcmp(argv[i], "--width") == 0 && i + 1 < argc) w = atoi(argv[++i]);
        else if (strcmp(argv[i], "--height") == 0 && i + 1 < argc) h = atoi(argv[++i]);
    }

    unsigned char *img = NULL;

    if (strcmp(type, "gradient") == 0) {
        int c1[3] = {30, 15, 60}, c2[3] = {255, 100, 50};
        img = make_gradient(w, h, c1, c2, "vertical");
    } else if (strcmp(type, "dark") == 0) {
        int c1[3] = {10, 5, 30}, c2[3] = {40, 20, 80};
        img = make_gradient(w, h, c1, c2, "diagonal");
    } else if (strcmp(type, "pattern") == 0) {
        img = make_pattern(w, h, 0);
    } else {
        fprintf(stderr, "brain: неизвестный тип: %s\n", type);
        return 1;
    }

    if (!img) {
        fprintf(stderr, "brain: не хватило памяти\n");
        return 1;
    }

    if (!stbi_write_png(output, w, h, 3, img, w * 3)) {
        fprintf(stderr, "brain: не удалось записать %s\n", output);
        free(img);
        return 1;
    }
    free(img);

    StrBuf sb; sb_init(&sb);
    sb_append(&sb, "{\"status\":\"ok\",\"path\":");
    sb_append_json_string(&sb, output);
    sb_append(&sb, "}\n");
    printf("%s", sb.data);
    free(sb.data);
    return 0;
}

/* ============================================================
 *  Анализ файлов
 * ============================================================ */

typedef struct { const char *ext; const char *category; } SortRule;

static const SortRule SORT_RULES[] = {
    {"jpg", "Images"}, {"jpeg", "Images"}, {"png", "Images"},
    {"gif", "Images"}, {"bmp", "Images"}, {"webp", "Images"},
    {"svg", "Images"}, {"tiff", "Images"},
    {"pdf", "Documents/PDF"}, {"doc", "Documents/Word"}, {"docx", "Documents/Word"},
    {"txt", "Documents/Text"}, {"odt", "Documents/Text"}, {"rtf", "Documents/Text"},
    {"xls", "Documents/Spreadsheets"}, {"xlsx", "Documents/Spreadsheets"},
    {"csv", "Documents/Spreadsheets"},
    {"mp4", "Video"}, {"avi", "Video"}, {"mkv", "Video"},
    {"mov", "Video"}, {"wmv", "Video"}, {"webm", "Video"},
    {"mp3", "Audio"}, {"wav", "Audio"}, {"flac", "Audio"},
    {"ogg", "Audio"}, {"aac", "Audio"}, {"m4a", "Audio"},
    {"zip", "Archives"}, {"tar", "Archives"}, {"gz", "Archives"},
    {"rar", "Archives"}, {"7z", "Archives"},
    {"py", "Code/Python"}, {"js", "Code/JavaScript"}, {"ts", "Code/TypeScript"},
    {"rs", "Code/Rust"}, {"go", "Code/Go"}, {"lua", "Code/Lua"},
    {"c", "Code/C"}, {"cpp", "Code/C++"}, {"h", "Code/Headers"},
    {NULL, NULL}
};

static void str_to_lower(char *s) {
    for (; *s; s++) if (*s >= 'A' && *s <= 'Z') *s += 32;
}

/* Последний сегмент после '/' (или '\\' на Windows) */
static const char *path_base(const char *path) {
    const char *base = strrchr(path, '/');
#ifdef _WIN32
    const char *bslash = strrchr(path, '\\');
    if (bslash && (!base || bslash > base)) base = bslash;
#endif
    return base ? base + 1 : path;
}

/* Возвращает расширение файла (без точки, в нижнем регистре) */
static const char *file_extension(const char *name) {
    const char *dot = strrchr(name, '.');
    if (!dot) return "";
    return dot + 1;
}

static void categorize(const char *ext, const char **category, const char **folder) {
    char extl[32];
    snprintf(extl, sizeof(extl), "%s", ext);
    str_to_lower(extl);

    for (const SortRule *r = SORT_RULES; r->ext; r++) {
        if (strcmp(r->ext, extl) == 0) {
            *category = r->category;
            *folder = path_base(r->category);
            return;
        }
    }
    *category = "Other";
    *folder = "Other";
}

static void format_size(double bytes, char *out, size_t outlen) {
    const char *units[] = {"B", "KB", "MB", "GB", "TB"};
    int u = 0;
    while (bytes >= 1024.0 && u < 4) { bytes /= 1024.0; u++; }
    snprintf(out, outlen, "%.1f %s", bytes, units[u]);
}

/* Проверяет, является ли путь обычным файлом */
static int is_file(const char *path) {
#ifdef _WIN32
    DWORD attr = GetFileAttributesA(path);
    return attr != INVALID_FILE_ATTRIBUTES && !(attr & FILE_ATTRIBUTE_DIRECTORY);
#else
    struct stat st;
    if (stat(path, &st) != 0) return 0;
    return S_ISREG(st.st_mode);
#endif
}

static long long file_size(const char *path) {
#ifdef _WIN32
    /* Win32: GetFileAttributesExA возвращает размер без открытия дескриптора */
    WIN32_FILE_ATTRIBUTE_DATA fd;
    if (!GetFileAttributesExA(path, GetFileExInfoStandard, &fd)) return 0;
    return (long long)fd.nFileSizeLow | ((long long)fd.nFileSizeHigh << 32);
#else
    struct stat st;
    if (stat(path, &st) != 0) return 0;
    return (long long)st.st_size;
#endif
}

/* Добавляет JSON-объект анализа одного файла в буфер */
static void analyze_file_json(StrBuf *sb, const char *path) {
    char abs[4096];
#ifdef _WIN32
    if (_fullpath(abs, path, sizeof(abs)) == NULL) snprintf(abs, sizeof(abs), "%s", path);
#else
    if (path[0] == '/') {
        snprintf(abs, sizeof(abs), "%s", path);
    } else {
        /* Превращаем относительный путь в абсолютный через текущую директорию */
        if (getcwd(abs, sizeof(abs)) == NULL) snprintf(abs, sizeof(abs), "%s", path);
        size_t l = strlen(abs);
        if (l > 0 && l + strlen(path) + 1 < sizeof(abs)) {
            snprintf(abs + l, sizeof(abs) - l, "/%s", path);
        }
    }
#endif

    const char *name = path_base(abs);
    const char *ext = file_extension(name);
    const char *category, *folder;
    categorize(ext, &category, &folder);

    long long size = file_size(abs);
    char human[32];
    format_size((double)size, human, sizeof(human));

    sb_append(sb, "{\"name\":");       sb_append_json_string(sb, name);
    sb_append(sb, ",\"path\":");       sb_append_json_string(sb, abs);
    sb_append(sb, ",\"extension\":");  sb_append_json_string(sb, ext);
    sb_append(sb, ",\"size_bytes\":"); sb_printf(sb, "%lld", size);
    sb_append(sb, ",\"size_human\":"); sb_append_json_string(sb, human);
    sb_append(sb, ",\"category\":");   sb_append_json_string(sb, category);
    sb_append(sb, ",\"priority\":2");
    sb_append(sb, ",\"suggested_folder\":"); sb_append_json_string(sb, folder);
    sb_append(sb, "}");
}

/* Рекурсивно обходит директорию, заполняя JSON-массив. first=1 пока элемент первый. */
static void walk_directory(StrBuf *sb, const char *dirpath, int *first) {
#ifdef _WIN32
    char pattern[4096];
    snprintf(pattern, sizeof(pattern), "%s\\*", dirpath);
    WIN32_FIND_DATAA fd;
    HANDLE h = FindFirstFileA(pattern, &fd);
    if (h == INVALID_HANDLE_VALUE) return;
    do {
        if (strcmp(fd.cFileName, ".") == 0 || strcmp(fd.cFileName, "..") == 0) continue;
        char full[4096];
        snprintf(full, sizeof(full), "%s\\%s", dirpath, fd.cFileName);
        if (fd.dwFileAttributes & FILE_ATTRIBUTE_DIRECTORY) {
            walk_directory(sb, full, first);
        } else {
            if (!*first) sb_append(sb, ",");
            *first = 0;
            analyze_file_json(sb, full);
        }
    } while (FindNextFileA(h, &fd));
    FindClose(h);
#else
    DIR *d = opendir(dirpath);
    if (!d) return;
    struct dirent *e;
    while ((e = readdir(d)) != NULL) {
        if (strcmp(e->d_name, ".") == 0 || strcmp(e->d_name, "..") == 0) continue;
        char full[4096];
        snprintf(full, sizeof(full), "%s/%s", dirpath, e->d_name);
        struct stat st;
        if (stat(full, &st) != 0) continue;
        if (S_ISDIR(st.st_mode)) {
            walk_directory(sb, full, first);
        } else if (S_ISREG(st.st_mode)) {
            if (!*first) sb_append(sb, ",");
            *first = 0;
            analyze_file_json(sb, full);
        }
    }
    closedir(d);
#endif
}

static int cmd_analyze(int argc, char **argv) {
    if (argc < 1) {
        fprintf(stderr, "brain: укажите путь для анализа\n");
        return 1;
    }
    const char *target = argv[0];
    StrBuf sb; sb_init(&sb);
    sb_append(&sb, "[");
    int first = 1;

    if (is_file(target)) {
        analyze_file_json(&sb, target);
    } else {
        walk_directory(&sb, target, &first);
    }

    sb_append(&sb, "]\n");
    printf("%s", sb.data);
    free(sb.data);
    return 0;
}

/* ============================================================
 *  Конвертация изображений
 * ============================================================ */

static int cmd_convert(int argc, char **argv) {
    if (argc < 2) {
        fprintf(stderr, "brain: укажите input и output\n");
        return 1;
    }
    const char *input = argv[0];
    const char *output = argv[1];
    const char *fmt = "PNG";

    for (int i = 2; i < argc; i++) {
        if (strcmp(argv[i], "--format") == 0 && i + 1 < argc) fmt = argv[++i];
    }

    int w, h, n;
    unsigned char *data = stbi_load(input, &w, &h, &n, 3);
    if (!data) {
        fprintf(stderr, "brain: не удалось прочитать %s: %s\n", input, stbi_failure_reason());
        return 1;
    }

    int ok = 0;
    if (strcasecmp(fmt, "PNG") == 0)
        ok = stbi_write_png(output, w, h, 3, data, w * 3);
    else if (strcasecmp(fmt, "JPEG") == 0 || strcasecmp(fmt, "JPG") == 0)
        ok = stbi_write_jpg(output, w, h, 3, data, 95);
    else if (strcasecmp(fmt, "BMP") == 0)
        ok = stbi_write_bmp(output, w, h, 3, data);
    else if (strcasecmp(fmt, "TGA") == 0)
        ok = stbi_write_tga(output, w, h, 3, data);
    else
        fprintf(stderr, "brain: неизвестный формат: %s\n", fmt);

    stbi_image_free(data);

    if (!ok) {
        fprintf(stderr, "brain: не удалось записать %s\n", output);
        return 1;
    }

    StrBuf sb; sb_init(&sb);
    sb_append(&sb, "{\"status\":\"ok\",\"path\":");
    sb_append_json_string(&sb, output);
    sb_append(&sb, "}\n");
    printf("%s", sb.data);
    free(sb.data);
    return 0;
}

/* ============================================================
 *  Точка входа
 * ============================================================ */

static void usage(void) {
    fprintf(stderr,
        "Использование:\n"
        "  brain generate [--type gradient|pattern|dark] [--width W] [--height H] [--output FILE.png]\n"
        "  brain analyze <путь_к_файлу_или_папке>\n"
        "  brain convert <input> <output> [--format PNG|JPEG|BMP|TGA]\n");
}

int main(int argc, char **argv) {
    if (argc < 2) {
        usage();
        return 1;
    }

    const char *cmd = argv[1];

    if (strcmp(cmd, "generate") == 0)
        return cmd_generate(argc - 2, argv + 2);
    if (strcmp(cmd, "analyze") == 0)
        return cmd_analyze(argc - 2, argv + 2);
    if (strcmp(cmd, "convert") == 0)
        return cmd_convert(argc - 2, argv + 2);

    fprintf(stderr, "brain: неизвестная команда: %s\n", cmd);
    usage();
    return 1;
}