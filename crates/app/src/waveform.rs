use std::ops::Range;

use eframe::egui::{self, Color32, Rangef, Rect, Sense, Stroke, vec2};
use watershed_dsp::Peaks;

use crate::view::View;

/// What the user did on a waveform.
pub enum WaveformAction {
    /// Clicked: seek to this frame.
    Seek(usize),
    /// Dragged: select these frames as a loop. Updates while the drag goes on.
    Select(usize, usize),
    /// Dragged while the track is movable: its new offset in frames.
    Move(i64),
}

/// A tag on a grid row: bar 1's time signature, or a tempo or time signature
/// change later on.
pub struct SignatureTag {
    pub frame: usize,
    /// `"7/8"`, `"63 BPM"`.
    pub label: String,
    /// Index of the change; `None` for bar 1's.
    pub change: Option<usize>,
    /// False for a change before bar 1, which changes nothing.
    pub in_use: bool,
}

/// A line of the beat grid: its frame, whether it starts a bar, and its label.
pub struct GridLine {
    pub frame: usize,
    pub bar: bool,
    pub label: Option<String>,
}

/// What is drawn over the waveforms and the ruler. Positions are in frames.
pub struct Overlay<'a> {
    /// The frames shown across the width (zoom and scroll).
    pub visible: Range<usize>,
    pub position: usize,
    /// `(start, end, enabled)` of the A-B loop.
    pub loop_region: Option<(usize, usize, bool)>,
    /// Marker positions and labels, in time order.
    pub markers: &'a [(usize, String)],
    /// The beat grid, empty when it is off.
    pub grid: &'a [GridLine],
}

/// Maps the visible frames to x coordinates across `x_range` and back. Frames
/// outside the view map outside the range, where painting is clipped.
#[derive(Clone, Copy)]
pub struct Timeline {
    pub x_range: Rangef,
    pub start: usize,
    pub len: usize,
}

impl Timeline {
    pub fn new(x_range: Rangef, visible: &Range<usize>) -> Self {
        Self {
            x_range,
            start: visible.start,
            len: visible.len().max(1),
        }
    }

    pub fn x_of(self, frame: usize) -> f32 {
        let fraction = (frame as f64 - self.start as f64) / self.len as f64;
        self.x_range.min + (fraction * f64::from(self.x_range.span().max(1.0))) as f32
    }

    pub fn frame_at(self, x: f32) -> usize {
        let fraction = ((x - self.x_range.min) / self.x_range.span().max(1.0)).clamp(0.0, 1.0);
        self.start + (f64::from(fraction) * self.len as f64) as usize
    }
}

/// Shades the A-B loop region, `(start, end, enabled)` in frames, and outlines it.
pub fn paint_loop(
    ui: &egui::Ui,
    painter: &egui::Painter,
    rect: Rect,
    timeline: Timeline,
    loop_region: Option<(usize, usize, bool)>,
) {
    let Some((start, end, enabled)) = loop_region else {
        return;
    };
    let strength = if enabled { 1.0 } else { 0.4 };
    let tint = ui.visuals().strong_text_color();
    let area = Rect::from_x_y_ranges(timeline.x_of(start)..=timeline.x_of(end), rect.y_range());
    painter.rect_filled(area, 0.0, tint.gamma_multiply(0.12 * strength));
    for x in [area.left(), area.right()] {
        painter.vline(
            x,
            rect.y_range(),
            Stroke::new(1.0, tint.gamma_multiply(0.6 * strength)),
        );
    }
}

pub fn marker_color(ui: &egui::Ui) -> Color32 {
    ui.visuals().warn_fg_color
}

/// Draws a stereo waveform (left channel on top, right below) with the overlay,
/// moved in time by `offset` frames. Returns the rect it used, so the ruler can line
/// up with it, and what the user did on it. When `movable`, a drag moves the track
/// instead of selecting a loop.
pub fn waveform(
    ui: &mut egui::Ui,
    peaks: &[Peaks; 2],
    color: Color32,
    height: f32,
    overlay: &Overlay,
    offset: i64,
    movable: bool,
) -> (Rect, Option<WaveformAction>) {
    let (rect, response) =
        ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::click_and_drag());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 2.0, ui.visuals().extreme_bg_color);
    if movable {
        painter.rect_stroke(
            rect.shrink(0.5),
            2.0,
            Stroke::new(1.0, color.gamma_multiply(0.6)),
            egui::StrokeKind::Inside,
        );
        if response.dragged() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
        } else if response.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
        }
    }

    let timeline = Timeline::new(rect.x_range(), &overlay.visible);
    paint_loop(ui, &painter, rect, timeline, overlay.loop_region);
    // Behind the waveform, so it stays readable.
    let grid_color = ui.visuals().widgets.noninteractive.bg_stroke.color;
    for line in overlay.grid {
        let strength = if line.bar { 1.0 } else { 0.45 };
        painter.vline(
            timeline.x_of(line.frame),
            rect.y_range(),
            Stroke::new(1.0, grid_color.gamma_multiply(strength)),
        );
    }

    // Each channel gets half the height, centered on its own axis.
    let half = rect.height() * 0.25 - 1.0;
    let columns = rect.width().ceil() as usize;
    let (first, len) = (timeline.start, timeline.len);
    for (channel, peaks) in peaks.iter().enumerate() {
        let mid = rect.top() + rect.height() * (0.25 + 0.5 * channel as f32);
        for x in 0..columns {
            // Song frames of this column, then the track's frames under them.
            let start = (first + x * len / columns) as i64 - offset;
            let end = (first + (x + 1) * len / columns) as i64 - offset;
            let (lo, hi) = peaks.range(start.max(0) as usize, end.max(0) as usize);
            let px = rect.left() + x as f32 + 0.5;
            let top = mid - hi.min(1.0) * half;
            let bottom = mid - lo.max(-1.0) * half;
            painter.vline(px, top..=bottom.max(top + 1.0), Stroke::new(1.0, color));
        }
    }
    painter.hline(
        rect.x_range(),
        rect.center().y,
        Stroke::new(1.0, ui.visuals().widgets.noninteractive.bg_stroke.color),
    );

    let marker_stroke = Stroke::new(1.0, marker_color(ui).gamma_multiply(0.5));
    for &(frame, _) in overlay.markers {
        painter.vline(timeline.x_of(frame), rect.y_range(), marker_stroke);
    }

    painter.vline(
        timeline.x_of(overlay.position),
        rect.y_range(),
        Stroke::new(1.5, ui.visuals().strong_text_color()),
    );

    let action = response.interact_pointer_pos().and_then(|pointer| {
        if movable && response.dragged() {
            // Measured from where the drag started, so rounding never accumulates.
            if response.drag_started() {
                ui.data_mut(|d| d.insert_temp(response.id, offset));
            }
            let start_offset = ui.data(|d| d.get_temp::<i64>(response.id))?;
            let origin = ui.input(|i| i.pointer.press_origin())?;
            let frames_per_point = timeline.len as f64 / f64::from(rect.width().max(1.0));
            let delta = f64::from(pointer.x - origin.x) * frames_per_point;
            Some(WaveformAction::Move(start_offset + delta.round() as i64))
        } else if response.dragged() {
            let origin = ui.input(|i| i.pointer.press_origin())?;
            let (a, b) = (timeline.frame_at(origin.x), timeline.frame_at(pointer.x));
            Some(WaveformAction::Select(a.min(b), a.max(b)))
        } else if response.clicked() {
            Some(WaveformAction::Seek(timeline.frame_at(pointer.x)))
        } else {
            None
        }
    });
    (rect, action)
}

/// Draws a horizontal scrollbar for the view in the rect of `response`, under the
/// timeline spanning `x_range`. Dragging the thumb scrolls; clicking beside it
/// centers the view there. Draws nothing when the whole song is visible.
pub fn scrollbar(ui: &egui::Ui, response: &egui::Response, x_range: Rangef, view: &mut View) {
    if view.is_full() || view.duration() <= 0.0 {
        return;
    }
    let rect = Rect::from_x_y_ranges(x_range, response.rect.y_range());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 3.0, ui.visuals().faint_bg_color);
    let width = f64::from(x_range.span().max(1.0));
    let secs_per_point = view.duration() / width;
    let x_of = |secs: f64| x_range.min + (secs / secs_per_point) as f32;
    let thumb = Rect::from_x_y_ranges(x_of(view.start)..=x_of(view.end()), rect.y_range());
    let visuals = ui.visuals();
    let color = if response.hovered() || response.dragged() {
        visuals.widgets.hovered.bg_fill
    } else {
        visuals.widgets.inactive.bg_fill
    };
    painter.rect_filled(thumb, 3.0, color);

    if response.dragged() {
        view.pan(f64::from(response.drag_delta().x) * secs_per_point);
    } else if response.clicked()
        && let Some(pointer) = response.interact_pointer_pos()
        && !thumb.contains(pointer)
    {
        view.center_on(f64::from(pointer.x - x_range.min) * secs_per_point);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timeline_maps_both_ways() {
        let timeline = Timeline::new(Rangef::new(100.0, 300.0), &(0..1000));
        assert_eq!(timeline.x_of(0), 100.0);
        assert_eq!(timeline.x_of(500), 200.0);
        assert_eq!(timeline.frame_at(200.0), 500);
        // Outside the range clamps to the ends.
        assert_eq!(timeline.frame_at(50.0), 0);
        assert_eq!(timeline.frame_at(400.0), 1000);

        // Zoomed in on frames 1000..2000.
        let timeline = Timeline::new(Rangef::new(100.0, 300.0), &(1000..2000));
        assert_eq!(timeline.x_of(1500), 200.0);
        assert_eq!(timeline.x_of(0), -100.0);
        assert_eq!(timeline.frame_at(150.0), 1250);
        assert_eq!(timeline.frame_at(0.0), 1000);
    }
}
