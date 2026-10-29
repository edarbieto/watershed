// Hide the console window on Windows release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[cfg(target_os = "macos")]
mod alloc;
mod app;
mod bpm;
mod grid;
mod key;
mod loader;
mod loops;
mod markers;
mod metronome;
mod playhead;
mod separate;
mod shortcuts;
mod stems;
mod usage;
mod view;
mod waveform;

use eframe::egui;

#[cfg(target_os = "macos")]
#[global_allocator]
static ALLOCATOR: alloc::Allocator = alloc::Allocator;

fn main() -> eframe::Result {
    env_logger::init();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Watershed")
            // The app sets the minimum size from its contents once it has drawn them.
            .with_inner_size([1320.0, 760.0]),
        // On macOS the wgpu (Metal) renderer sometimes leaves the main thread unable
        // to answer for ~300 ms when another app moves or resizes the window through
        // the accessibility API (window managers like Rectangle), so the move and
        // the resize show as two steps. OpenGL answers at once. Measured with a bare
        // eframe app too, so it is not our drawing.
        #[cfg(target_os = "macos")]
        renderer: eframe::Renderer::Glow,
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
