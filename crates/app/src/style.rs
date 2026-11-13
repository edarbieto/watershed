//! The app's look: its fonts (Geist for text, Geist Mono for numbers that change, the
//! icon font for [`crate::icons`]) and its light theme.

use std::sync::Arc;

use eframe::egui::{self, Color32, FontData, FontDefinitions, FontFamily, FontTweak, Stroke};

const GEIST: &str = "Geist";
const GEIST_MONO: &str = "Geist Mono";
const ICONS: &str = "Material Symbols";

/// Installs the fonts and the light theme. The dark theme is egui's own.
pub fn install(ctx: &egui::Context) {
    ctx.set_fonts(fonts());
    ctx.set_visuals_of(egui::Theme::Light, light());
}

fn fonts() -> FontDefinitions {
    let mut fonts = FontDefinitions::default();
    let font = |bytes: &'static [u8]| Arc::new(FontData::from_static(bytes));
    fonts.font_data.insert(
        GEIST.to_owned(),
        font(include_bytes!("../../../assets/fonts/Geist-Regular.ttf")),
    );
    fonts.font_data.insert(
        GEIST_MONO.to_owned(),
        font(include_bytes!(
            "../../../assets/fonts/GeistMono-Regular.ttf"
        )),
    );
    fonts.font_data.insert(
        ICONS.to_owned(),
        Arc::new(
            FontData::from_static(include_bytes!(
                "../../../assets/fonts/MaterialSymbolsRounded-Watershed.ttf"
            ))
            .tweak(FontTweak {
                // The icons fill their em square: a bit larger than the text's capitals
                // reads as the same size, and centered on them.
                scale: 1.25,
                y_offset_factor: 0.0,
                ..FontTweak::default()
            }),
        ),
    );
    // Ours first; egui's own fonts stay behind them for characters ours lack.
    for (family, first) in [
        (FontFamily::Proportional, GEIST),
        (FontFamily::Monospace, GEIST_MONO),
    ] {
        let list = fonts.families.entry(family).or_default();
        list.insert(0, first.to_owned());
        list.insert(1, ICONS.to_owned());
    }
    fonts
}

/// A track's color as drawn on the current ground: the stem colors are light, so on
/// the light theme they are drawn darker (waveforms and track names).
pub fn track_color(visuals: &egui::Visuals, color: Color32) -> Color32 {
    if visuals.dark_mode {
        color
    } else {
        color.lerp_to_gamma(Color32::BLACK, 0.3)
    }
}

/// "Warm stone": a warm gray ground instead of white, near-black text and widgets
/// that stand out from it. egui's light theme is white with light gray text.
fn light() -> egui::Visuals {
    let rgb = |hex: u32| Color32::from_rgb((hex >> 16) as u8, (hex >> 8) as u8, hex as u8);
    let ground = rgb(0xe7e4df);
    let line = rgb(0xc9c3ba);
    let text = rgb(0x3d3934);
    let strong = rgb(0x14120f);

    let mut v = egui::Visuals::light();
    v.panel_fill = ground;
    v.window_fill = ground;
    v.window_stroke = Stroke::new(1.0, line);
    v.extreme_bg_color = rgb(0xf6f4f0);
    v.faint_bg_color = rgb(0xdcd8d1);
    v.code_bg_color = rgb(0xd3cec6);
    v.weak_text_color = Some(rgb(0x736d65));
    v.warn_fg_color = rgb(0xb85c00);
    v.error_fg_color = rgb(0xc0262d);
    v.hyperlink_color = rgb(0x1f5fa8);

    let w = &mut v.widgets;
    w.noninteractive.bg_fill = ground;
    w.noninteractive.weak_bg_fill = ground;
    w.noninteractive.bg_stroke = Stroke::new(1.0, line);
    w.noninteractive.fg_stroke = Stroke::new(1.0, text);
    for (state, fill) in [
        (&mut w.inactive, 0xd3cec6),
        (&mut w.open, 0xd3cec6),
        (&mut w.hovered, 0xc8c2b9),
        (&mut w.active, 0xb8b1a6),
    ] {
        state.bg_fill = rgb(fill);
        state.weak_bg_fill = rgb(fill);
    }
    w.inactive.fg_stroke = Stroke::new(1.0, strong);
    w.open.fg_stroke = Stroke::new(1.0, strong);
    w.hovered.bg_stroke = Stroke::new(1.0, rgb(0x8f877c));
    w.hovered.fg_stroke = Stroke::new(1.5, Color32::BLACK);
    w.active.bg_stroke = Stroke::new(1.0, rgb(0x5f584f));

    v.selection.bg_fill = rgb(0xc4d8ee);
    v.selection.stroke = Stroke::new(1.0, rgb(0x123a62));
    v
}
