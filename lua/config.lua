-- config.lua — Конфигурация MangoGeneration
-- Пользовательские правила для автоматизации задач
-- Встраивается в Rust через mlua

-- ============================================
-- НАСТРОЙКИ ПРИЛОЖЕНИЯ
-- ============================================

Config = {
    -- Имя пользователя
    user_name = "User",
    
    -- Папки по умолчанию
    paths = {
        downloads = os.getenv("HOME") .. "/Downloads",
        pictures  = os.getenv("HOME") .. "/Pictures",
        documents = os.getenv("HOME") .. "/Documents",
    },
    
    -- Интервал автоматической смены обоев (в секундах)
    wallpaper_interval = 3600,  -- 1 час
    
    -- Генерировать обои при запуске
    generate_on_start = true,
}

-- ============================================
-- ПРАВИЛА СОРТИРОВКИ ФАЙЛОВ
-- ============================================

SortRules = {
    -- Правило: если расширение PNG и размер > 1MB → в папку Gallery
    {
        name = "Big PNG to Gallery",
        condition = function(file)
            return file.extension == "png" and file.size_bytes > 1048576
        end,
        action = function(file)
            return { folder = "Gallery/PNG", reason = "Large PNG image" }
        end,
    },
    
    -- Правило: скриншоты (начинаются с "Screenshot" или "Screen")
    {
        name = "Screenshots",
        condition = function(file)
            local name_lower = string.lower(file.name)
            return string.find(name_lower, "^screenshot") ~= nil
                or string.find(name_lower, "^screen") ~= nil
        end,
        action = function(file)
            return { folder = "Screenshots", reason = "Screenshot detected" }
        end,
    },
    
    -- Правило: PDF-документы
    {
        name = "PDFs",
        condition = function(file)
            return file.extension == "pdf"
        end,
        action = function(file)
            return { folder = "Documents/PDF", reason = "PDF document" }
        end,
    },
    
    -- Правило: видео
    {
        name = "Videos",
        condition = function(file)
            local video_exts = { "mp4", "mkv", "avi", "mov", "webm" }
            for _, ext in ipairs(video_exts) do
                if file.extension == ext then return true end
            end
            return false
        end,
        action = function(file)
            return { folder = "Videos", reason = "Video file" }
        end,
    },
    
    -- Правило: временные файлы
    {
        name = "Temp files",
        condition = function(file)
            local name_lower = string.lower(file.name)
            return string.find(name_lower, "%.tmp$") ~= nil
                or string.find(name_lower, "^~") ~= nil
                or string.find(name_lower, "%.bak$") ~= nil
        end,
        action = function(file)
            return { folder = "Trash", reason = "Temp/backup file" }
        end,
    },
}

-- ============================================
-- ПРАВИЛА АВТОМАТИЧЕСКОЙ СМЕНЫ ОБОЕВ
-- ============================================

WallpaperRules = {
    -- Утро: яркие тёплые обои
    {
        name = "Morning wallpaper",
        condition = function(hour)
            return hour >= 6 and hour < 12
        end,
        action = function()
            return {
                type = "gradient",
                color_start = { 255, 200, 100 },  -- золотистый
                color_end = { 255, 120, 50 },      -- оранжевый
                direction = "vertical",
            }
        end,
    },
    
    -- День: прохладные синие тона
    {
        name = "Daytime wallpaper",
        condition = function(hour)
            return hour >= 12 and hour < 17
        end,
        action = function()
            return {
                type = "gradient",
                color_start = { 50, 100, 200 },   -- синий
                color_end = { 100, 200, 255 },     -- голубой
                direction = "horizontal",
            }
        end,
    },
    
    -- Вечер: тёмные фиолетовые тона
    {
        name = "Evening wallpaper",
        condition = function(hour)
            return hour >= 17 and hour < 21
        end,
        action = function()
            return {
                type = "dark",
            }
        end,
    },
    
    -- Ночь: полностью тёмные обои
    {
        name = "Night wallpaper",
        condition = function(hour)
            return hour >= 21 or hour < 6
        end,
        action = function()
            return {
                type = "gradient",
                color_start = { 5, 2, 15 },
                color_end = { 15, 8, 35 },
                direction = "diagonal",
            }
        end,
    },
}

-- ============================================
-- ФУНКЦИИ-ПОМОЩНИКИ
-- ============================================

--- Получить текущий час (0-23)
function GetCurrentHour()
    return os.date("*t").hour
end

--- Применить правила сортировки к файлу
function ApplySortRules(file_info)
    for _, rule in ipairs(SortRules) do
        if rule.condition(file_info) then
            return rule.action(file_info)
        end
    end
    return nil  -- ни одно правило не сработало
end

--- Получить настройки обоев для текущего времени суток
function GetWallpaperForNow()
    local hour = GetCurrentHour()
    for _, rule in ipairs(WallpaperRules) do
        if rule.condition(hour) then
            return rule.action()
        end
    end
    -- По умолчанию — тёмные обои
    return { type = "dark" }
end

--- Вывести информацию о конфигурации
function PrintConfig()
    print("=== MangoGeneration Config ===")
    print("User: " .. Config.user_name)
    print("Downloads: " .. Config.paths.downloads)
    print("Pictures: " .. Config.paths.pictures)
    print("Wallpaper interval: " .. Config.wallpaper_interval .. "s")
    print("Sort rules: " .. #SortRules)
    print("Wallpaper rules: " .. #WallpaperRules)
end

-- Автоматически выводим конфигурацию при загрузке
PrintConfig()
