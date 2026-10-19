// Hide the console window on Windows release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod loader;
mod loops;
mod markers;
mod stems;
mod waveform;

use eframe::egui;

fn main() -> eframe::Result {
    env_logger::init();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Watershed")
            .with_inner_size([1100.0, 700.0])
            .with_min_inner_size([640.0, 400.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Watershed",
        options,
        Box::new(|cc| {
            let mut app = app::WatershedApp::new(&cc.egui_ctx);
            // A folder or files passed on the command line are opened right away.
            let paths: Vec<_> = std::env::args_os().skip(1).map(Into::into).collect();
            if !paths.is_empty() {
                app.open(&paths, &cc.egui_ctx);
            }
            Ok(Box::new(app))
        }),
    )
}
