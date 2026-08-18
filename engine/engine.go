// engine.go — Высокоскоростной движок MangoGeneration
// Компилируется в C-совместимую динамическую библиотеку (.dll/.so)
// Обеспечивает параллельное копирование файлов и конвертацию

package main

// #include <stdlib.h>
import "C"
import (
	"fmt"
	"io"
	"os"
	"path/filepath"
	"strings"
	"sync"
	"unsafe"
)

// BatchCopyRequest описывает задачу на копирование файлов
type BatchCopyRequest struct {
	Sources []string
	DestDir string
}

// CopyResult результат копирования одного файла
type CopyResult struct {
	Source string
	Dest   string
	Ok     bool
	Error  string
}

// BatchCopyResults результат пакетного копирования
type BatchCopyResults struct {
	Results []CopyResult
	Total   int
	Success int
	Failed  int
}

//export CopyFile
// CopyFile копирует один файл из source в dest.
// Возвращает 0 при успехе, -1 при ошибке.
func CopyFile(source, dest *C.char) C.int {
	srcPath := C.GoString(source)
	dstPath := C.GoString(dest)

	// Создаём директорию назначения, если её нет
	dstDir := filepath.Dir(dstPath)
	if err := os.MkdirAll(dstDir, 0755); err != nil {
		fmt.Fprintf(os.Stderr, "engine: mkdir error: %v\n", err)
		return -1
	}

	// Открываем исходный файл
	srcFile, err := os.Open(srcPath)
	if err != nil {
		fmt.Fprintf(os.Stderr, "engine: open source error: %v\n", err)
		return -1
	}
	defer srcFile.Close()

	// Создаём файл назначения
	dstFile, err := os.Create(dstPath)
	if err != nil {
		fmt.Fprintf(os.Stderr, "engine: create dest error: %v\n", err)
		return -1
	}
	defer dstFile.Close()

	// Копируем с буфером 1MB для максимальной скорости
	buf := make([]byte, 1024*1024)
	_, err = io.CopyBuffer(dstFile, srcFile, buf)
	if err != nil {
		fmt.Fprintf(os.Stderr, "engine: copy error: %v\n", err)
		return -1
	}

	return 0
}

//export BatchCopy
// BatchCopy параллельно копирует набор файлов.
// Принимает JSON-массив пар [source, dest], возвращает количество успешных копирований.
// Для простоты используем простой формат: source1|dest1\nsource2|dest2\n...
func BatchCopy(tasks *C.char) C.int {
	taskStr := C.GoString(tasks)
	lines := strings.Split(strings.TrimSpace(taskStr), "\n")

	var wg sync.WaitGroup
	results := make(chan CopyResult, len(lines))

	// Ограничиваем количество одновременных потоков
	workers := 8
	sem := make(chan struct{}, workers)

	for _, line := range lines {
		line = strings.TrimSpace(line)
		if line == "" {
			continue
		}

		parts := strings.SplitN(line, "|", 2)
		if len(parts) != 2 {
			continue
		}

		src, dst := parts[0], parts[1]

		wg.Add(1)
		go func(s, d string) {
			defer wg.Done()
			sem <- struct{}{}        // захват воркера
			defer func() { <-sem }() // освобождение

			err := copySingleFile(s, d)
			results <- CopyResult{
				Source: s,
				Dest:   d,
				Ok:     err == nil,
				Error:  fmt.Sprintf("%v", err),
			}
		}(src, dst)
	}

	// Ждём завершения и закрываем канал
	go func() {
		wg.Wait()
		close(results)
	}()

	successCount := 0
	for r := range results {
		if r.Ok {
			successCount++
		} else {
			fmt.Fprintf(os.Stderr, "engine: failed to copy %s -> %s: %s\n", r.Source, r.Dest, r.Error)
		}
	}

	return C.int(successCount)
}

// copySingleFile копирует один файл (внутренняя функция)
func copySingleFile(src, dst string) error {
	dstDir := filepath.Dir(dst)
	if err := os.MkdirAll(dstDir, 0755); err != nil {
		return err
	}

	srcFile, err := os.Open(src)
	if err != nil {
		return err
	}
	defer srcFile.Close()

	dstFile, err := os.Create(dst)
	if err != nil {
		return err
	}
	defer dstFile.Close()

	buf := make([]byte, 1024*1024)
	_, err = io.CopyBuffer(dstFile, srcFile, buf)
	return err
}

//export GetFileInfo
// GetFileInfo возвращает размер файла в байтах. -1 при ошибке.
func GetFileInfo(path *C.char) C.longlong {
	filePath := C.GoString(path)
	info, err := os.Stat(filePath)
	if err != nil {
		return -1
	}
	return C.longlong(info.Size())
}

//export GetFileExtension
// GetFileExtension возвращает расширение файла (без точки, в нижнем регистре).
func GetFileExtension(path *C.char) *C.char {
	filePath := C.GoString(path)
	ext := strings.TrimPrefix(filepath.Ext(filePath), ".")
	return C.CString(strings.ToLower(ext))
}

//export ListDirectory
// ListDirectory возвращает список файлов в директории, разделённых переносом строки.
func ListDirectory(dir *C.char) *C.char {
	dirPath := C.GoString(dir)
	entries, err := os.ReadDir(dirPath)
	if err != nil {
		return C.CString("")
	}

	var names []string
	for _, entry := range entries {
		names = append(names, entry.Name())
	}
	return C.CString(strings.Join(names, "\n"))
}

//export FreeString
// FreeString освобождает память, выделенную C.CString
func FreeString(s *C.char) {
	C.free(unsafe.Pointer(s))
}

func main() {}
