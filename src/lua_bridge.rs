// lua_bridge.rs — Мост между Rust и Lua
// Встраивает Lua 5.4 через mlua для пользовательских сценариев

use crate::paths;
use log::info;
use mlua::prelude::*;
use std::fs;

/// Загружает Lua-рантайм и выполняет config.lua.
/// Единая точка инициализации, чтобы не дублировать логику в каждом вызове.
/// Конфиг берётся из папки mango/ рядом с бинарником (создаётся при первом запуске).
fn new_lua_with_config() -> Result<Lua, String> {
    let lua = Lua::new();

    let config_path = paths::ensure_config()?;
    let code = fs::read_to_string(&config_path)
        .map_err(|e| format!("Не удалось прочитать {}: {}", config_path.display(), e))?;

    lua.load(&code)
        .exec()
        .map_err(|e| format!("Ошибка выполнения Lua: {}", e))?;

    Ok(lua)
}

/// Получает выбранную тему из config.lua (Config.theme)
pub fn get_theme() -> Result<String, String> {
    let lua = new_lua_with_config()?;
    let globals = lua.globals();
    let config: LuaTable = globals
        .get("Config")
        .map_err(|e| format!("Config не найден: {}", e))?;
    Ok(config
        .get::<String>("theme")
        .unwrap_or_else(|_| "system".to_string()))
}

/// Сохраняет тему в config.lua (папка mango/), перезаписывая Config.theme
pub fn save_theme(theme: &str) -> Result<(), String> {
    let config_path = paths::config_file().ok_or("Не удалось определить путь к config.lua")?;
    let content = fs::read_to_string(&config_path)
        .map_err(|e| format!("Не удалось прочитать config.lua: {}", e))?;

    let new_content = replace_theme(&content, theme);
    fs::write(&config_path, new_content)
        .map_err(|e| format!("Не удалось записать config.lua: {}", e))?;

    info!("Тема сохранена в config.lua: {}", theme);
    Ok(())
}

/// Заменяет значение `theme = "..."` в содержимом config.lua.
/// Если такой строки нет — добавляет `theme = "<theme>"` в блок Config.
fn replace_theme(content: &str, theme: &str) -> String {
    // Пытаемся найти `theme` ... `=` ... `"..."` и заменить значение
    if let Some(i) = content.find("theme") {
        let after = &content[i..];
        if let Some(eq_rel) = after.find('=') {
            let val_slice = &content[i + eq_rel + 1..];
            if let Some(q) = val_slice.find('"') {
                let q_start = i + eq_rel + 1 + q;
                let after_open = &content[q_start + 1..];
                if let Some(q_end) = after_open.find('"') {
                    let end = q_start + 1 + q_end + 1;
                    let mut out = String::new();
                    out.push_str(&content[..q_start]);
                    out.push('"');
                    out.push_str(theme);
                    out.push('"');
                    out.push_str(&content[end..]);
                    return out;
                }
            }
        }
    }

    // Не нашли — вставляем после "Config = {"
    if let Some(i) = content.find("Config = {") {
        let mut out = String::new();
        let end = i + "Config = {".len();
        out.push_str(&content[..end]);
        out.push_str(&format!("\n    theme = \"{}\",", theme));
        out.push_str(&content[end..]);
        return out;
    }

    content.to_string()
}

/// Загружает и выполняет Lua-конфигурационный файл
pub fn load_config(path: &str) -> Result<(), String> {
    info!("Загрузка Lua-конфигурации: {}", path);

    let _lua = new_lua_with_config()?;
    Ok(())
}

/// Получает список правил сортировки из Lua-конфигурации
pub fn get_sort_rules() -> Result<Vec<String>, String> {
    let lua = new_lua_with_config()?;

    let globals = lua.globals();
    let sort_rules: LuaTable = globals
        .get("SortRules")
        .map_err(|e| format!("SortRules не найден: {}", e))?;

    let mut rules = Vec::new();
    for pair in sort_rules.pairs::<u32, LuaTable>() {
        let (_, rule) = pair.map_err(|e| format!("Ошибка чтения правила: {}", e))?;
        if let Ok(name) = rule.get::<String>("name") {
            rules.push(name);
        }
    }

    Ok(rules)
}

/// Получает настройки обоев для текущего времени суток
pub fn get_wallpaper_for_now() -> Result<WallpaperSettings, String> {
    let lua = new_lua_with_config()?;

    let globals = lua.globals();

    let get_wallpaper: LuaFunction = globals
        .get("GetWallpaperForNow")
        .map_err(|e| format!("GetWallpaperForNow не найден: {}", e))?;

    let result: LuaTable = get_wallpaper
        .call(())
        .map_err(|e| format!("Ошибка вызова GetWallpaperForNow: {}", e))?;

    let wallpaper_type = result
        .get::<String>("type")
        .unwrap_or_else(|_| "dark".to_string());

    // Читаем цвета из правила, если они заданы
    let color_start = parse_color(&result, "color_start");
    let color_end = parse_color(&result, "color_end");

    Ok(WallpaperSettings {
        wallpaper_type,
        color_start,
        color_end,
    })
}

/// Читает цвет из Lua-таблицы (массив из 3 байтов R, G, B)
fn parse_color(table: &LuaTable, key: &str) -> Option<Vec<u8>> {
    if let Ok(color) = table.get::<Vec<u8>>(key) {
        if color.len() == 3 {
            return Some(color);
        }
    }
    None
}

/// Настройки обоев из Lua
pub struct WallpaperSettings {
    pub wallpaper_type: String,
    pub color_start: Option<Vec<u8>>,
    pub color_end: Option<Vec<u8>>,
}

/// Применяет правило сортировки к файлу через Lua
pub fn apply_sort_rule(file_info: &LuaFileInfo) -> Result<Option<String>, String> {
    let lua = new_lua_with_config()?;

    let globals = lua.globals();

    let file_table = lua
        .create_table()
        .map_err(|e| format!("Ошибка создания таблицы: {}", e))?;
    file_table
        .set("name", file_info.name.clone())
        .map_err(|e| format!("Ошибка: {}", e))?;
    file_table
        .set("extension", file_info.extension.clone())
        .map_err(|e| format!("Ошибка: {}", e))?;
    file_table
        .set("size_bytes", file_info.size_bytes)
        .map_err(|e| format!("Ошибка: {}", e))?;

    let apply_rules: LuaFunction = globals
        .get("ApplySortRules")
        .map_err(|e| format!("ApplySortRules не найден: {}", e))?;

    let result: LuaValue = apply_rules
        .call(file_table)
        .map_err(|e| format!("Ошибка вызова ApplySortRules: {}", e))?;

    match result {
        LuaValue::Table(tbl) => {
            let folder = tbl
                .get::<String>("folder")
                .unwrap_or_else(|_| "Other".to_string());
            Ok(Some(folder))
        }
        _ => Ok(None),
    }
}

/// Информация о файле для Lua
pub struct LuaFileInfo {
    pub name: String,
    pub extension: String,
    pub size_bytes: u64,
}