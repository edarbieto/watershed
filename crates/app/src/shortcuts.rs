//! The window listing the keyboard shortcuts and mouse gestures.

use eframe::egui::{self, RichText};

/// Sections of `(keys, what they do)`. `{cmd}` is Cmd on macOS, Ctrl elsewhere.
const SECTIONS: &[(&str, &[(&str, &str)])] = &[
    (
        "Playback",
        &[
            ("Space", "Play or pause"),
            ("Home", "Back to the loop start, or the song start"),
            ("← / →", "Back or forward 5 seconds"),
        ],
    ),
    (
        "Loops and markers",
        &[
            ("A / B", "Loop start or end at the playhead"),
            ("L", "Loop on or off"),
            ("M", "Add a marker at the playhead"),
            ("1 – 9", "Go to marker 1 to 9"),
        ],
    ),
    (
        "Grid and tempo",
        &[
            ("T", "Tap tempo"),
            ("C", "Click on or off"),
            ("Shift+C", "Change the time signature at the playhead"),
        ],
    ),
    (
        "View",
        &[
            ("+ / −", "Zoom in or out"),
            ("{cmd} + scroll, pinch", "Zoom around the pointer"),
            ("Shift + scroll, two fingers", "Scroll sideways"),
        ],
    ),
    (
        "Mouse",
        &[
            ("Double-click the time ruler", "Add a marker"),
            (
                "Drag the bar ruler",
                "Move bar 1 (double-click: back to the estimate)",
            ),
            (
                "Right-click the bar ruler",
                "Change the time signature there",
            ),
            (
                "Click a time signature",
                "Change or delete it (drag to move it)",
            ),
            ("Double-click a fader or pan", "Back to its default"),
        ],
    ),
    (
        "App",
        &[
            ("F5", "Reload the song folder"),
            ("{cmd}+,", "Settings"),
            ("?", "This list"),
        ],
    ),
];

/// Whether the shortcut for this window was pressed: `?`, which is Shift+/ on a
/// US keyboard (some platforms report the slash, others the question mark).
pub fn pressed(ctx: &egui::Context) -> bool {
    ctx.input_mut(|i| {
        i.consume_key(egui::Modifiers::NONE, egui::Key::Questionmark)
            || i.consume_key(egui::Modifiers::SHIFT, egui::Key::Questionmark)
            || i.consume_key(egui::Modifiers::SHIFT, egui::Key::Slash)
    })
}

pub fn window(ctx: &egui::Context, open: &mut bool) {
    let cmd = if cfg!(target_os = "macos") {
        "Cmd"
    } else {
        "Ctrl"
    };
    egui::Window::new("Keyboard shortcuts")
        .open(open)
        .collapsible(false)
        .resizable(false)
        .show(ctx, |ui| {
            // One grid, so the columns line up across sections.
            egui::Grid::new("shortcuts")
                .num_columns(2)
                .spacing([16.0, 4.0])
                .show(ui, |ui| {
                    for (index, (title, rows)) in SECTIONS.iter().enumerate() {
                        ui.vertical(|ui| {
                            if index > 0 {
                                ui.add_space(8.0);
                            }
                            ui.label(RichText::new(*title).strong());
                        });
                        ui.end_row();
                        for (keys, what) in *rows {
                            let keys = RichText::new(keys.replace("{cmd}", cmd));
                            // Keys in monospace; gestures are words.
                            ui.label(if *title == "Mouse" {
                                keys
                            } else {
                                keys.monospace()
                            });
                            ui.label(*what);
                            ui.end_row();
                        }
                    }
                });
        });
}
