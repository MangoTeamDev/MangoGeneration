// engine_bridge.rs — Мост между Rust и внешними компонентами (C-мозг, Go-движок)
// Вызывает собранный brain (C) как локальный процесс и Go-библиотеку через FFI.

use crate::paths;
use log::{error, info};
use std::ffi::{CStr, CString};
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::gui::FileAnalysis;

/// Вызывает C-мозг (brain) для генерации обоев
pub fn run_brain_generate(wallpaper_type: &str, output: &str) -> Result<String, String> {
    info!("Запуск генерации: type={}, output={}", wallpaper_type, output);

    let brain = find_brain_bin()?;

    let result = Command::new(&brain)
        .arg("generate")
        .arg("--type")
        .arg(wallpaper_type)
        .arg("--output")
        .arg(output)
        .output()
        .map_err(|e| format!("Ошибка запуска brain: {}", e))?;

    if !result.status.success() {
        let stderr = String::from_utf8_lossy(&result.stderr);
        return Err(format!("brain ошибка: {}", stderr));
    }

    let stdout = String::from_utf8_lossy(&result.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout)
        .map_err(|e| format!("Ошибка парсинга ответа: {}", e))?;

    let path = json["path"].as_str().unwrap_or(output).to_string();
    Ok(path)
}

/// Вызывает C-мозг (brain) для анализа директории
pub fn run_brain_analyze(dirpath: &str) -> Result<Vec<FileAnalysis>, String> {
    info!("Запуск анализа: {}", dirpath);

    let brain = find_brain_bin()?;

    let output = Command::new(&brain)
        .arg("analyze")
        .arg(dirpath)
        .output()
        .map_err(|e| format!("Ошибка запуска brain: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("brain ошибка: {}", stderr));
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
            size_bytes: item["size_bytes"].as_u64().unwrap_or(0),
            category: item["category"].as_str().unwrap_or("Other").to_string(),
            target_folder: item["suggested_folder"].as_str().unwrap_or("Other").to_string(),
        })
        .collect();

    Ok(results)
}

/// Вызывает C-мозг (brain) для генерации QR-кода
pub fn run_brain_qrcode(text: &str, output: &str, size: i32) -> Result<String, String> {
    info!("Генерация QR-кода: text_len={}, output={}", text.len(), output);

    let brain = find_brain_bin()?;

    let result = Command::new(&brain)
        .arg("qrcode")
        .arg("--text")
        .arg(text)
        .arg("--output")
        .arg(output)
        .arg("--size")
        .arg(size.to_string())
        .output()
        .map_err(|e| format!("Ошибка запуска brain: {}", e))?;

    if !result.status.success() {
        let stderr = String::from_utf8_lossy(&result.stderr);
        return Err(format!("brain ошибка: {}", stderr));
    }

    let stdout = String::from_utf8_lossy(&result.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout)
        .map_err(|e| format!("Ошибка парсинга ответа: {}", e))?;

    Ok(json["path"].as_str().unwrap_or(output).to_string())
}

/// Вызывает C-мозг (brain) для генерации аватарки из имени
pub fn run_brain_avatar(name: &str, output: &str, size: i32) -> Result<String, String> {
    info!("Генерация аватарки: name={}, output={}", name, output);

    let brain = find_brain_bin()?;

    let result = Command::new(&brain)
        .arg("avatar")
        .arg("--name")
        .arg(name)
        .arg("--output")
        .arg(output)
        .arg("--size")
        .arg(size.to_string())
        .output()
        .map_err(|e| format!("Ошибка запуска brain: {}", e))?;

    if !result.status.success() {
        let stderr = String::from_utf8_lossy(&result.stderr);
        return Err(format!("brain ошибка: {}", stderr));
    }

    let stdout = String::from_utf8_lossy(&result.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout)
        .map_err(|e| format!("Ошибка парсинга ответа: {}", e))?;

    Ok(json["path"].as_str().unwrap_or(output).to_string())
}

/// Вызывает C-мозг (brain) для конвертации изображения
pub fn run_brain_convert(input: &str, output: &str, format: &str) -> Result<String, String> {
    info!("Конвертация: {} -> {} ({})", input, output, format);

    let brain = find_brain_bin()?;

    let out = Command::new(&brain)
        .arg("convert")
        .arg(input)
        .arg(output)
        .arg("--format")
        .arg(format)
        .output()
        .map_err(|e| format!("Ошибка запуска brain: {}", e))?;

    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        return Err(format!("brain ошибка: {}", stderr));
    }

    Ok(output.to_string())
}

/// Определяет целевую папку файла: сначала правила Lua, затем категория из анализа.
/// Так распределяются ВСЕ файлы, включая файлы без расширения.
pub(crate) fn target_folder_for(item: &FileAnalysis) -> String {
    let lua_info = crate::lua_bridge::LuaFileInfo {
        name: item.name.clone(),
        extension: item.extension.clone(),
        size_bytes: item.size_bytes,
    };

    match crate::lua_bridge::apply_sort_rule(&lua_info) {
        Ok(Some(folder)) => folder,
        _ => item.target_folder.clone(),
    }
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
        let folder = target_folder_for(item);
        let dst = format!("{}/{}/{}", dest_dir, folder, item.name);
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
        let folder = target_folder_for(item);
        let dst = format!("{}/{}/{}", dest_dir, folder, item.name);
        tasks.push_str(&format!("{}|{}\n", item.path, dst));
    }
    tasks
}

/// Имя исполняемого файла C-мозга для текущей платформы
fn brain_bin_name() -> &'static str {
    if cfg!(target_os = "windows") {
        "brain.exe"
    } else {
        "brain"
    }
}

/// Ищет собранный C-мозг (brain) в известных местах
fn find_brain_bin() -> Result<String, String> {
    let name = brain_bin_name();
    let candidates = [
        format!("brain/{}", name),
        format!("../brain/{}", name),
        format!("../../brain/{}", name),
        name.to_string(),
    ];

    let rel: Vec<&str> = candidates.iter().map(|s| s.as_str()).collect();
    if let Some(p) = paths::find_resource(&rel) {
        return Ok(p.display().to_string());
    }

    Err("brain (C) не найден. Соберите его: make -C brain".to_string())
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

    let rel: Vec<&str> = candidates.iter().map(|s| s.as_str()).collect();
    paths::find_resource(&rel)
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

/// Устанавливает изображение в качестве обоев рабочего стола
pub fn set_desktop_wallpaper(path: &str) -> Result<(), String> {
    info!("Установка обоев: {}", path);

    #[cfg(target_os = "linux")]
    {
        let abs = std::path::absolute(Path::new(path))
            .unwrap_or_else(|_| PathBuf::from(path));
        let uri = format!("file://{}", abs.display());

        // GNOME: gsettings
        if which::which("gsettings").is_ok() {
            let ok = Command::new("gsettings")
                .args([
                    "set",
                    "org.gnome.desktop.background",
                    "picture-uri",
                    &uri,
                ])
                .status()
                .map(|s| s.success())
                .unwrap_or(false);
            if ok {
                return Ok(());
            }
        }

        // XFCE: xfconf-query
        if which::which("xfconf-query").is_ok() {
            let ok = Command::new("xfconf-query")
                .args([
                    "-c", "xfce4-desktop",
                    "-p", "/backdrop/screen0/monitor0/workspace0/last-image",
                    "-s", abs.to_str().unwrap_or(path),
                ])
                .status()
                .map(|s| s.success())
                .unwrap_or(false);
            if ok {
                return Ok(());
            }
        }

        // Универсальные инструменты
        for tool in ["feh", "nitrogen", "hsetroot"] {
            if which::which(tool).is_ok() {
                let ok = Command::new(tool)
                    .arg("--bg-fill")
                    .arg(path)
                    .status()
                    .map(|s| s.success())
                    .unwrap_or(false);
                if ok {
                    return Ok(());
                }
            }
        }

        return Err(
            "Не удалось установить обои: не найден поддерживаемый инструмент (gsettings, feh, nitrogen, hsetroot)"
                .to_string(),
        );
    }

    #[cfg(target_os = "windows")]
    {
        set_windows_wallpaper(path)
    }

    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    {
        Err("Установка обоев поддерживается только на Linux и Windows".to_string())
    }
}

/// Устанавливает обои на Windows через SystemParametersInfoW (user32.dll)
#[cfg(target_os = "windows")]
fn set_windows_wallpaper(path: &str) -> Result<(), String> {
    use std::os::windows::ffi::OsStrExt;
    use std::os::raw::c_void;

    // WinAPI константы
    const SPI_SETDESKWALLPAPER: u32 = 0x0014;
    const SPIF_UPDATEINIFILE: u32 = 0x0001;

    // Кодируем путь в UTF-16 для SystemParametersInfoW
    let wide: Vec<u16> = std::ffi::OsStr::new(path)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    let user32 = unsafe {
        libloading::Library::new("user32.dll")
            .map_err(|e| format!("Не удалось загрузить user32.dll: {}", e))?
    };

    let spi: libloading::Symbol<unsafe extern "system" fn(u32, u32, *mut c_void, u32) -> i32> =
        unsafe {
            user32
                .get(b"SystemParametersInfoW")
                .map_err(|e| format!("SystemParametersInfoW не найдена: {}", e))?
        };

    let ok = unsafe {
        spi(
            SPI_SETDESKWALLPAPER,
            0,
            wide.as_ptr() as *mut c_void,
            SPIF_UPDATEINIFILE,
        )
    };

    if ok != 0 {
        Ok(())
    } else {
        Err("SystemParametersInfoW вернул ошибку".to_string())
    }
}