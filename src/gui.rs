// gui.rs — Графический интерфейс MangoGeneration
// Красивое окно с поддержкой Drag-and-Drop и интеграцией всех модулей

use eframe::egui;
use log::error;
use std::path::PathBuf;

use crate::lua_bridge;

/// Главное состояние приложения
pub struct MangoApp {
    active_tab: Tab,
    status: String,
    sort_folder: String,
    analysis_results: Vec<FileAnalysis>,
    wallpaper_type: WallpaperType,
    wallpaper_output: String,
    lua_loaded: bool,
    dropped_files: Vec<PathBuf>,
    progress: f32,
    is_busy: bool,
    last_wallpaper: Option<String>,
    theme: Theme,
}

/// Тема оформления интерфейса
#[derive(PartialEq, Clone, Copy)]
enum Theme {
    System,
    Dark,
    Light,
}

impl Theme {
    fn as_str(&self) -> &'static str {
        match self {
            Theme::System => "system",
            Theme::Dark => "dark",
            Theme::Light => "light",
        }
    }

    fn from_str(s: &str) -> Theme {
        match s {
            "dark" => Theme::Dark,
            "light" => Theme::Light,
            _ => Theme::System,
        }
    }
}

#[derive(PartialEq)]
enum Tab {
    Generator,
    Sorter,
    Converter,
    Settings,
}

#[derive(PartialEq, Clone)]
enum WallpaperType {
    Gradient,
    Pattern,
    Dark,
}

/// Результат анализа файла (pub(crate) для доступа из engine_bridge)
#[derive(Clone)]
pub(crate) struct FileAnalysis {
    pub(crate) name: String,
    pub(crate) path: String,
    pub(crate) extension: String,
    pub(crate) size: String,
    pub(crate) category: String,
    pub(crate) target_folder: String,
}

impl Default for MangoApp {
    fn default() -> Self {
        Self {
            active_tab: Tab::Generator,
            status: String::from("Готов к работе"),
            sort_folder: String::new(),
            analysis_results: Vec::new(),
            wallpaper_type: WallpaperType::Gradient,
            wallpaper_output: String::from("wallpaper.png"),
            lua_loaded: false,
            dropped_files: Vec::new(),
            progress: 0.0,
            is_busy: false,
            last_wallpaper: None,
            theme: Theme::System,
        }
    }
}

impl MangoApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let mut style = (*cc.egui_ctx.style()).clone();
        style.spacing.item_spacing = egui::vec2(8.0, 8.0);
        cc.egui_ctx.set_style(style);

        let lua_loaded = match lua_bridge::load_config("lua/config.lua") {
            Ok(()) => {
                log::info!("Lua-конфигурация загружена");
                true
            }
            Err(e) => {
                error!("Ошибка загрузки Lua: {}", e);
                false
            }
        };

        let mut app = Self::default();
        app.lua_loaded = lua_loaded;

        // Считываем начальную тему из config.lua (если Lua загрузился)
        if lua_loaded {
            if let Ok(theme_str) = lua_bridge::get_theme() {
                app.theme = Theme::from_str(&theme_str);
            }
        }

        app
    }

    fn handle_dropped_files(&mut self, ctx: &egui::Context) {
        let dropped_files = ctx.input(|i| i.raw.dropped_files.clone());
        if !dropped_files.is_empty() {
            for file in dropped_files {
                if let Some(path) = file.path {
                    self.dropped_files.push(path);
                }
            }
            self.status = format!("Загружено файлов: {}", self.dropped_files.len());
        }
    }

    /// Применяет выбранную тему оформления к интерфейсу
    fn apply_theme(&self, ctx: &egui::Context) {
        let visuals = match self.theme {
            Theme::Dark => egui::Visuals::dark(),
            Theme::Light => egui::Visuals::light(),
            Theme::System => match ctx.system_theme() {
                Some(egui::Theme::Dark) => egui::Visuals::dark(),
                _ => egui::Visuals::light(),
            },
        };
        ctx.set_visuals(visuals);
    }
}

impl eframe::App for MangoApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.apply_theme(ctx);
        self.handle_dropped_files(ctx);

        egui::SidePanel::left("navigation").resizable(false).show(ctx, |ui| {
            ui.heading("Mango");
            ui.separator();
            ui.label("Генератор и\nпреобразователь");
            ui.add_space(16.0);

            if ui.selectable_label(self.active_tab == Tab::Generator, "Обои").clicked() {
                self.active_tab = Tab::Generator;
            }
            if ui.selectable_label(self.active_tab == Tab::Sorter, "Уборка").clicked() {
                self.active_tab = Tab::Sorter;
            }
            if ui.selectable_label(self.active_tab == Tab::Converter, "Конвертер").clicked() {
                self.active_tab = Tab::Converter;
            }
            if ui.selectable_label(self.active_tab == Tab::Settings, "Настройки").clicked() {
                self.active_tab = Tab::Settings;
            }

            ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                ui.add_space(8.0);
                let status_color = if self.is_busy {
                    egui::Color32::from_rgb(255, 200, 0)
                } else {
                    egui::Color32::from_rgb(100, 200, 100)
                };
                ui.colored_label(status_color, &self.status);
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            match self.active_tab {
                Tab::Generator => self.render_generator_tab(ui),
                Tab::Sorter => self.render_sorter_tab(ui),
                Tab::Converter => self.render_converter_tab(ui),
                Tab::Settings => self.render_settings_tab(ui),
            }
        });
    }
}

impl MangoApp {
    fn render_generator_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading("Генератор обоев");
        ui.separator();

        ui.label("Выберите тип обоев:");
        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.wallpaper_type, WallpaperType::Gradient, "Градиент");
            ui.selectable_value(&mut self.wallpaper_type, WallpaperType::Pattern, "Паттерн");
            ui.selectable_value(&mut self.wallpaper_type, WallpaperType::Dark, "Тёмные");
        });

        ui.add_space(8.0);
        ui.label("Имя файла:");
        ui.text_edit_singleline(&mut self.wallpaper_output);

        ui.add_space(8.0);

        let btn = ui.button("Сгенерировать обои");
        if btn.clicked() && !self.is_busy {
            self.is_busy = true;
            self.status = "Генерация...".to_string();

            let wallpaper_type = match self.wallpaper_type {
                WallpaperType::Gradient => "gradient",
                WallpaperType::Pattern => "pattern",
                WallpaperType::Dark => "dark",
            };

            match crate::engine_bridge::run_brain_generate(
                wallpaper_type,
                &self.wallpaper_output,
            ) {
                Ok(path) => {
                    self.status = format!("Сохранено: {}", path);
                    self.last_wallpaper = Some(path);
                    self.progress = 1.0;
                }
                Err(e) => {
                    self.status = format!("Ошибка: {}", e);
                    error!("Ошибка генерации: {}", e);
                }
            }
            self.is_busy = false;
        }

        // Кнопка: генерация по правилам времени суток из Lua
        let lua_btn = ui.button("По времени суток (правила из Lua)");
        if lua_btn.clicked() && !self.is_busy {
            match lua_bridge::get_wallpaper_for_now() {
                Ok(settings) => {
                    self.is_busy = true;
                    self.status = "Генерация по Lua-правилам...".to_string();

                    let output = format!("wallpaper_{}.png", settings.wallpaper_type);
                    let result = crate::engine_bridge::run_brain_generate(
                        &settings.wallpaper_type,
                        &output,
                    );

                    match result {
                        Ok(path) => {
                            self.status = format!("Сохранено (Lua): {}", path);
                            self.last_wallpaper = Some(path);
                            self.progress = 1.0;
                        }
                        Err(e) => {
                            self.status = format!("Ошибка: {}", e);
                            error!("Ошибка генерации по Lua: {}", e);
                        }
                    }
                    self.is_busy = false;
                }
                Err(e) => {
                    self.status = format!("Ошибка Lua: {}", e);
                    error!("Ошибка Lua: {}", e);
                }
            }
        }

        // Кнопка: установить последний сгенерированный файл как обои
        if let Some(path) = &self.last_wallpaper {
            ui.add_space(4.0);
            let set_btn = ui.button("Установить как обои рабочего стола");
            if set_btn.clicked() && !self.is_busy {
                self.is_busy = true;
                self.status = "Установка обоев...".to_string();

                match crate::engine_bridge::set_desktop_wallpaper(path) {
                    Ok(()) => {
                        self.status = format!("Обои установлены: {}", path);
                    }
                    Err(e) => {
                        self.status = format!("Ошибка установки обоев: {}", e);
                        error!("Ошибка установки обоев: {}", e);
                    }
                }
                self.is_busy = false;
            }
        }

        ui.add_space(16.0);
        ui.separator();
        let available = ui.available_size();
        let (rect, _response) = ui.allocate_exact_size(
            egui::vec2(available.x, 120.0),
            egui::Sense::hover(),
        );

        ui.painter().rect_stroke(
            rect,
            4.0,
            egui::Stroke::new(2.0_f32, egui::Color32::from_rgb(100, 100, 200)),
            egui::StrokeKind::Inside,
        );

        ui.painter().text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            "Перетащите файлы сюда",
            egui::FontId::proportional(16.0),
            egui::Color32::GRAY,
        );

        if !self.dropped_files.is_empty() {
            ui.add_space(8.0);
            ui.label(format!("Загружено: {} файлов", self.dropped_files.len()));
            for file in &self.dropped_files {
                ui.label(file.display().to_string());
            }
        }
    }

    fn render_sorter_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading("Умная уборка файлов");
        ui.separator();

        ui.label("Папка для анализа:");
        ui.horizontal(|ui| {
            ui.text_edit_singleline(&mut self.sort_folder);
            if ui.button("Обзор...").clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .set_title("Выберите папку")
                    .pick_folder()
                {
                    self.sort_folder = path.display().to_string();
                }
            }
        });

        ui.add_space(8.0);

        if ui.button("Анализировать").clicked() && !self.sort_folder.is_empty() {
            self.is_busy = true;
            self.status = "Анализ...".to_string();

            match crate::engine_bridge::run_brain_analyze(&self.sort_folder) {
                Ok(results) => {
                    self.analysis_results = results;
                    self.status = format!("Найдено файлов: {}", self.analysis_results.len());
                }
                Err(e) => {
                    self.status = format!("Ошибка: {}", e);
                    error!("Ошибка анализа: {}", e);
                }
            }
            self.is_busy = false;
        }

        ui.add_space(8.0);

        if !self.analysis_results.is_empty() {
            ui.separator();
            ui.label("Результаты анализа:");

            egui::ScrollArea::vertical().show(ui, |ui| {
                egui::Grid::new("analysis_grid").striped(true).show(ui, |ui| {
                    ui.label(egui::RichText::new("Имя").strong());
                    ui.label(egui::RichText::new("Тип").strong());
                    ui.label(egui::RichText::new("Размер").strong());
                    ui.label(egui::RichText::new("Куда").strong());
                    ui.end_row();

                    for item in &self.analysis_results {
                        ui.label(&item.name);
                        ui.label(&item.category);
                        ui.label(&item.size);
                        ui.label(&item.target_folder);
                        ui.end_row();
                    }
                });
            });
        }

        if !self.analysis_results.is_empty() {
            ui.add_space(8.0);
            if ui.button("Скопировать файлы по папкам").clicked() && !self.is_busy {
                self.is_busy = true;
                self.status = "Копирование...".to_string();

                match crate::engine_bridge::batch_copy_files(
                    &self.sort_folder,
                    &self.analysis_results,
                ) {
                    Ok(n) => {
                        self.status = format!("Скопировано файлов: {}", n);
                    }
                    Err(e) => {
                        self.status = format!("Ошибка: {}", e);
                    }
                }
                self.is_busy = false;
            }
        }
    }

    fn render_converter_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading("Конвертер форматов");
        ui.separator();

        ui.label("Перетащите файлы для конвертации или используйте кнопку ниже:");
        if ui.button("Выбрать файлы").clicked() {
            if let Some(files) = rfd::FileDialog::new()
                .set_title("Выберите изображения")
                .add_filter("Изображения", &["png", "jpg", "jpeg", "webp", "bmp", "gif"])
                .pick_files()
            {
                for file in files {
                    self.dropped_files.push(file);
                }
            }
        }

        ui.add_space(8.0);
        ui.label("Формат вывода:");
        let mut format = String::from("PNG");
        ui.horizontal(|ui| {
            ui.selectable_value(&mut format, String::from("PNG"), "PNG");
            ui.selectable_value(&mut format, String::from("JPEG"), "JPEG");
            ui.selectable_value(&mut format, String::from("BMP"), "BMP");
            ui.selectable_value(&mut format, String::from("TGA"), "TGA");
        });

        if !self.dropped_files.is_empty() {
            ui.add_space(8.0);
            ui.label(format!("Файлов к конвертации: {}", self.dropped_files.len()));

            if ui.button("Конвертировать").clicked() && !self.is_busy {
                self.is_busy = true;
                self.status = "Конвертация...".to_string();
                let mut converted = 0;

                for file in &self.dropped_files {
                    let stem = file.file_stem().unwrap_or_default().to_string_lossy();
                    let output = file.parent().unwrap_or(&PathBuf::from("."))
                        .join(format!("{}.{}", stem, format.to_lowercase()));

                    match crate::engine_bridge::run_brain_convert(
                        &file.display().to_string(),
                        &output.display().to_string(),
                        &format,
                    ) {
                        Ok(_) => converted += 1,
                        Err(e) => error!("Ошибка конвертации {}: {}", file.display(), e),
                    }
                }

                self.status = format!("Конвертировано: {}", converted);
                self.is_busy = false;
            }
        }
    }

    fn render_settings_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading("Настройки");
        ui.separator();

        ui.label("Тема оформления:");
        let mut theme_changed = false;
        ui.horizontal(|ui| {
            if ui
                .selectable_value(&mut self.theme, Theme::System, "Системная")
                .changed()
            {
                theme_changed = true;
            }
            if ui
                .selectable_value(&mut self.theme, Theme::Dark, "Тёмная")
                .changed()
            {
                theme_changed = true;
            }
            if ui
                .selectable_value(&mut self.theme, Theme::Light, "Светлая")
                .changed()
            {
                theme_changed = true;
            }
        });
        if theme_changed {
            if let Err(e) = lua_bridge::save_theme(self.theme.as_str()) {
                error!("Ошибка сохранения темы: {}", e);
            }
        }

        ui.add_space(12.0);
        ui.label("Lua-конфигурация:");
        if self.lua_loaded {
            ui.colored_label(egui::Color32::from_rgb(100, 200, 100), "Загружена");
        } else {
            ui.colored_label(egui::Color32::from_rgb(200, 100, 100), "Не загружена");
        }
        if let Some(cfg) = crate::paths::config_file() {
            ui.label(format!("Файл: {}", cfg.display()));
        }

        ui.add_space(8.0);
        ui.label("Правила сортировки (из Lua):");
        if self.lua_loaded {
            match lua_bridge::get_sort_rules() {
                Ok(rules) => {
                    for rule in &rules {
                        ui.label(format!("> {}", rule));
                    }
                }
                Err(e) => {
                    ui.colored_label(egui::Color32::RED, format!("Ошибка: {}", e));
                }
            }
        }

        ui.add_space(16.0);
        ui.separator();
        ui.label("MangoGeneration v0.1.0");
        ui.label("Rust + Go + C + Lua");
        ui.label("FOSS | Локально | Безопасно");
    }
}

pub fn run() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([800.0, 600.0])
            .with_min_inner_size([600.0, 400.0])
            .with_title("MangoGeneration"),
        ..Default::default()
    };

    eframe::run_native(
        "MangoGeneration",
        options,
        Box::new(|cc| Ok(Box::new(MangoApp::new(cc)))),
    )
}
