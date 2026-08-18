# Сборка MangoGeneration

## Зависимости

### Rust (основное приложение)
```bash
# Установите Rust через rustup
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### Go (высокоскоростной движок)
```bash
# Установите Go 1.26+
# https://go.dev/doc/install
go version  # должна быть 1.26+
```

### Python (мозг и ИИ)
```bash
# Установите Python 3.14+
python3 --version  # должна быть 3.14+

# Установите зависимости
pip install -r brain/requirements.txt
```

### Lua (конфигурация)
Lua 5.4 вендорится через mlua автоматически — ничего устанавливать не нужно.

---

## Сборка

### 1. Сборка Go-библиотеки (опционально, для максимальной скорости)

```bash
cd engine
chmod +x build.sh

# Linux
./build.sh linux amd64

# Windows (кросс-компиляция)
./build.sh windows amd64
```

Результат: `engine/libengine.so` (Linux) или `engine/engine.dll` (Windows).

### 2. Сборка Rust-приложения

```bash
# Из корня проекта
cargo build --release
```

Результат: `target/release/mangogeneration`

### 3. Запуск

```bash
# Запуск из корня проекта (нужен доступ к brain/ и lua/)
cargo run --release
```

---

## Кросс-компиляция

### Linux → Windows

```bash
# Установите целевой компонент
rustup target add x86_64-pc-windows-gnu

# Соберите
cargo build --release --target x86_64-pc-windows-gnu
```

### Windows → Linux

```bash
rustup target add x86_64-unknown-linux-gnu
cargo build --release --target x86_64-unknown-linux-gnu
```

---

## Структура проекта

```
mangogeneration/
├── Cargo.toml              # Rust workspace
├── BUILD.md                # Этот файл
├── src/
│   ├── main.rs             # Точка входа
│   ├── gui.rs              # GUI (egui)
│   ├── engine_bridge.rs    # Мост к Go и Python
│   └── lua_bridge.rs       # Мост к Lua
├── engine/
│   ├── engine.go           # Go-библиотека (C-shared)
│   ├── go.mod
│   └── build.sh            # Скрипт сборки Go
├── brain/
│   ├── brain.py            # Python: генерация, анализ, конвертация
│   └── requirements.txt
└── lua/
    └── config.lua          # Пользовательские правила
```

---

## Архитектура

| Компонент | Язык | Роль |
|-----------|------|------|
| **Интерфейс** | Rust (egui) | GUI, Drag-and-Drop, системные действия |
| **Движок** | Go (c-shared) | Параллельное копирование файлов, скачивание |
| **Мозг** | Python (Pillow) | Генерация обоев, анализ, конвертация |
| **Конфигурация** | Lua 5.4 | Правила сортировки, автоматизация |

Взаимодействие:
- Rust → Go: FFI через `libloading` или CLI-вызов
- Rust → Python: вызов `brain.py` через процесс
- Rust → Lua: встраивание через `mlua`
