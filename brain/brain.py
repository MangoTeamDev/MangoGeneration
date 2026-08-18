#!/usr/bin/env python3
"""
brain.py — Мозг MangoGeneration (Python 3.14+)
Генерирует изображения, анализирует файлы для сортировки, конвертирует форматы.
Работает как локальный скрипт, вызываемый из Rust через процесс или pyo3.
"""

import sys
import os
import json
import math
import time
from pathlib import Path
from typing import Optional

# Пытаемся импортировать Pillow (нужен для генерации изображений)
try:
    from PIL import Image, ImageDraw, ImageFilter
    HAS_PILLOW = True
except ImportError:
    HAS_PILLOW = False
    print("brain: Pillow не установлен. Генерация изображений недоступна.", file=sys.stderr)
    print("brain: Установите: pip install Pillow", file=sys.stderr)


# === МОДУЛЬ ГЕНЕРАЦИИ ИЗОБРАЖЕНИЙ ===

def generate_gradient(
    width: int = 1920,
    height: int = 1080,
    color_start: tuple = (30, 15, 60),
    color_end: tuple = (255, 100, 50),
    direction: str = "vertical",
    output_path: str = "wallpaper.png"
) -> str:
    """
    Генерирует градиентное изображение.
    
    Args:
        width: Ширина изображения
        height: Высота изображения
        color_start: Начальный цвет (R, G, B)
        color_end: Конечный цвет (R, G, B)
        direction: Направление градиента ("vertical", "horizontal", "diagonal")
        output_path: Путь для сохранения
    
    Returns:
        Путь к сохранённому файлу
    """
    if not HAS_PILLOW:
        raise RuntimeError("Pillow не установлен")
    
    img = Image.new("RGB", (width, height))
    pixels = img.load()
    
    for y in range(height):
        for x in range(width):
            # Вычисляем прогресс для разных направлений
            if direction == "vertical":
                t = y / height
            elif direction == "horizontal":
                t = x / width
            elif direction == "diagonal":
                t = (x / width + y / height) / 2
            else:
                t = y / height
            
            # Линейная интерполяция цветов
            r = int(color_start[0] + (color_end[0] - color_start[0]) * t)
            g = int(color_start[1] + (color_end[1] - color_start[1]) * t)
            b = int(color_start[2] + (color_end[2] - color_start[2]) * t)
            
            pixels[x, y] = (r, g, b)
    
    img.save(output_path)
    return output_path


def generate_abstract_pattern(
    width: int = 1920,
    height: int = 1080,
    seed: Optional[int] = None,
    output_path: str = "pattern.png"
) -> str:
    """
    Генерирует абстрактный паттерн из полупрозрачных кругов.
    
    Args:
        width: Ширина
        height: Высота
        seed: Зерно генератора (для воспроизводимости)
        output_path: Путь сохранения
    
    Returns:
        Путь к файлу
    """
    if not HAS_PILLOW:
        raise RuntimeError("Pillow не установлен")
    
    import random
    if seed is not None:
        random.seed(seed)
    else:
        random.seed(int(time.time()))
    
    img = Image.new("RGB", (width, height), (10, 10, 20))
    overlay = Image.new("RGBA", (width, height), (0, 0, 0, 0))
    draw = ImageDraw.Draw(overlay)
    
    # Рисуем случайные полупрозрачные круги
    num_circles = random.randint(20, 60)
    for _ in range(num_circles):
        x = random.randint(-100, width + 100)
        y = random.randint(-100, height + 100)
        radius = random.randint(50, 300)
        r = random.randint(0, 255)
        g = random.randint(0, 255)
        b = random.randint(0, 255)
        alpha = random.randint(30, 100)
        
        draw.ellipse(
            [x - radius, y - radius, x + radius, y + radius],
            fill=(r, g, b, alpha)
        )
    
    # Накладываем и размываем
    img = img.convert("RGBA")
    img = Image.alpha_composite(img, overlay)
    img = img.filter(ImageFilter.GaussianBlur(radius=15))
    img = img.convert("RGB")
    
    img.save(output_path)
    return output_path


def generate_dark_wallpaper(
    width: int = 1920,
    height: int = 1080,
    output_path: str = "dark_wallpaper.png"
) -> str:
    """Генерирует тёмные обои для вечернего режима."""
    return generate_gradient(
        width=width,
        height=height,
        color_start=(10, 5, 30),
        color_end=(40, 20, 80),
        direction="diagonal",
        output_path=output_path
    )


# === МОДУЛЬ АНАЛИЗА И СОРТИРОВКИ ФАЙЛОВ ===

# Правила сортировки по расширению
SORT_RULES = {
    # Изображения
    "jpg": "Images", "jpeg": "Images", "png": "Images",
    "gif": "Images", "bmp": "Images", "webp": "Images",
    "svg": "Images", "tiff": "Images",
    
    # Документы
    "pdf": "Documents/PDF", "doc": "Documents/Word",
    "docx": "Documents/Word", "txt": "Documents/Text",
    "odt": "Documents/Text", "rtf": "Documents/Text",
    
    # Таблицы
    "xls": "Documents/Spreadsheets", "xlsx": "Documents/Spreadsheets",
    "csv": "Documents/Spreadsheets",
    
    # Видео
    "mp4": "Video", "avi": "Video", "mkv": "Video",
    "mov": "Video", "wmv": "Video", "webm": "Video",
    
    # Аудио
    "mp3": "Audio", "wav": "Audio", "flac": "Audio",
    "ogg": "Audio", "aac": "Audio", "m4a": "Audio",
    
    # Архивы
    "zip": "Archives", "tar": "Archives", "gz": "Archives",
    "rar": "Archives", "7z": "Archives",
    
    # Код
    "py": "Code/Python", "js": "Code/JavaScript",
    "ts": "Code/TypeScript", "rs": "Code/Rust",
    "go": "Code/Go", "lua": "Code/Lua",
    "c": "Code/C", "cpp": "Code/C++", "h": "Code/Headers",
}


def analyze_file(filepath: str) -> dict:
    """
    Анализирует файл и предлагает категорию для сортировки.
    
    Args:
        filepath: Путь к файлу
    
    Returns:
        Словарь с информацией о файле
    """
    path = Path(filepath)
    ext = path.suffix.lstrip(".").lower()
    size = path.stat().st_size if path.exists() else 0
    
    # Определяем категорию
    category = SORT_RULES.get(ext, "Other")
    
    # Определяем приоритет (чем меньше расширение — тем выше)
    priority_map = {
        "pdf": 1, "doc": 1, "docx": 1,
        "jpg": 2, "jpeg": 2, "png": 2,
        "mp4": 3, "avi": 3, "mkv": 3,
        "mp3": 4, "wav": 4, "flac": 4,
    }
    
    return {
        "path": str(path.absolute()),
        "name": path.name,
        "extension": ext,
        "size_bytes": size,
        "size_human": _format_size(size),
        "category": category,
        "priority": priority_map.get(ext, 99),
        "suggested_folder": category.split("/")[-1],
    }


def _format_size(size_bytes: int) -> str:
    """Форматирует размер в читаемый вид."""
    for unit in ["B", "KB", "MB", "GB", "TB"]:
        if size_bytes < 1024:
            return f"{size_bytes:.1f} {unit}"
        size_bytes /= 1024
    return f"{size_bytes:.1f} PB"


def analyze_directory(dirpath: str, recursive: bool = True) -> list:
    """
    Анализирует все файлы в директории.
    
    Args:
        dirpath: Путь к директории
        recursive: Рекурсивно ли обходить поддиректории
    
    Returns:
        Список результатов анализа
    """
    results = []
    path = Path(dirpath)
    
    if not path.exists():
        return results
    
    pattern = "**/*" if recursive else "*"
    
    for item in path.glob(pattern):
        if item.is_file():
            try:
                info = analyze_file(str(item))
                results.append(info)
            except (PermissionError, OSError) as e:
                print(f"brain: Ошибка анализа {item}: {e}", file=sys.stderr)
    
    return results


# === МОДУЛЬ КОНВЕРТАЦИИ ФОРМАТОВ ===

def convert_image(
    input_path: str,
    output_path: str,
    output_format: str = "PNG",
    quality: int = 95
) -> str:
    """
    Конвертирует изображение в другой формат.
    
    Args:
        input_path: Путь к исходному файлу
        output_path: Путь к результату
        output_format: Целевой формат (PNG, JPEG, WEBP, BMP)
        quality: Качество (для JPEG/WEBP)
    
    Returns:
        Путь к конвертированному файлу
    """
    if not HAS_PILLOW:
        raise RuntimeError("Pillow не установлен")
    
    img = Image.open(input_path)
    
    # Конвертируем в RGB если нужно сохранить в JPEG
    if output_format.upper() in ("JPEG", "JPG") and img.mode in ("RGBA", "P"):
        img = img.convert("RGB")
    
    # Сохраняем
    save_kwargs = {}
    if output_format.upper() in ("JPEG", "JPG", "WEBP"):
        save_kwargs["quality"] = quality
    
    img.save(output_path, format=output_format.upper(), **save_kwargs)
    return output_path


# === ТОЧКА ВХОДА ДЛЯ CLI ===

def main():
    """CLI-интерфейс brain.py для вызова из Rust."""
    if len(sys.argv) < 2:
        print("Использование:")
        print("  python brain.py generate [--type gradient|pattern|dark] [--output file.png]")
        print("  python brain.py analyze <путь_к_файлу_или_папке>")
        print("  python brain.py convert <input> <output> [--format PNG|JPEG|WEBP]")
        sys.exit(1)
    
    command = sys.argv[1]
    
    if command == "generate":
        wallpaper_type = "gradient"
        output = "wallpaper.png"
        
        i = 2
        while i < len(sys.argv):
            if sys.argv[i] == "--type" and i + 1 < len(sys.argv):
                wallpaper_type = sys.argv[i + 1]
                i += 2
            elif sys.argv[i] == "--output" and i + 1 < len(sys.argv):
                output = sys.argv[i + 1]
                i += 2
            else:
                i += 1
        
        if wallpaper_type == "gradient":
            result = generate_gradient(output_path=output)
        elif wallpaper_type == "pattern":
            result = generate_abstract_pattern(output_path=output)
        elif wallpaper_type == "dark":
            result = generate_dark_wallpaper(output_path=output)
        else:
            print(f"Неизвестный тип: {wallpaper_type}", file=sys.stderr)
            sys.exit(1)
        
        print(json.dumps({"status": "ok", "path": result}))
    
    elif command == "analyze":
        if len(sys.argv) < 3:
            print("Укажите путь для анализа", file=sys.stderr)
            sys.exit(1)
        
        target = sys.argv[2]
        if os.path.isdir(target):
            results = analyze_directory(target)
        else:
            results = [analyze_file(target)]
        
        print(json.dumps(results, ensure_ascii=False, indent=2))
    
    elif command == "convert":
        if len(sys.argv) < 4:
            print("Укажите input и output", file=sys.stderr)
            sys.exit(1)
        
        input_path = sys.argv[2]
        output_path = sys.argv[3]
        fmt = "PNG"
        
        if "--format" in sys.argv:
            idx = sys.argv.index("--format")
            if idx + 1 < len(sys.argv):
                fmt = sys.argv[idx + 1]
        
        result = convert_image(input_path, output_path, fmt)
        print(json.dumps({"status": "ok", "path": result}))
    
    else:
        print(f"Неизвестная команда: {command}", file=sys.stderr)
        sys.exit(1)


if __name__ == "__main__":
    main()
