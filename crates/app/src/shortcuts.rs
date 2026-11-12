//! The window listing the keyboard shortcuts and mouse gestures.

use eframe::egui::{self, RichText};

/// Sections of `(keys, what they do)`. `{cmd}` is Cmd on macOS, Ctrl elsewhere.
const SECTIONS: &[(&str, &[(&str, &str)])] = &[
    (
        "Playback",
        &[
            (
                "Space",
                "Play, or stop and go back to where playback started",
            ),
            ("Enter", "Play, or pause where it is"),
            ("Home", "Back to the loop start, or the song start"),
            (
                "← / →",
                "Back or forward 5 seconds, or one grid line with snap on",
            ),
        ],
    ),
    (
        "Tracks",
        &[
            ("↑ / ↓", "Select the track above or below"),
            ("M", "Mute the selected track"),
            ("S", "Solo the selected track"),
            (
                "Delete / Backspace",
                "Delete the selected track (its file stays; Edit menu to restore)",
            ),
        ],
    ),
    (
        "Loops and markers",
        &[
            ("A / B", "Loop start or end at the playhead"),
            ("L", "Loop on or off"),
            ("Shift+M", "Add a marker at the playhead"),
            ("1 – 9", "Go to marker 1 to 9"),
        ],
    ),
    (
        "Grid and tempo",
        &[
            ("T", "Tap tempo"),
            ("C", "Click on or off"),
            (
                "Shift+C",
                "Change the tempo or time signature at the playhead",
            ),
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
            ("Click or drag the time row", "Move the playhead"),
            ("Drag the playhead line", "Move the playhead"),
            ("Click a track", "Select it"),
            (
                "Drag the loop row",
                "Set a loop (drag an end to move it, click it: on / off)",
            ),
            (
                "Double-click the markers row",
                "Add a marker (click one to go there)",
            ),
            (
                "Drag the bar 1 row",
                "Move bar 1 (double-click: back to the estimate)",
            ),
            (
                "Click the bar 1 flag",
                "Change the time signature from bar 1",
            ),
            (
                "Right-click the changes row",
                "Change the tempo or time signature there",
            ),
            (
                "Click a tempo or time signature",
                "Change or delete it (drag to move it)",
            ),
            (
                "Click or drag the bars row",
                "Go to the nearest bar or beat",
            ),
            ("Click a chord", "Go to where it starts"),
            ("Double-click a fader or pan", "Back to its default"),
        ],
    ),
    (
        "App",
        &[
            ("{cmd}+O", "Open a folder of stems"),
            ("{cmd}+Shift+O", "Separate a song"),
            ("F5", "Reload the song folder"),
            (
                "Enter in a Recent search",
                "Open the first song that matches",
            ),
            ("{cmd}+,", "Settings"),
            ("?", "This list"),
            ("Esc / {cmd}+W", "Close this window or Settings"),
            ("{cmd}+Q", "Quit"),
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

/// The tallest the list gets before it scrolls.
const MAX_HEIGHT: f32 = 520.0;

/// The window's name (see `subwindow.rs`).
pub const WINDOW: &str = "shortcuts";

pub fn window(ctx: &egui::Context, open: &mut bool) {
    let cmd = if cfg!(target_os = "macos") {
        "Cmd"
    } else {
        "Ctrl"
    };
    // The list is long: it scrolls past most of the main window's height.
    let main_height = ctx.input(|i| i.viewport().inner_rect.map(|r| r.height()));
    let max_height = main_height.map_or(MAX_HEIGHT, |h| (h * 0.75).min(MAX_HEIGHT));
    crate::subwindow::show(
        ctx,
        WINDOW,
        "Keyboard Shortcuts",
        crate::subwindow::Width::Fit,
        open,
        |ui| {
            egui::ScrollArea::vertical()
                .max_height(max_height)
                // Not limited by the window's current height, which follows the
                // contents (see `subwindow::fit`).
                .min_scrolled_height(max_height)
                .show(ui, |ui| grid(ui, cmd));
        },
    );
}

/// Every section in one grid, so the columns line up across them.
fn grid(ui: &mut egui::Ui, cmd: &str) {
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
}
