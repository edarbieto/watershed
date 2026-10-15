use eframe::egui::{self, Color32, Pos2, Sense, Stroke, vec2};
use watershed_dsp::Peaks;

/// Draws a waveform with a playhead. Returns the frame to seek to when the user
/// clicks or drags on it.
pub fn waveform(
    ui: &mut egui::Ui,
    peaks: &Peaks,
    color: Color32,
    height: f32,
    position: usize,
) -> Option<usize> {
    let (rect, response) =
        ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::click_and_drag());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 2.0, ui.visuals().extreme_bg_color);

    let total = peaks.total_frames.max(1);
    let width = rect.width().max(1.0);
    let mid = rect.center().y;
    let half = rect.height() * 0.5 - 1.0;

    let columns = width.ceil() as usize;
    for x in 0..columns {
        let start = x * total / columns;
        let end = (x + 1) * total / columns;
        let (lo, hi) = peaks.range(start, end);
        let px = rect.left() + x as f32 + 0.5;
        let top = mid - hi.min(1.0) * half;
        let bottom = mid - lo.max(-1.0) * half;
        painter.line_segment(
            [Pos2::new(px, top), Pos2::new(px, bottom.max(top + 1.0))],
            Stroke::new(1.0, color),
        );
    }

    let playhead = rect.left() + position as f32 / total as f32 * width;
    painter.line_segment(
        [
            Pos2::new(playhead, rect.top()),
            Pos2::new(playhead, rect.bottom()),
        ],
        Stroke::new(1.5, ui.visuals().strong_text_color()),
    );

    if response.clicked() || response.dragged() {
        let x = response.interact_pointer_pos()?.x;
        let fraction = ((x - rect.left()) / width).clamp(0.0, 1.0);
        return Some((fraction as f64 * total as f64) as usize);
    }
    None
}
