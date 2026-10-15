// Hide the console window on Windows release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

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
        Box::new(|_cc| Ok(Box::new(WatershedApp))),
    )
}

struct WatershedApp;

impl eframe::App for WatershedApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::Frame::central_panel(ui.style()).show(ui, |ui| {
            ui.centered_and_justified(|ui| {
                ui.label(egui::RichText::new("Drop a song here").weak().size(20.0));
            });
        });
    }
}
