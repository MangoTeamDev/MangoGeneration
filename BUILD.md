# Сборка MangoGeneration

Кроссплатформенный локальный «генератор и преобразователь всего»:
обои для рабочего стола, умная уборка файлов, конвертация медиа.

Технологии: **Rust** (GUI + оркестрация), **Go** (высокоскоростной движок, c-shared),
**C** (мозг: генерация, анализ, конвертация изображений), **Lua 5.4** (правила).

## Зависимости

| Компонент | Что нужно | Проверка |
|-----------|-----------|----------|
| Rust | 1.97+ (`rustup`) | `rustc --version` |
| Go | 1.26+ | `go version` |
| C-компилятор | GCC/Clang + make | `cc --version` |
| Lua | 5.4 (вендорится через mlua) | ничего ставить не нужно |

## Сборка

### 1. C-мозг (brain)

```bash
cd brain
make            # Linux: brain/brain

# Windows (кросс-сборка, нужен mingw-w64)
#   sudo apt install gcc-mingw-w64-x86-64
make windows    # Windows: brain/brain.exe
```

### 2. Go-движок (опционально, для максимальной скорости копирования)

```bash
cd engine

# Linux
go build -buildmode=c-shared -o libengine.so engine.go

# Windows (из Linux, кросс-компиляция)
GOOS=windows GOARCH=amd64 CGO_ENABLED=1 go build -buildmode=c-shared -o engine.dll engine.go
```

Если библиотека отсутствует, приложение автоматически переключается на
встроенный Rust-fallback для копирования файлов.

### 3. Rust-приложение

```bash
# Из корня проекта
cargo build --release
```

Результат: `target/release/mangogeneration`

### 4. Запуск

```bash
cargo run --release
```

Приложение само находит `brain/` и `engine/` — как из корня проекта, так и при
запуске собранного бинарника из любой папки.

### Конфигурация и темы

При первом запуске рядом с бинарником создаётся папка `mango/` и в неё
копируется `config.lua` (шаблон из `lua/config.lua`). Все настройки читаются
именно оттуда:

```
<папка_бинарника>/mango/config.lua
```

Пользователь может править этот файл (правила сортировки, обои по времени
суток, тема). Тема оформления (Системная / Тёмная / Светлая) выбирается во
вкладке «Настройки» и сохраняется в `Config.theme` этого файла.

## Кросс-компиляция под Windows

```bash
# Установите целевой компонент и MinGW-w64 (для линковки)
rustup target add x86_64-pc-windows-gnu
sudo apt install gcc-mingw-w64-x86-64

# C-мозг
cd brain && make windows && cd ..

# Go-библиотека
cd engine
GOOS=windows GOARCH=amd64 CGO_ENABLED=1 go build -buildmode=c-shared -o engine.dll engine.go
cd ..

# Rust-приложение
cargo build --release --target x86_64-pc-windows-gnu
```

Результат: `target/x86_64-pc-windows-gnu/release/mangogeneration.exe`
(рядом положите `engine.dll` и `brain/brain.exe`).

## Структура проекта

```
mangogeneration/
├── Cargo.toml              # Rust-проект (eframe/egui + mlua + libloading)
├── src/
│   ├── main.rs             # Точка входа
│   ├── gui.rs              # GUI (egui), Drag-and-Drop
│   ├── engine_bridge.rs    # Мост к Go (FFI) и C (процессы), установка обоев
│   ├── lua_bridge.rs       # Встраивание Lua 5.4 (mlua)
│   └── paths.rs            # Поиск ресурсов относительно бинарника/корня
├── brain/
│   ├── brain.c             # C-мозг: генерация, анализ, конвертация, QR, аватарки
│   ├── vendor/             # stb_image.h, stb_image_write.h, qrcodegen.* (MIT)
│   └── Makefile            # make / make windows
├── engine/
│   ├── engine.go           # Go: параллельное копирование (c-shared)
│   └── go.mod
└── lua/
    └── config.lua          # Пользовательские правила (сортировка, обои по времени суток)
```

## Архитектура

| Компонент | Язык | Роль |
|-----------|------|------|
| **Интерфейс** | Rust (egui) | GUI, Drag-and-Drop, установка обоев |
| **Движок** | Go (c-shared) | Параллельное копирование файлов |
| **Мозг** | C (stb + qrcodegen) | Генерация обоев, анализ, конвертация, QR, аватарки |
| **Конфигурация** | Lua 5.4 | Правила сортировки, обои по времени суток |

Взаимодействие:
- Rust → Go: FFI через `libloading` (без сокетов)
- Rust → C: вызов `brain` как локального процесса
- Rust → Lua: встраивание через `mlua`