// main.rs — Точка входа MangoGeneration
// Запускает GUI приложение с интеграцией всех компонентов

mod gui;
mod engine_bridge;
mod lua_bridge;

use log::info;

fn main() -> eframe::Result<()> {
    // Инициализация логирования
    env_logger::init();
    info!("MangoGeneration запускается...");

    // Запуск GUI
    gui::run()
}
