//! Remembers the window between runs: position, size, maximized or full screen, and so
//! the monitor. eframe saves and restores it, and brings it back onto a monitor that
//! is still connected. What it saves is replaced here by the last normal geometry seen:
//! a window closed while minimized would otherwise reopen tiny (Windows reports it at
//! -32000 with no size), and one closed maximized would save the maximized size as its
//! normal one, so unmaximizing it would not shrink it. Full screen is not left to
//! eframe: a window created in full screen on macOS leaves it at the size of the
//! screen, so it opens normal and goes to full screen on its first frame.

use eframe::egui::{self, Pos2, Vec2};
use serde::{Deserialize, Serialize};

/// Where eframe stores the window.
const STORAGE_KEY: &str = "window";
/// Whether the window was in full screen, stored beside it.
const FULLSCREEN_KEY: &str = "watershed_fullscreen";

/// eframe's window settings (egui-winit's `WindowSettings`), field for field: it
/// restores what is saved under [`STORAGE_KEY`] in this shape.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
struct Settings {
    /// Position of the window's content, in physical pixels.
    inner_position_pixels: Option<Pos2>,
    /// Position of the window's frame, in physical pixels.
    outer_position_pixels: Option<Pos2>,
    fullscreen: bool,
    maximized: bool,
    /// Size of the content, in points.
    inner_size_points: Option<Vec2>,
}

/// The window as last seen not minimized, maximized or in full screen, and whether
/// it was maximized or in full screen.
#[derive(Debug, Default)]
pub struct WindowMemory {
    settings: Option<Settings>,
    /// Go to full screen on the next frame, as the window was when last closed.
    enter_fullscreen: bool,
}

impl WindowMemory {
    /// What eframe saved last time: until the window is seen in a normal state, the
    /// normal geometry is that one.
    pub fn load(storage: Option<&dyn eframe::Storage>) -> Self {
        Self {
            settings: storage.and_then(|s| eframe::get_value(s, STORAGE_KEY)),
            enter_fullscreen: storage
                .and_then(|s| eframe::get_value(s, FULLSCREEN_KEY))
                .unwrap_or(false),
        }
    }

    /// Once per frame: takes the window's state, and goes back to full screen on
    /// the first frame if it was.
    pub fn update(&mut self, ctx: &egui::Context) {
        if std::mem::take(&mut self.enter_fullscreen) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(true));
        }
        ctx.input(|i| self.track(i.viewport()));
    }

    fn track(&mut self, info: &egui::ViewportInfo) {
        if info.minimized == Some(true) {
            return;
        }
        let maximized = info.maximized.unwrap_or(false);
        let fullscreen = info.fullscreen.unwrap_or(false);
        let mut settings = self.settings.unwrap_or_default();
        settings.maximized = maximized;
        settings.fullscreen = fullscreen;
        if !maximized
            && !fullscreen
            && let (Some(inner), Some(outer), Some(ppp)) = (
                info.inner_rect,
                info.outer_rect,
                info.native_pixels_per_point,
            )
            && inner.width() > 0.0
            && inner.height() > 0.0
        {
            settings.inner_position_pixels = Some(inner.min * ppp);
            settings.outer_position_pixels = Some(outer.min * ppp);
            settings.inner_size_points = Some(inner.size());
        }
        self.settings = Some(settings);
    }

    /// Replaces what eframe saved for the window; called after it.
    pub fn save(&self, storage: &mut dyn eframe::Storage) {
        if let Some(settings) = self.settings.filter(|s| s.inner_size_points.is_some()) {
            let normal = Settings {
                fullscreen: false,
                ..settings
            };
            eframe::set_value(storage, STORAGE_KEY, &normal);
            eframe::set_value(storage, FULLSCREEN_KEY, &settings.fullscreen);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn info(minimized: bool, maximized: bool, inner: egui::Rect) -> egui::ViewportInfo {
        egui::ViewportInfo {
            native_pixels_per_point: Some(2.0),
            inner_rect: Some(inner),
            outer_rect: Some(inner.expand2(egui::vec2(0.0, 14.0))),
            minimized: Some(minimized),
            maximized: Some(maximized),
            fullscreen: Some(false),
            ..Default::default()
        }
    }

    #[test]
    fn keeps_the_normal_window_through_maximized_and_minimized() {
        let normal = egui::Rect::from_min_size(egui::pos2(100.0, 50.0), egui::vec2(1320.0, 760.0));
        let mut memory = WindowMemory::default();
        memory.track(&info(false, false, normal));
        let big = egui::Rect::from_min_size(egui::pos2(0.0, 25.0), egui::vec2(1728.0, 1080.0));
        memory.track(&info(false, true, big));
        // As Windows reports a minimized window.
        let gone = egui::Rect::from_min_size(egui::pos2(-16000.0, -16000.0), egui::Vec2::ZERO);
        memory.track(&info(true, false, gone));
        let settings = memory.settings.unwrap();
        assert!(settings.maximized);
        assert_eq!(settings.inner_size_points, Some(egui::vec2(1320.0, 760.0)));
        assert_eq!(
            settings.inner_position_pixels,
            Some(egui::pos2(200.0, 100.0))
        );
        assert_eq!(
            settings.outer_position_pixels,
            Some(egui::pos2(200.0, 72.0))
        );
    }

    #[test]
    fn saves_in_the_shape_of_eframes_window_settings() {
        let settings = Settings {
            inner_position_pixels: Some(egui::pos2(200.0, 100.0)),
            outer_position_pixels: Some(egui::pos2(200.0, 72.0)),
            fullscreen: false,
            maximized: true,
            inner_size_points: Some(egui::vec2(1320.0, 760.0)),
        };
        // Through egui-winit's own type and back: a field it renamed or dropped
        // would come back as its default.
        let text = ron::to_string(&settings).unwrap();
        let theirs: egui_winit::WindowSettings = ron::from_str(&text).unwrap();
        let back: Settings = ron::from_str(&ron::to_string(&theirs).unwrap()).unwrap();
        assert_eq!(back, settings);
    }
}
