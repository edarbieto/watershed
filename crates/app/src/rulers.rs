//! The rulers above the tracks, one row per job. On top, by time: the seconds (and the
//! playhead's handle), the A-B loop and the markers. Then the beat grid: where bar 1
//! starts (with its time signature), the tempo and time signature changes, and the bar
//! numbers. The chord lane (`chords.rs`) goes last.
//!
//! Each function draws one row in the rect of its `response`, with the timeline
//! spanning `x_range`, and returns what the user did there. Empty rows show a hint.

use eframe::egui::{self, Align2, FontId, Pos2, Rangef, Rect, Sense, Stroke, vec2};

use crate::waveform::{Overlay, SignatureTag, Timeline, marker_color, paint_loop};

/// Seconds between labeled ticks on the time row, from fine to coarse.
const TICK_STEPS: &[f64] = &[1.0, 2.0, 5.0, 10.0, 15.0, 30.0, 60.0, 120.0, 300.0, 600.0];
/// Minimum distance between labeled ticks, in points.
const MIN_TICK_SPACING: f32 = 60.0;
/// A drag that starts this close to an end of the loop, in points, moves that end.
const LOOP_HANDLE: f32 = 6.0;

/// What the user did on the time row or the markers row.
pub enum RulerAction {
    /// Clicked or dragged on the time row: move the playhead to this frame.
    Seek(usize),
    /// Clicked a marker: go exactly to it.
    GoTo(usize),
    /// Double-clicked the markers row: add a marker at this frame.
    AddMarker(usize),
}

/// What the user did on the loop row.
pub enum LoopAction {
    /// Dragged: the loop is now these frames. Updates while the drag goes on.
    Select(usize, usize),
    /// Clicked inside the loop: turn looping on or off.
    Toggle,
}

/// What the user did on the grid rows.
pub enum GridAction {
    /// Dragged the bar 1 row: move bar 1 by this many seconds from where it was when
    /// the drag began (`started` on the first frame of the drag).
    Drag { started: bool, delta_secs: f64 },
    /// Double-clicked the bar 1 row: back to the estimated bar 1.
    Reset,
    /// Right-clicked the changes row at this frame: change the tempo or time
    /// signature there. `at` is where the pointer was, for the editor.
    AddChange { frame: usize, at: Pos2 },
    /// Clicked a tag (`None` is bar 1's time signature): edit it in a popup at `at`.
    EditSignature { change: Option<usize>, at: Pos2 },
    /// Dragged a change: move it by this many seconds from where it was when the
    /// drag began, like [`GridAction::Drag`].
    MoveSignature {
        change: usize,
        started: bool,
        delta_secs: f64,
    },
    /// Clicked or dragged the bar numbers: go to the grid line nearest this frame.
    Seek(usize),
}

/// The row's rect over the timeline, its painter (clipped to it) and the timeline,
/// with the row's background painted.
fn row(
    ui: &egui::Ui,
    response: &egui::Response,
    x_range: Rangef,
    overlay: &Overlay,
) -> (Rect, egui::Painter, Timeline) {
    let rect = Rect::from_x_y_ranges(x_range, response.rect.y_range());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 2.0, ui.visuals().faint_bg_color);
    (rect, painter, Timeline::new(x_range, &overlay.visible))
}

/// The playhead's line across a row.
fn paint_playhead(ui: &egui::Ui, painter: &egui::Painter, rect: Rect, x: f32) {
    painter.vline(
        x,
        rect.y_range(),
        Stroke::new(1.5, ui.visuals().strong_text_color()),
    );
}

/// A faint hint at the left of an empty row.
fn paint_hint(ui: &egui::Ui, painter: &egui::Painter, rect: Rect, text: &str) {
    painter.text(
        rect.left_center() + vec2(4.0, 0.0),
        Align2::LEFT_CENTER,
        text,
        FontId::proportional(10.0),
        ui.visuals().weak_text_color().gamma_multiply(0.7),
    );
}

/// Seconds the pointer moved since the drag of `response` began, at the timeline's
/// scale.
fn drag_secs(
    ui: &egui::Ui,
    response: &egui::Response,
    timeline: Timeline,
    sample_rate: u32,
) -> Option<f64> {
    let pointer = response.interact_pointer_pos()?;
    let origin = ui.input(|i| i.pointer.press_origin())?;
    let frames_per_point = timeline.len as f64 / f64::from(timeline.x_range.span().max(1.0));
    Some(f64::from(pointer.x - origin.x) * frames_per_point / f64::from(sample_rate.max(1)))
}

/// The time row: `m:ss` ticks and the playhead's handle. A click or a drag seeks.
pub fn time_row(
    ui: &egui::Ui,
    response: &egui::Response,
    x_range: Rangef,
    sample_rate: u32,
    overlay: &Overlay,
) -> Option<RulerAction> {
    let (rect, painter, timeline) = row(ui, response, x_range, overlay);
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
            painter.vline(x, rect.bottom() - 6.0..=rect.bottom(), tick);
            painter.text(
                Pos2::new(x + 3.0, rect.top() + 1.0),
                Align2::LEFT_TOP,
                format_tick(secs),
                font.clone(),
                text_color,
            );
        } else {
            painter.vline(x, rect.bottom() - 3.0..=rect.bottom(), tick);
        }
        i += 1;
    }

    // The playhead as a small triangle pointing down at the rows below.
    let x = timeline.x_of(overlay.position);
    painter.add(egui::Shape::convex_polygon(
        vec![
            Pos2::new(x - 5.0, rect.top() + 2.0),
            Pos2::new(x + 5.0, rect.top() + 2.0),
            Pos2::new(x, rect.bottom()),
        ],
        ui.visuals().strong_text_color(),
        Stroke::NONE,
    ));

    // Seeking resets the stretcher, so only seek when the pointer actually moves.
    let pointer = response.interact_pointer_pos()?;
    let inside = x_range.contains(pointer.x);
    ((response.clicked() && inside) || (response.dragged() && response.drag_delta().x != 0.0))
        .then(|| RulerAction::Seek(timeline.frame_at(pointer.x)))
}

/// The loop row: the A-B loop as a bar. Dragging where there is no loop selects a
/// new one; dragging near one of its ends moves that end; a click inside it turns
/// looping on or off.
pub fn loop_row(
    ui: &egui::Ui,
    response: &egui::Response,
    x_range: Rangef,
    overlay: &Overlay,
) -> Option<LoopAction> {
    let (rect, painter, timeline) = row(ui, response, x_range, overlay);
    let ends = overlay
        .loop_region
        .map(|(a, b, enabled)| (timeline.x_of(a), timeline.x_of(b), a, b, enabled));
    match ends {
        Some((left, right, _, _, enabled)) => {
            let visuals = ui.visuals();
            let fill = if enabled {
                visuals.selection.bg_fill
            } else {
                visuals.widgets.inactive.bg_fill
            };
            let bar = Rect::from_x_y_ranges(left..=right, rect.y_range().shrink(3.0));
            painter.rect_filled(bar, 2.0, fill);
            for x in [left, right] {
                painter.vline(
                    x,
                    rect.y_range(),
                    Stroke::new(2.0, visuals.strong_text_color()),
                );
            }
        }
        None => paint_hint(ui, &painter, rect, "Drag here to set a loop"),
    }
    paint_playhead(ui, &painter, rect, timeline.x_of(overlay.position));

    let near_end = |x: f32| {
        ends.and_then(|(left, right, a, b, _)| {
            // The anchor is the end that stays.
            if (x - left).abs() <= LOOP_HANDLE {
                Some(b)
            } else if (x - right).abs() <= LOOP_HANDLE {
                Some(a)
            } else {
                None
            }
        })
    };
    if let Some(hover) = response.hover_pos()
        && (near_end(hover.x).is_some() || response.dragged())
    {
        ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
    }
    let pointer = response.interact_pointer_pos()?;
    if response.drag_started() {
        let origin = ui.input(|i| i.pointer.press_origin())?;
        let anchor = near_end(origin.x).unwrap_or_else(|| timeline.frame_at(origin.x));
        ui.data_mut(|d| d.insert_temp(response.id, anchor));
    }
    if response.dragged() {
        let anchor = ui.data(|d| d.get_temp::<usize>(response.id))?;
        let frame = timeline.frame_at(pointer.x);
        return Some(LoopAction::Select(anchor.min(frame), anchor.max(frame)));
    }
    let inside = ends.is_some_and(|(left, right, ..)| (left..=right).contains(&pointer.x));
    (response.clicked() && inside).then_some(LoopAction::Toggle)
}

/// The markers row: a line and a label each, clipped at the next marker. A click on
/// a marker goes to it; a double-click adds one.
pub fn markers_row(
    ui: &egui::Ui,
    response: &egui::Response,
    x_range: Rangef,
    overlay: &Overlay,
) -> Option<RulerAction> {
    let (rect, painter, timeline) = row(ui, response, x_range, overlay);
    let color = marker_color(ui);
    let mut hit = None;
    let pointer = response.interact_pointer_pos();
    for (index, (frame, label)) in overlay.markers.iter().enumerate() {
        let x = timeline.x_of(*frame);
        let next = overlay
            .markers
            .get(index + 1)
            .map_or(x_range.max, |(f, _)| timeline.x_of(*f));
        painter.vline(x, rect.y_range(), Stroke::new(1.5, color));
        let clip = Rect::from_x_y_ranges(x..=next - 2.0, rect.y_range()).intersect(rect);
        let galley = painter.layout_no_wrap(label.clone(), FontId::proportional(11.0), color);
        let label_rect = Rect::from_min_size(
            Pos2::new(x, rect.center().y - galley.size().y / 2.0),
            galley.size() + vec2(6.0, 0.0),
        )
        .intersect(clip);
        painter
            .with_clip_rect(clip)
            .galley(label_rect.min + vec2(3.0, 0.0), galley, color);
        if pointer.is_some_and(|p| label_rect.contains(p) || (p.x - x).abs() <= 3.0) {
            hit = Some(*frame);
        }
    }
    if overlay.markers.is_empty() {
        paint_hint(ui, &painter, rect, "Double-click here to add a marker");
    }
    paint_playhead(ui, &painter, rect, timeline.x_of(overlay.position));

    let pointer = pointer?;
    if response.double_clicked() && x_range.contains(pointer.x) {
        Some(RulerAction::AddMarker(timeline.frame_at(pointer.x)))
    } else if response.clicked() {
        hit.map(RulerAction::GoTo)
    } else {
        None
    }
}

/// A tag on a grid row: a line up from its x and a label on a dark background.
/// Returns its rect, clipped to the row, when visible.
fn paint_tag(
    ui: &egui::Ui,
    painter: &egui::Painter,
    rect: Rect,
    x: f32,
    label: &str,
    color: egui::Color32,
) -> Option<Rect> {
    let galley = painter.layout_no_wrap(label.to_owned(), FontId::proportional(10.0), color);
    let tag_rect = Rect::from_min_size(
        Pos2::new(x, rect.center().y - (galley.size().y + 1.0) / 2.0),
        galley.size() + vec2(6.0, 1.0),
    );
    let visible = tag_rect.intersect(rect);
    if !visible.is_positive() {
        return None;
    }
    painter.rect_filled(tag_rect, 2.0, ui.visuals().extreme_bg_color);
    painter.vline(x, rect.y_range(), Stroke::new(1.5, color));
    painter.galley(tag_rect.min + vec2(4.0, 0.5), galley, color);
    Some(visible)
}

/// The bar 1 row: a flag where bar 1 starts, with its time signature. Dragging the
/// row moves bar 1; a double-click puts it back where it was estimated; a click on
/// the flag edits the time signature. `enabled` false (no grid) draws it empty.
pub fn bar_one_row(
    ui: &egui::Ui,
    response: &egui::Response,
    x_range: Rangef,
    sample_rate: u32,
    overlay: &Overlay,
    bar_one: Option<&SignatureTag>,
    enabled: bool,
) -> Option<GridAction> {
    let (rect, painter, timeline) = row(ui, response, x_range, overlay);
    if !enabled {
        return None;
    }
    let flag = bar_one.and_then(|tag| {
        let x = timeline.x_of(tag.frame);
        let visible = paint_tag(ui, &painter, rect, x, &tag.label, marker_color(ui))?;
        Some((x, visible))
    });
    paint_playhead(ui, &painter, rect, timeline.x_of(overlay.position));
    if let Some((x, visible)) = flag {
        let flag_response = ui
            .interact(visible, response.id.with("bar one"), Sense::click())
            .on_hover_text("Bar 1 and its time signature. Click to change the time signature.");
        if flag_response.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        if flag_response.clicked() {
            return Some(GridAction::EditSignature {
                change: None,
                at: Pos2::new(x, rect.bottom()),
            });
        }
    }

    if response.hovered() || response.dragged() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
    }
    if response.double_clicked() {
        return Some(GridAction::Reset);
    }
    if response.dragged() {
        return Some(GridAction::Drag {
            started: response.drag_started(),
            delta_secs: drag_secs(ui, response, timeline, sample_rate)?,
        });
    }
    None
}

/// The changes row: a tag per tempo or time signature change. A right-click adds
/// one; a click on a tag edits it and a drag moves it.
pub fn changes_row(
    ui: &egui::Ui,
    response: &egui::Response,
    x_range: Rangef,
    sample_rate: u32,
    overlay: &Overlay,
    changes: &[SignatureTag],
    enabled: bool,
) -> Option<GridAction> {
    let (rect, painter, timeline) = row(ui, response, x_range, overlay);
    if !enabled {
        return None;
    }
    if changes.is_empty() {
        paint_hint(
            ui,
            &painter,
            rect,
            "Right-click here to change the tempo or time signature",
        );
    }
    let mut action = None;
    // Tags drawn later (later in the song) cover earlier ones.
    for (index, tag) in changes.iter().enumerate() {
        let Some(change) = tag.change else { continue };
        let x = timeline.x_of(tag.frame);
        let accent = marker_color(ui);
        let color = if tag.in_use {
            accent
        } else {
            accent.gamma_multiply(0.4)
        };
        let Some(visible) = paint_tag(ui, &painter, rect, x, &tag.label, color) else {
            continue;
        };
        let tag_response = ui
            .interact(
                visible,
                response.id.with(("change", index)),
                Sense::click_and_drag(),
            )
            .on_hover_text(
                "Tempo or time signature from here on. Click to change or delete it, drag \
                 to move it.",
            );
        if tag_response.hovered() || tag_response.dragged() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
        }
        if tag_response.clicked() {
            action = Some(GridAction::EditSignature {
                change: Some(change),
                at: Pos2::new(x, rect.bottom()),
            });
        } else if tag_response.dragged()
            && let Some(delta_secs) = drag_secs(ui, &tag_response, timeline, sample_rate)
        {
            action = Some(GridAction::MoveSignature {
                change,
                started: tag_response.drag_started(),
                delta_secs,
            });
        }
    }
    paint_playhead(ui, &painter, rect, timeline.x_of(overlay.position));
    if action.is_some() {
        return action;
    }
    if response.secondary_clicked() {
        let at = response.interact_pointer_pos()?;
        return Some(GridAction::AddChange {
            frame: timeline.frame_at(at.x),
            at,
        });
    }
    None
}

/// The bars row: bar numbers, and beats when zoomed in. A click or a drag goes to
/// the nearest grid line.
pub fn bars_row(
    ui: &egui::Ui,
    response: &egui::Response,
    x_range: Rangef,
    overlay: &Overlay,
    enabled: bool,
) -> Option<GridAction> {
    let (rect, painter, timeline) = row(ui, response, x_range, overlay);
    if !enabled {
        return None;
    }
    let text_color = ui.visuals().weak_text_color();
    let font = FontId::proportional(10.0);
    for line in overlay.grid {
        let x = timeline.x_of(line.frame);
        let height = if line.bar { rect.height() } else { 5.0 };
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
                Pos2::new(x + 3.0, rect.top() + 1.0),
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
    paint_playhead(ui, &painter, rect, timeline.x_of(overlay.position));

    let pointer = response.interact_pointer_pos()?;
    let inside = x_range.contains(pointer.x);
    ((response.clicked() && inside) || (response.dragged() && response.drag_delta().x != 0.0))
        .then(|| GridAction::Seek(timeline.frame_at(pointer.x)))
}

/// Time row label: `m:ss`.
fn format_tick(secs: f64) -> String {
    let secs = secs.round() as u64;
    format!("{}:{:02}", secs / 60, secs % 60)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tick_labels() {
        assert_eq!(format_tick(0.0), "0:00");
        assert_eq!(format_tick(75.0), "1:15");
    }
}
