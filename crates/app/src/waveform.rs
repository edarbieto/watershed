use eframe::egui::{self, Align2, Color32, FontId, Pos2, Rangef, Rect, Sense, Stroke, vec2};
use watershed_dsp::Peaks;

/// What the user did on a waveform.
pub enum WaveformAction {
    /// Clicked: seek to this frame.
    Seek(usize),
    /// Dragged: select these frames as a loop. Updates while the drag goes on.
    Select(usize, usize),
}

/// What the user did on the ruler.
pub enum RulerAction {
    /// Clicked or dragged: move the playhead to this frame.
    Seek(usize),
    /// Double-clicked: add a marker at this frame.
    AddMarker(usize),
}

/// What is drawn over the waveforms and the ruler. Positions are in frames.
pub struct Overlay<'a> {
    pub position: usize,
    /// `(start, end, enabled)` of the A-B loop.
    pub loop_region: Option<(usize, usize, bool)>,
    /// Marker positions and labels, in time order.
    pub markers: &'a [(usize, String)],
}

/// Height of the time labels row of the ruler; markers go in the row below.
const RULER_TIME_HEIGHT: f32 = 15.0;

/// Seconds between labeled ticks on the ruler, from fine to coarse.
const TICK_STEPS: &[f64] = &[1.0, 2.0, 5.0, 10.0, 15.0, 30.0, 60.0, 120.0, 300.0, 600.0];
/// Minimum distance between labeled ticks, in points.
const MIN_TICK_SPACING: f32 = 60.0;

/// Maps frames to x coordinates across `x_range` and back.
#[derive(Clone, Copy)]
struct Timeline {
    x_range: Rangef,
    total: usize,
}

impl Timeline {
    fn new(x_range: Rangef, total: usize) -> Self {
        Self {
            x_range,
            total: total.max(1),
        }
    }

    fn x_of(self, frame: usize) -> f32 {
        self.x_range.min + frame as f32 / self.total as f32 * self.x_range.span().max(1.0)
    }

    fn frame_at(self, x: f32) -> usize {
        let fraction = ((x - self.x_range.min) / self.x_range.span().max(1.0)).clamp(0.0, 1.0);
        (fraction as f64 * self.total as f64) as usize
    }
}

/// Shades the A-B loop region, `(start, end, enabled)` in frames, and outlines it.
fn paint_loop(
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

fn marker_color(ui: &egui::Ui) -> Color32 {
    ui.visuals().warn_fg_color
}

/// Draws a stereo waveform (left channel on top, right below) with the overlay.
/// Returns the rect it used, so the ruler can line up with it, and what the user did
/// on it.
pub fn waveform(
    ui: &mut egui::Ui,
    peaks: &[Peaks; 2],
    color: Color32,
    height: f32,
    overlay: &Overlay,
) -> (Rect, Option<WaveformAction>) {
    let (rect, response) =
        ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::click_and_drag());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 2.0, ui.visuals().extreme_bg_color);

    let timeline = Timeline::new(rect.x_range(), peaks[0].total_frames);
    paint_loop(ui, &painter, rect, timeline, overlay.loop_region);

    // Each channel gets half the height, centered on its own axis.
    let half = rect.height() * 0.25 - 1.0;
    let columns = rect.width().ceil() as usize;
    let total = timeline.total;
    for (channel, peaks) in peaks.iter().enumerate() {
        let mid = rect.top() + rect.height() * (0.25 + 0.5 * channel as f32);
        for x in 0..columns {
            let start = x * total / columns;
            let end = (x + 1) * total / columns;
            let (lo, hi) = peaks.range(start, end);
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
        if response.dragged() {
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

/// Draws a time ruler with the markers in the rect of `response`, with the timeline
/// spanning `x_range`.
pub fn ruler(
    ui: &egui::Ui,
    response: &egui::Response,
    x_range: Rangef,
    total_frames: usize,
    sample_rate: u32,
    overlay: &Overlay,
) -> Option<RulerAction> {
    let rect = response.rect;
    let time_bottom = rect.top() + RULER_TIME_HEIGHT;
    let painter = ui.painter_at(rect);
    let timeline = Timeline::new(x_range, total_frames);
    let bar = Rect::from_x_y_ranges(x_range, rect.y_range());
    painter.rect_filled(bar, 2.0, ui.visuals().faint_bg_color);
    paint_loop(ui, &painter, bar, timeline, overlay.loop_region);

    let rate = f64::from(sample_rate.max(1));
    let duration = timeline.total as f64 / rate;
    let points_per_sec = x_range.span() as f64 / duration.max(1e-9);
    let step = TICK_STEPS
        .iter()
        .copied()
        .find(|s| s * points_per_sec >= f64::from(MIN_TICK_SPACING))
        .unwrap_or(*TICK_STEPS.last().unwrap_or(&60.0));
    let text_color = ui.visuals().weak_text_color();
    let tick = Stroke::new(1.0, text_color);
    let font = FontId::proportional(10.0);
    // A short unlabeled tick halfway between labeled ones.
    let mut i = 0;
    loop {
        let secs = i as f64 * step / 2.0;
        if secs > duration {
            break;
        }
        let x = timeline.x_of((secs * rate) as usize);
        if i % 2 == 0 {
            painter.vline(x, time_bottom - 6.0..=time_bottom, tick);
            painter.text(
                Pos2::new(x + 3.0, rect.top() + 1.0),
                Align2::LEFT_TOP,
                format_tick(secs),
                font.clone(),
                text_color,
            );
        } else {
            painter.vline(x, time_bottom - 3.0..=time_bottom, tick);
        }
        i += 1;
    }

    // Markers: a line and a label clipped at the next marker, in the lower row.
    let color = marker_color(ui);
    let lane = Rangef::new(time_bottom, rect.bottom());
    for (index, (frame, label)) in overlay.markers.iter().enumerate() {
        let x = timeline.x_of(*frame);
        let next = overlay
            .markers
            .get(index + 1)
            .map_or(x_range.max, |(f, _)| timeline.x_of(*f));
        painter.vline(x, lane, Stroke::new(1.5, color));
        let clip = Rect::from_x_y_ranges(x..=next - 2.0, lane).intersect(rect);
        painter.with_clip_rect(clip).text(
            Pos2::new(x + 3.0, lane.center()),
            Align2::LEFT_CENTER,
            label,
            FontId::proportional(11.0),
            color,
        );
    }

    // The playhead as a small triangle pointing down at the waveforms.
    let x = timeline.x_of(overlay.position);
    let color = ui.visuals().strong_text_color();
    painter.add(egui::Shape::convex_polygon(
        vec![
            Pos2::new(x - 5.0, rect.top() + 3.0),
            Pos2::new(x + 5.0, rect.top() + 3.0),
            Pos2::new(x, time_bottom),
        ],
        color,
        Stroke::NONE,
    ));
    painter.vline(x, lane, Stroke::new(1.5, color));

    // Seeking resets the stretcher, so only seek when the pointer actually moves.
    let pointer = response.interact_pointer_pos()?;
    let inside = x_range.contains(pointer.x);
    let frame = timeline.frame_at(pointer.x);
    if response.double_clicked() && inside {
        Some(RulerAction::AddMarker(frame))
    } else if (response.clicked() && inside)
        || (response.dragged() && response.drag_delta().x != 0.0)
    {
        Some(RulerAction::Seek(frame))
    } else {
        None
    }
}

/// Ruler label: `m:ss`.
fn format_tick(secs: f64) -> String {
    let secs = secs.round() as u64;
    format!("{}:{:02}", secs / 60, secs % 60)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timeline_maps_both_ways() {
        let timeline = Timeline::new(Rangef::new(100.0, 300.0), 1000);
        assert_eq!(timeline.x_of(0), 100.0);
        assert_eq!(timeline.x_of(500), 200.0);
        assert_eq!(timeline.frame_at(200.0), 500);
        // Outside the range clamps to the ends.
        assert_eq!(timeline.frame_at(50.0), 0);
        assert_eq!(timeline.frame_at(400.0), 1000);
    }

    #[test]
    fn tick_labels() {
        assert_eq!(format_tick(0.0), "0:00");
        assert_eq!(format_tick(75.0), "1:15");
    }
}
