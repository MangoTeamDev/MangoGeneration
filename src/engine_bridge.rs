// engine_bridge.rs — Мост между Rust и внешними компонентами (Go, Python)
// Управляет вызовами внешних процессов и библиотек

use log::{error, info};
use std::ffi::{CStr, CString};
use std::path::Path;
use std::process::Command;

use crate::gui::FileAnalysis;

/// Вызывает Python brain.py для генерации обоев
pub fn run_python_generate(wallpaper_type: &str, output: &str) -> Result<String, String> {
    info!("Запуск генерации: type={}, output={}", wallpaper_type, output);

    let python = find_python()?;
    let brain_path = find_brain_py()?;

    let result = Command::new(&python)
        .arg(&brain_path)
        .arg("generate")
        .arg("--type")
        .arg(wallpaper_type)
        .arg("--output")
        .arg(output)
        .output()
        .map_err(|e| format!("Ошибка запуска Python: {}", e))?;

    if !result.status.success() {
        let stderr = String::from_utf8_lossy(&result.stderr);
        return Err(format!("Python ошибка: {}", stderr));
    }

    let stdout = String::from_utf8_lossy(&result.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout)
        .map_err(|e| format!("Ошибка парсинга ответа: {}", e))?;

    let path = json["path"].as_str().unwrap_or(output).to_string();
    Ok(path)
}

/// Вызывает Python brain.py для анализа директории
pub fn run_python_analyze(dirpath: &str) -> Result<Vec<FileAnalysis>, String> {
    info!("Запуск анализа: {}", dirpath);

    let python = find_python()?;
    let brain_path = find_brain_py()?;

    let output = Command::new(&python)
        .arg(&brain_path)
        .arg("analyze")
        .arg(dirpath)
        .output()
        .map_err(|e| format!("Ошибка запуска Python: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Python ошибка: {}", stderr));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let items: Vec<serde_json::Value> = serde_json::from_str(&stdout)
        .map_err(|e| format!("Ошибка парсинга JSON: {}", e))?;

    let results = items
        .into_iter()
        .map(|item| FileAnalysis {
            name: item["name"].as_str().unwrap_or("unknown").to_string(),
            path: item["path"].as_str().unwrap_or("").to_string(),
            extension: item["extension"].as_str().unwrap_or("").to_string(),
            size: item["size_human"].as_str().unwrap_or("?").to_string(),
            category: item["category"].as_str().unwrap_or("Other").to_string(),
            target_folder: item["suggested_folder"].as_str().unwrap_or("Other").to_string(),
        })
        .collect();

    Ok(results)
}

/// Вызывает Python brain.py для конвертации изображения
pub fn run_python_convert(input: &str, output: &str, format: &str) -> Result<String, String> {
    info!("Конвертация: {} -> {} ({})", input, output, format);

    let python = find_python()?;
    let brain_path = find_brain_py()?;

    let out = Command::new(&python)
        .arg(&brain_path)
        .arg("convert")
        .arg(input)
        .arg(output)
        .arg("--format")
        .arg(format)
        .output()
        .map_err(|e| format!("Ошибка запуска Python: {}", e))?;

    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        return Err(format!("Python ошибка: {}", stderr));
    }

    Ok(output.to_string())
}

/// Использует Go-библиотеку для быстрого копирования файлов
pub fn batch_copy_files(
    source_dir: &str,
    analysis_results: &[FileAnalysis],
) -> Result<i32, String> {
    info!("Пакетное копирование {} файлов", analysis_results.len());

    let dest_dir = format!("{}/Sorted", source_dir);

    // Пытаемся использовать Go-библиотеку если она доступна
    let tasks = build_copy_tasks(analysis_results, &dest_dir);
    match call_go_engine(&tasks) {
        Ok(count) => return Ok(count),
        Err(e) => {
            error!("Go-библиотека недоступна: {}. Используем fallback.", e);
        }
    }

    // Fallback: копируем через стандартный Rust
    let mut success_count = 0;
    for item in analysis_results {
        let dst = format!("{}/{}/{}", dest_dir, item.target_folder, item.name);
        let dst_path = Path::new(&dst);

        if let Some(parent) = dst_path.parent() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                error!("Ошибка создания папки {}: {}", parent.display(), e);
                continue;
            }
        }

        match std::fs::copy(&item.path, &dst) {
            Ok(_) => success_count += 1,
            Err(e) => error!("Ошибка копирования {}: {}", item.path, e),
        }
    }

    Ok(success_count)
}

fn build_copy_tasks(results: &[FileAnalysis], dest_dir: &str) -> String {
    let mut tasks = String::new();
    for item in results {
        let dst = format!("{}/{}/{}", dest_dir, item.target_folder, item.name);
        tasks.push_str(&format!("{}|{}\n", item.path, dst));
    }
    tasks
}

/// Имя файла Go-библиотеки для текущей платформы
fn engine_library_name() -> &'static str {
    if cfg!(target_os = "windows") {
        "engine.dll"
    } else {
        "libengine.so"
    }
}

/// Ищет скомпилированную Go-библиотеку в известных местах
fn find_engine_library() -> Option<std::path::PathBuf> {
    let name = engine_library_name();
    let candidates = [
        format!("engine/{}", name),
        format!("../engine/{}", name),
        format!("../../engine/{}", name),
        name.to_string(),
    ];

    for path in &candidates {
        let p = Path::new(path);
        if p.exists() {
            return Some(p.to_path_buf());
        }
    }

    None
}

/// Вызывает Go-библиотеку через FFI (libloading)
/// Формат задач: "src1|dst1\nsrc2|dst2\n..."
fn call_go_engine(tasks: &str) -> Result<i32, String> {
    let lib_path = find_engine_library()
        .ok_or_else(|| format!("{} не найдена", engine_library_name()))?;

    // Загружаем динамическую библиотеку
    let lib = unsafe {
        libloading::Library::new(&lib_path)
            .map_err(|e| format!("Не удалось загрузить {}: {}", lib_path.display(), e))?
    };

    // Ищем функцию BatchCopy: int BatchCopy(char* tasks)
    let batch_copy: libloading::Symbol<unsafe extern "C" fn(*mut std::os::raw::c_char) -> std::os::raw::c_int> =
        unsafe {
            lib
                .get(b"BatchCopy")
                .map_err(|e| format!("Функция BatchCopy не найдена: {}", e))?
        };

    // Передаём задачи как C-строку
    let tasks_cstr = CString::new(tasks).map_err(|e| format!("Ошибка CString: {}", e))?;

    let count = unsafe { batch_copy(tasks_cstr.as_ptr() as *mut std::os::raw::c_char) };

    info!("Go-движок скопировал {} файлов", count);
    Ok(count)
}

/// Получает расширение файла через Go-движок (пример использования других функций)
#[allow(dead_code)]
fn go_get_file_extension(path: &str) -> Option<String> {
    let lib_path = find_engine_library()?;
    let lib = unsafe { libloading::Library::new(&lib_path).ok()? };

    unsafe {
        let func: libloading::Symbol<unsafe extern "C" fn(*mut std::os::raw::c_char) -> *mut std::os::raw::c_char> =
            lib.get(b"GetFileExtension").ok()?;

        let path_cstr = CString::new(path).ok()?;
        let ptr = func(path_cstr.as_ptr() as *mut std::os::raw::c_char);

        if ptr.is_null() {
            return None;
        }

        let ext = CStr::from_ptr(ptr).to_string_lossy().to_string();

        // Освобождаем память, выделенную Go
        let free: libloading::Symbol<unsafe extern "C" fn(*mut std::os::raw::c_char)> =
            lib.get(b"FreeString").ok()?;
        free(ptr);

        Some(ext)
    }
}

/// Получает размер файла через Go-движок
#[allow(dead_code)]
fn go_get_file_info(path: &str) -> Option<i64> {
    let lib_path = find_engine_library()?;
    let lib = unsafe { libloading::Library::new(&lib_path).ok()? };

    unsafe {
        let func: libloading::Symbol<unsafe extern "C" fn(*mut std::os::raw::c_char) -> i64> =
            lib.get(b"GetFileInfo").ok()?;

        let path_cstr = CString::new(path).ok()?;
        let size = func(path_cstr.as_ptr() as *mut std::os::raw::c_char);

        if size < 0 {
            None
        } else {
            Some(size)
        }
    }
}

/// Ищем интерпретатор Python
fn find_python() -> Result<String, String> {
    // Сначала пробуем Python из виртуального окружения проекта
    let venv_python = if cfg!(target_os = "windows") {
        ".venv/Scripts/python.exe"
    } else {
        ".venv/bin/python3"
    };

    if Path::new(venv_python).exists() {
        return Ok(venv_python.to_string());
    }

    for name in &["python3", "python"] {
        if which::which(name).is_ok() {
            return Ok(name.to_string());
        }
    }
    Err("Python не найден. Установите Python 3.14+".to_string())
}

/// Ищем скрипт brain.py
fn find_brain_py() -> Result<String, String> {
    let candidates = [
        "brain/brain.py",
        "../brain/brain.py",
        "../../brain/brain.py",
    ];

    for path in &candidates {
        if Path::new(path).exists() {
            return Ok(path.to_string());
        }
    }

    Err("brain.py не найден. Убедитесь, что он находится в папке brain/".to_string())
}