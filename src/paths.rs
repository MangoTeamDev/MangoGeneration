// paths.rs — Надёжный поиск ресурсов приложения (brain/, lua/, engine/)
// Работает как из корня проекта (cargo run), так и при запуске бинарника из любой папки.

use std::path::{Path, PathBuf};

/// Директория конфигурации: <папка_бинарника>/mango
pub fn config_dir() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let dir = exe.parent()?;
    Some(dir.join("mango"))
}

/// Путь к конфигурационному файлу: <mango>/config.lua
pub fn config_file() -> Option<PathBuf> {
    config_dir().map(|d| d.join("config.lua"))
}

/// Ищет "прошитый" config.lua в ресурсах проекта (шаблон для первого запуска)
fn bundled_config() -> Option<PathBuf> {
    find_resource(&["lua/config.lua", "../lua/config.lua", "../../lua/config.lua"])
}

/// Создаёт папку mango/ рядом с бинарником и, если config.lua отсутствует,
/// копирует в неё "прошитый" config.lua. Возвращает путь к активному config.lua.
pub fn ensure_config() -> Result<PathBuf, String> {
    let dir = config_dir().ok_or("Не удалось определить папку рядом с бинарником")?;
    std::fs::create_dir_all(&dir)
        .map_err(|e| format!("Не удалось создать {}: {}", dir.display(), e))?;

    let config = dir.join("config.lua");
    if !config.exists() {
        let bundled = bundled_config().ok_or("config.lua не найден в ресурсах")?;
        std::fs::copy(&bundled, &config)
            .map_err(|e| format!("Не удалось скопировать config.lua: {}", e))?;
        log::info!("Создан конфигурационный файл: {}", config.display());
    }

    Ok(config)
}

/// Ищет ресурс по списку относительных путей.
/// Сначала пробует относительно текущей рабочей директории, затем относительно бинарника,
/// затем относительно корня проекта (для сборок через `cargo run`).
pub fn find_resource(relative_candidates: &[&str]) -> Option<PathBuf> {
    // 1) Относительно текущей рабочей директории
    for rel in relative_candidates {
        let p = Path::new(rel);
        if p.exists() {
            return Some(p.to_path_buf());
        }
    }

    // 2) Относительно директории исполняемого файла
    if let Ok(exe) = std::env::current_exe() {
        if let Some(exe_dir) = exe.parent() {
            for rel in relative_candidates {
                let p = exe_dir.join(rel);
                if p.exists() {
                    return Some(p);
                }
            }

            // 3) Для `cargo run` бинарник лежит в <корень>/target/release/ —
            //    поднимаемся на два уровня вверх, чтобы найти ресурсы в корне проекта
            if let Some(root) = exe_dir.parent().and_then(|d| d.parent()) {
                for rel in relative_candidates {
                    let p = root.join(rel);
                    if p.exists() {
                        return Some(p);
                    }
                }
            }
        }
    }

    None
}