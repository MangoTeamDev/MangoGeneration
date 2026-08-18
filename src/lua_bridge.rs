// lua_bridge.rs — Мост между Rust и Lua
// Встраивает Lua 5.4 через mlua для пользовательских сценариев

use log::info;
use mlua::prelude::*;
use std::fs;

/// Загружает и выполняет Lua-конфигурационный файл
pub fn load_config(path: &str) -> Result<(), String> {
    info!("Загрузка Lua-конфигурации: {}", path);

    let lua = Lua::new();

    let code = fs::read_to_string(path)
        .map_err(|e| format!("Не удалось прочитать {}: {}", path, e))?;

    lua.load(&code)
        .exec()
        .map_err(|e| format!("Ошибка выполнения Lua: {}", e))?;

    Ok(())
}

/// Получает список правил сортировки из Lua-конфигурации
pub fn get_sort_rules() -> Result<Vec<String>, String> {
    let lua = Lua::new();

    let code = fs::read_to_string("lua/config.lua")
        .map_err(|e| format!("Не удалось прочитать config.lua: {}", e))?;

    lua.load(&code)
        .exec()
        .map_err(|e| format!("Ошибка выполнения Lua: {}", e))?;

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
    let lua = Lua::new();

    let code = fs::read_to_string("lua/config.lua")
        .map_err(|e| format!("Не удалось прочитать config.lua: {}", e))?;

    lua.load(&code)
        .exec()
        .map_err(|e| format!("Ошибка выполнения Lua: {}", e))?;

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

    Ok(WallpaperSettings {
        wallpaper_type,
        color_start: None,
        color_end: None,
    })
}

/// Настройки обоев из Lua
pub struct WallpaperSettings {
    pub wallpaper_type: String,
    pub color_start: Option<Vec<u8>>,
    pub color_end: Option<Vec<u8>>,
}

/// Применяет правило сортировки к файлу через Lua
pub fn apply_sort_rule(file_info: &LuaFileInfo) -> Result<Option<String>, String> {
    let lua = Lua::new();

    let code = fs::read_to_string("lua/config.lua")
        .map_err(|e| format!("Не удалось прочитать config.lua: {}", e))?;

    lua.load(&code)
        .exec()
        .map_err(|e| format!("Ошибка выполнения Lua: {}", e))?;

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
