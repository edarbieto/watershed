//! The chord lane: the song's chords, estimated on the beat grid (one per bar, or
//! per half bar when the halves clearly differ), shown under the bar ruler. A rough
//! guide, like the key. Estimated again whenever the grid's beats change; stored
//! nowhere. Shown as heard: transposed with the pitch control.

use eframe::egui::{self, Align2, FontId, Rangef, Rect, Stroke};
use watershed_audio::Beats;
use watershed_dsp::{Chord, estimate_chords, half_bars};

use crate::loader::Harmony;
use crate::view::View;

/// Space left of a chord name inside its block, in points.
const PADDING: f32 = 4.0;
/// Font sizes of the chord names: the first that fits its block.
const FONT_SIZES: [f32; 2] = [13.0, 10.0];

/// A run of spans with the same chord.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Block {
    pub start: f64,
    pub end: f64,
    pub chord: Chord,
}

#[derive(Debug, Default)]
pub struct SongChords {
    /// The beat times and bar starts the blocks were estimated on.
    estimated_on: Option<(Vec<f64>, Vec<usize>)>,
    blocks: Vec<Block>,
}

impl SongChords {
    /// Estimates the chords again if `beats` changed since the last time. No beats
    /// (no grid) means no chords.
    pub fn update(&mut self, harmony: &Harmony, beats: Option<&Beats>) {
        let Some(beats) = beats else {
            self.estimated_on = None;
            self.blocks.clear();
            return;
        };
        let times = beats.times();
        let bar_starts: Vec<usize> = (0..times.len())
            .filter(|&i| beats.accented(i as i64))
            .collect();
        if self
            .estimated_on
            .as_ref()
            .is_some_and(|(t, b)| t == times && *b == bar_starts)
        {
            return;
        }
        let start = std::time::Instant::now();
        let spans = half_bars(times, &bar_starts);
        let chords = estimate_chords(
            &harmony.treble,
            harmony.bass.as_ref(),
            &harmony.chroma,
            &spans,
        );
        self.blocks = merge(spans.iter().map(|s| (s.start, s.end)).zip(chords));
        log::debug!(
            "{} chords on {} spans in {:.1?}",
            self.blocks.len(),
            spans.len(),
            start.elapsed()
        );
        self.estimated_on = Some((times.to_vec(), bar_starts));
    }

    pub fn blocks(&self) -> &[Block] {
        &self.blocks
    }
}

/// Joins consecutive spans with the same chord into blocks; spans with no chord are
/// left out.
fn merge(spans: impl Iterator<Item = ((f64, f64), Option<Chord>)>) -> Vec<Block> {
    let mut blocks: Vec<Block> = Vec::new();
    for ((start, end), chord) in spans {
        let Some(chord) = chord else { continue };
        match blocks.last_mut() {
            Some(last) if last.chord == chord && (last.end - start).abs() < 1e-6 => {
                last.end = end;
            }
            _ => blocks.push(Block { start, end, chord }),
        }
    }
    blocks
}

/// What the lane shows and how.
pub struct Lane<'a> {
    pub blocks: &'a [Block],
    /// Whole semitones the song is transposed by.
    pub semitones: i32,
    /// Spell the names with flats (from the key as heard).
    pub flats: bool,
    pub playhead: f64,
    /// The lane is on and the grid has beats.
    pub shown: bool,
}

/// Draws the chord lane in `response`'s row across `x_range`. Returns the time to
/// seek to when a chord is clicked: its start.
pub fn lane(
    ui: &egui::Ui,
    response: &egui::Response,
    x_range: Rangef,
    view: &View,
    lane: &Lane,
) -> Option<f64> {
    let rect = Rect::from_x_y_ranges(x_range, response.rect.y_range());
    let painter = ui.painter_at(rect);
    let visuals = ui.visuals();
    painter.rect_filled(rect, 2.0, visuals.faint_bg_color);
    if !lane.shown {
        return None;
    }
    let span = view.span.max(1e-9);
    let x_of = |secs: f64| x_range.min + ((secs - view.start) / span) as f32 * x_range.span();
    let mut clicked = None;
    let pointer = response
        .clicked()
        .then(|| response.interact_pointer_pos())
        .flatten();
    for block in lane.blocks {
        if block.end < view.start || block.start > view.end() {
            continue;
        }
        let (left, right) = (x_of(block.start), x_of(block.end));
        let area = Rect::from_x_y_ranges(left + 0.5..=right - 0.5, rect.y_range().shrink(2.0));
        let current = (block.start..block.end).contains(&lane.playhead);
        let (fill, text) = if current {
            (visuals.selection.bg_fill, visuals.strong_text_color())
        } else {
            (visuals.widgets.inactive.bg_fill, visuals.text_color())
        };
        painter.rect_filled(area, 3.0, fill);
        painter.vline(
            left,
            area.y_range(),
            Stroke::new(1.0, visuals.weak_text_color()),
        );
        // The name stays in sight while its block scrolls past the left edge. A
        // smaller font when it does not fit; none when that does not fit either.
        let name = block.chord.transposed(lane.semitones).name(lane.flats);
        let x = left.max(rect.left()) + PADDING;
        let room = right.min(rect.right()) - x;
        let galley = FONT_SIZES
            .iter()
            .map(|&size| painter.layout_no_wrap(name.clone(), FontId::proportional(size), text))
            .find(|galley| galley.size().x <= room);
        if let Some(galley) = galley {
            painter.galley(
                egui::pos2(x, rect.center().y - galley.size().y / 2.0),
                galley,
                text,
            );
        }
        if pointer.is_some_and(|p| area.contains(p)) {
            clicked = Some(block.start);
        }
    }
    if lane.blocks.is_empty() {
        painter.text(
            rect.left_center() + egui::vec2(PADDING, 0.0),
            Align2::LEFT_CENTER,
            "No chords found",
            FontId::proportional(11.0),
            visuals.weak_text_color(),
        );
    }
    clicked
}

#[cfg(test)]
mod tests {
    use super::*;
    use watershed_dsp::Quality;

    #[test]
    fn merges_runs_and_skips_silence() {
        let c = Some(Chord::new(0, Quality::Major));
        let am = Some(Chord::new(9, Quality::Minor));
        let spans = [
            ((0.0, 1.0), c),
            ((1.0, 2.0), c),
            ((2.0, 3.0), am),
            ((3.0, 4.0), None),
            ((4.0, 5.0), am),
        ];
        let blocks = merge(spans.into_iter());
        let ranges: Vec<(f64, f64)> = blocks.iter().map(|b| (b.start, b.end)).collect();
        assert_eq!(ranges, [(0.0, 2.0), (2.0, 3.0), (4.0, 5.0)]);
    }
}
