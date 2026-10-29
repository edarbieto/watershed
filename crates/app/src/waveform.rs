use std::ops::Range;

use eframe::egui::{self, Align2, Color32, FontId, Pos2, Rangef, Rect, Sense, Stroke, vec2};
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

/// What the user did on the ruler.
pub enum RulerAction {
    /// Clicked or dragged: move the playhead to this frame.
    Seek(usize),
    /// Double-clicked: add a marker at this frame.
    AddMarker(usize),
}

/// What the user did on the bar ruler.
pub enum BarRulerAction {
    /// Dragged: move bar 1 by this many seconds from where it was when the drag
    /// began (`started` on the first frame of the drag).
    Drag { started: bool, delta_secs: f64 },
    /// Double-clicked: back to the estimated bar 1.
    Reset,
    /// Right-clicked at this frame: change the tempo or time signature there. `at`
    /// is where the pointer was, for the editor.
    AddChange { frame: usize, at: Pos2 },
    /// Clicked a tag (`None` is bar 1's time signature): edit it in a popup at `at`.
    EditSignature { change: Option<usize>, at: Pos2 },
    /// Dragged a change: move it by this many seconds from where it was when the
    /// drag began, like [`BarRulerAction::Drag`].
    MoveSignature {
        change: usize,
        started: bool,
        delta_secs: f64,
    },
}

/// A tag on the bar ruler: bar 1's time signature, or a tempo or time signature
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

/// Height of the time labels row of the ruler; markers go in the row below.
const RULER_TIME_HEIGHT: f32 = 15.0;
/// Height of the time signatures row of the bar ruler; bars go in the row below.
const SIGNATURE_ROW_HEIGHT: f32 = 14.0;

/// Seconds between labeled ticks on the ruler, from fine to coarse.
const TICK_STEPS: &[f64] = &[1.0, 2.0, 5.0, 10.0, 15.0, 30.0, 60.0, 120.0, 300.0, 600.0];
/// Minimum distance between labeled ticks, in points.
const MIN_TICK_SPACING: f32 = 60.0;

/// Maps the visible frames to x coordinates across `x_range` and back. Frames
/// outside the view map outside the range, where painting is clipped.
#[derive(Clone, Copy)]
struct Timeline {
    x_range: Rangef,
    start: usize,
    len: usize,
}

impl Timeline {
    fn new(x_range: Rangef, visible: &Range<usize>) -> Self {
        Self {
            x_range,
            start: visible.start,
            len: visible.len().max(1),
        }
    }

    fn x_of(self, frame: usize) -> f32 {
        let fraction = (frame as f64 - self.start as f64) / self.len as f64;
        self.x_range.min + (fraction * f64::from(self.x_range.span().max(1.0))) as f32
    }

    fn frame_at(self, x: f32) -> usize {
        let fraction = ((x - self.x_range.min) / self.x_range.span().max(1.0)).clamp(0.0, 1.0);
        self.start + (f64::from(fraction) * self.len as f64) as usize
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

/// Draws a time ruler with the markers in the rect of `response`, with the timeline
/// spanning `x_range`.
pub fn ruler(
    ui: &egui::Ui,
    response: &egui::Response,
    x_range: Rangef,
    sample_rate: u32,
    overlay: &Overlay,
) -> Option<RulerAction> {
    // Painted over the timeline only, in case the response is a bit wider.
    let rect = Rect::from_x_y_ranges(x_range, response.rect.y_range());
    let time_bottom = rect.top() + RULER_TIME_HEIGHT;
    let painter = ui.painter_at(rect);
    let timeline = Timeline::new(x_range, &overlay.visible);
    painter.rect_filled(rect, 2.0, ui.visuals().faint_bg_color);
    paint_loop(ui, &painter, rect, timeline, overlay.loop_region);

    let rate = f64::from(sample_rate.max(1));
    let (start, end) = (
        overlay.visible.start as f64 / rate,
        overlay.visible.end as f64 / rate,
    );
    let points_per_sec = f64::from(x_range.span()) / (end - start).max(1e-9);
    let step = TICK_STEPS
        .iter()
        .copied()
        .find(|s| s * points_per_sec >= f64::from(MIN_TICK_SPACING))
        .unwrap_or(*TICK_STEPS.last().unwrap_or(&60.0));
    let text_color = ui.visuals().weak_text_color();
    let tick = Stroke::new(1.0, text_color);
    let font = FontId::proportional(10.0);
    // A short unlabeled tick halfway between labeled ones.
    let half_step = step / 2.0;
    let mut i = (start / half_step).floor() as u64;
    loop {
        let secs = i as f64 * half_step;
        if secs > end {
            break;
        }
        let x = timeline.x_of((secs * rate) as usize);
        if i.is_multiple_of(2) {
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

/// Draws the bar ruler (tempo and time signature tags in the top row; bar numbers,
/// and beats when zoomed in, below) in the rect of `response`, with the timeline
/// spanning `x_range`. Dragging it moves the grid; a double-click puts bar 1 back
/// where it was estimated; a right-click adds a change there. A tag is edited with
/// a click and a change moved by dragging it. `enabled` false draws
/// it empty and ignores the pointer.
pub fn bar_ruler(
    ui: &egui::Ui,
    response: &egui::Response,
    x_range: Rangef,
    sample_rate: u32,
    overlay: &Overlay,
    signatures: &[SignatureTag],
    enabled: bool,
) -> Option<BarRulerAction> {
    let rect = Rect::from_x_y_ranges(x_range, response.rect.y_range());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 2.0, ui.visuals().faint_bg_color);
    let text_color = ui.visuals().weak_text_color();
    if !enabled {
        return None;
    }
    let timeline = Timeline::new(x_range, &overlay.visible);
    let frames_per_point = timeline.len as f64 / f64::from(x_range.span().max(1.0));
    let drag_secs = |response: &egui::Response| {
        let pointer = response.interact_pointer_pos()?;
        let origin = ui.input(|i| i.pointer.press_origin())?;
        Some(f64::from(pointer.x - origin.x) * frames_per_point / f64::from(sample_rate.max(1)))
    };
    let font = FontId::proportional(10.0);
    let bars_top = rect.top() + SIGNATURE_ROW_HEIGHT;
    for line in overlay.grid {
        let x = timeline.x_of(line.frame);
        let height = if line.bar {
            rect.bottom() - bars_top
        } else {
            5.0
        };
        painter.vline(
            x,
            rect.bottom() - height..=rect.bottom(),
            Stroke::new(
                1.0,
                text_color.gamma_multiply(if line.bar { 1.0 } else { 0.6 }),
            ),
        );
        if let Some(label) = &line.label {
            painter.text(
                Pos2::new(x + 3.0, bars_top + 1.0),
                Align2::LEFT_TOP,
                label,
                font.clone(),
                if line.bar {
                    ui.visuals().text_color()
                } else {
                    text_color
                },
            );
        }
    }
    painter.vline(
        timeline.x_of(overlay.position),
        rect.y_range(),
        Stroke::new(1.5, ui.visuals().strong_text_color()),
    );

    // Tempo and time signatures: a tag each, over the bar line they start at. Tags drawn
    // later (later in the song) cover earlier ones.
    let mut action = None;
    for (index, tag) in signatures.iter().enumerate() {
        let x = timeline.x_of(tag.frame);
        let accent = marker_color(ui);
        let color = if tag.in_use {
            accent
        } else {
            accent.gamma_multiply(0.4)
        };
        let galley = painter.layout_no_wrap(tag.label.clone(), font.clone(), color);
        let tag_rect = Rect::from_min_size(
            Pos2::new(x, rect.top() + 1.0),
            galley.size() + vec2(6.0, 1.0),
        );
        let visible = tag_rect.intersect(rect);
        if !visible.is_positive() {
            continue;
        }
        painter.rect_filled(tag_rect, 2.0, ui.visuals().extreme_bg_color);
        painter.vline(x, rect.top()..=bars_top, Stroke::new(1.5, color));
        painter.galley(tag_rect.min + vec2(4.0, 0.5), galley, color);

        // Bar 1's is only clicked: dragging it drags the ruler, which moves bar 1.
        let sense = if tag.change.is_some() {
            Sense::click_and_drag()
        } else {
            Sense::click()
        };
        let tag_response = ui
            .interact(visible, response.id.with(("signature", index)), sense)
            .on_hover_text(match tag.change {
                Some(_) => {
                    "Tempo or time signature from here on. Click to change or delete it, drag \
                     to move it."
                }
                None => "Time signature from bar 1. Click to change it.",
            });
        if tag_response.hovered() || tag_response.dragged() {
            ui.ctx().set_cursor_icon(if tag.change.is_some() {
                egui::CursorIcon::ResizeHorizontal
            } else {
                egui::CursorIcon::PointingHand
            });
        }
        let at = Pos2::new(x, rect.bottom());
        if tag_response.clicked() {
            action = Some(BarRulerAction::EditSignature {
                change: tag.change,
                at,
            });
        } else if let Some(change) = tag.change
            && tag_response.dragged()
            && let Some(delta_secs) = drag_secs(&tag_response)
        {
            action = Some(BarRulerAction::MoveSignature {
                change,
                started: tag_response.drag_started(),
                delta_secs,
            });
        }
    }
    if action.is_some() {
        return action;
    }

    if response.hovered() || response.dragged() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
    }
    if response.double_clicked() {
        return Some(BarRulerAction::Reset);
    }
    if response.secondary_clicked() {
        let at = response.interact_pointer_pos()?;
        return Some(BarRulerAction::AddChange {
            frame: timeline.frame_at(at.x),
            at,
        });
    }
    if response.dragged() {
        return Some(BarRulerAction::Drag {
            started: response.drag_started(),
            delta_secs: drag_secs(response)?,
        });
    }
    None
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

    #[test]
    fn tick_labels() {
        assert_eq!(format_tick(0.0), "0:00");
        assert_eq!(format_tick(75.0), "1:15");
    }
}
