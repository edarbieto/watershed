use eframe::egui::{self, Color32, Pos2, Rect, Sense, Stroke, vec2};
use watershed_dsp::Peaks;

/// What the user did on a waveform.
pub enum WaveformAction {
    /// Clicked: seek to this frame.
    Seek(usize),
    /// Dragged: select these frames as a loop. Updates while the drag goes on.
    Select(usize, usize),
}

/// Draws a waveform with a playhead and the A-B loop region, `(start, end, enabled)`
/// in frames.
pub fn waveform(
    ui: &mut egui::Ui,
    peaks: &Peaks,
    color: Color32,
    height: f32,
    position: usize,
    loop_region: Option<(usize, usize, bool)>,
) -> Option<WaveformAction> {
    let (rect, response) =
        ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::click_and_drag());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 2.0, ui.visuals().extreme_bg_color);

    let total = peaks.total_frames.max(1);
    let width = rect.width().max(1.0);
    let mid = rect.center().y;
    let half = rect.height() * 0.5 - 1.0;
    let x_of = |frame: usize| rect.left() + frame as f32 / total as f32 * width;
    let frame_at = |x: f32| {
        let fraction = ((x - rect.left()) / width).clamp(0.0, 1.0);
        (fraction as f64 * total as f64) as usize
    };

    if let Some((start, end, enabled)) = loop_region {
        let strength = if enabled { 1.0 } else { 0.4 };
        let tint = ui.visuals().strong_text_color();
        let area = Rect::from_x_y_ranges(x_of(start)..=x_of(end), rect.y_range());
        painter.rect_filled(area, 0.0, tint.gamma_multiply(0.12 * strength));
        for x in [area.left(), area.right()] {
            painter.line_segment(
                [Pos2::new(x, rect.top()), Pos2::new(x, rect.bottom())],
                Stroke::new(1.0, tint.gamma_multiply(0.6 * strength)),
            );
        }
    }

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

    let playhead = x_of(position);
    painter.line_segment(
        [
            Pos2::new(playhead, rect.top()),
            Pos2::new(playhead, rect.bottom()),
        ],
        Stroke::new(1.5, ui.visuals().strong_text_color()),
    );

    let pointer = response.interact_pointer_pos()?;
    if response.dragged() {
        let origin = ui.input(|i| i.pointer.press_origin())?;
        let (a, b) = (frame_at(origin.x), frame_at(pointer.x));
        return Some(WaveformAction::Select(a.min(b), a.max(b)));
    }
    if response.clicked() {
        return Some(WaveformAction::Seek(frame_at(pointer.x)));
    }
    None
}
