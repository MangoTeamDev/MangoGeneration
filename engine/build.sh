#!/bin/bash
# Скрипт сборки Go-библиотеки MangoGeneration
# Компилирует engine.go в динамическую C-совместимую библиотеку

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

echo "=== Сборка MangoGeneration Engine ==="

# Определяем целевую платформу
OS="${1:-$(uname -s | tr '[:upper:]' '[:lower:]')}"
ARCH="${2:-amd64}"

case "$OS" in
    linux|Linux)
        OUTPUT="libengine.so"
        echo "Цель: Linux ($ARCH)"
        ;;
    windows|mingw*|Windows)
        OUTPUT="engine.dll"
        echo "Цель: Windows ($ARCH)"
        export GOOS=windows
        ;;
    *)
        echo "Неизвестная платформа: $OS"
        exit 1
        ;;
esac

# Сборка
echo "Компиляция: go build -buildmode=c-shared -o $OUTPUT engine.go"
go build -buildmode=c-shared -o "$OUTPUT" engine.go

echo "=== Готово: $OUTPUT ==="
ls -lh "$OUTPUT"
