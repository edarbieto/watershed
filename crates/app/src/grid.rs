//! The beat grid: bars and beats from the song's BPM, a time signature and where
//! bar 1 starts. Nothing is drawn before bar 1, so a free intro stays clear. Times
//! are in seconds of the song.

use watershed_dsp::OnsetEnvelope;
use watershed_storage::Session;

/// Lines closer than this, in points, are thinned out.
const MIN_LINE_SPACING: f32 = 12.0;
/// Bar numbers closer than this, in points, are thinned out.
const MIN_BAR_LABEL_SPACING: f32 = 32.0;
/// Beats get a label (`3.2`) when they are at least this far apart, in points.
const MIN_BEAT_LABEL_SPACING: f32 = 40.0;
pub const MAX_BEATS_PER_BAR: u32 = 32;
/// Note values a beat can be (the bottom of the time signature).
pub const BEAT_UNITS: [u32; 4] = [2, 4, 8, 16];

#[derive(Debug)]
pub struct Grid {
    /// Off hides the grid and greys out the BPM, for songs without a steady beat.
    pub enabled: bool,
    pub beats_per_bar: u32,
    pub beat_unit: u32,
    /// Where bar 1 starts, set by the user.
    user_start: Option<f64>,
    /// Bar 1 as estimated from the audio, and the BPM it was estimated for.
    estimate: Option<(f64, f64)>,
}

impl Default for Grid {
    fn default() -> Self {
        Self {
            enabled: true,
            beats_per_bar: 4,
            beat_unit: 4,
            user_start: None,
            estimate: None,
        }
    }
}

/// One line of the grid.
#[derive(Clone, Debug, PartialEq)]
pub struct Line {
    pub secs: f64,
    /// The start of a bar, drawn stronger.
    pub bar: bool,
    /// `"12"` for a bar, `"12.3"` for a beat, when there is room for it.
    pub label: Option<String>,
}

impl Grid {
    pub fn from_session(session: &Session) -> Self {
        let beat_unit = if BEAT_UNITS.contains(&session.beat_unit) {
            session.beat_unit
        } else {
            4
        };
        Self {
            enabled: session.grid,
            beats_per_bar: session.beats_per_bar.clamp(1, MAX_BEATS_PER_BAR),
            beat_unit,
            user_start: session.grid_start.filter(|s| s.is_finite() && *s >= 0.0),
            estimate: None,
        }
    }

    /// Where bar 1 starts, if known.
    pub fn start(&self) -> Option<f64> {
        self.user_start.or(self.estimate.map(|(_, start)| start))
    }

    /// The user's bar 1, to save in the session.
    pub fn user_start(&self) -> Option<f64> {
        self.user_start
    }

    pub fn set_start(&mut self, secs: f64) {
        self.user_start = Some(secs.max(0.0));
    }

    /// Back to the estimated bar 1.
    pub fn reset_start(&mut self) {
        self.user_start = None;
    }

    /// Estimates bar 1 again if the BPM changed since the last estimate.
    pub fn update_estimate(&mut self, bpm: Option<f64>, onsets: &OnsetEnvelope) {
        let Some(bpm) = bpm else {
            self.estimate = None;
            return;
        };
        if self.estimate.is_some_and(|(for_bpm, _)| for_bpm == bpm) {
            return;
        }
        self.estimate = onsets.first_beat(bpm).map(|start| (bpm, start));
    }

    /// The lines between `from` and `to` seconds at `bpm`, thinned out for
    /// `points_per_sec` so they never crowd.
    pub fn lines(&self, bpm: f64, from: f64, to: f64, points_per_sec: f64) -> Vec<Line> {
        let Some(start) = self.start() else {
            return Vec::new();
        };
        if bpm <= 0.0 || points_per_sec <= 0.0 || to <= from {
            return Vec::new();
        }
        let beat = 60.0 / bpm;
        let per_bar = self.beats_per_bar.max(1) as usize;
        let points = |secs: f64| (secs * points_per_sec) as f32;
        // Every beat, or every n-th bar with n a power of two.
        let (step, step_secs) = if points(beat) >= MIN_LINE_SPACING {
            (1, beat)
        } else {
            let bar = beat * per_bar as f64;
            let mut bars = 1;
            while points(bar * bars as f64) < MIN_LINE_SPACING {
                bars *= 2;
            }
            (bars * per_bar, bar * bars as f64)
        };
        let bar_secs = beat * per_bar as f64;
        let mut label_bars = 1;
        while points(bar_secs * label_bars as f64) < MIN_BAR_LABEL_SPACING {
            label_bars *= 2;
        }
        let beat_labels = points(beat) >= MIN_BEAT_LABEL_SPACING;

        let first = ((from.max(start) - start) / step_secs).ceil().max(0.0) as usize * step;
        let mut lines = Vec::new();
        let mut index = first;
        loop {
            let secs = start + index as f64 * beat;
            if secs > to {
                break;
            }
            let (bar, beat_in_bar) = (index / per_bar, index % per_bar);
            let label = if beat_in_bar == 0 {
                bar.is_multiple_of(label_bars)
                    .then(|| (bar + 1).to_string())
            } else if beat_labels {
                Some(format!("{}.{}", bar + 1, beat_in_bar + 1))
            } else {
                None
            };
            lines.push(Line {
                secs,
                bar: beat_in_bar == 0,
                label,
            });
            index += step;
        }
        lines
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grid(start: f64, beats_per_bar: u32) -> Grid {
        Grid {
            beats_per_bar,
            user_start: Some(start),
            ..Grid::default()
        }
    }

    #[test]
    fn beats_and_bars_from_bar_one() {
        // 120 BPM in 3/4 from 1 s: a beat every 0.5 s, a bar every 1.5 s.
        let lines = grid(1.0, 3).lines(120.0, 0.0, 3.0, 100.0);
        let secs: Vec<f64> = lines.iter().map(|l| l.secs).collect();
        assert_eq!(secs, vec![1.0, 1.5, 2.0, 2.5, 3.0]);
        let bars: Vec<bool> = lines.iter().map(|l| l.bar).collect();
        assert_eq!(bars, vec![true, false, false, true, false]);
        let labels: Vec<Option<&str>> = lines.iter().map(|l| l.label.as_deref()).collect();
        assert_eq!(
            labels,
            vec![Some("1"), Some("1.2"), Some("1.3"), Some("2"), Some("2.2")]
        );
    }

    #[test]
    fn nothing_before_bar_one() {
        let lines = grid(10.0, 4).lines(120.0, 0.0, 9.9, 100.0);
        assert!(lines.is_empty());
        // A view starting mid-song begins at the next beat.
        let lines = grid(10.0, 4).lines(120.0, 12.2, 13.0, 100.0);
        assert_eq!(lines[0].secs, 12.5);
        assert_eq!(lines[0].label.as_deref(), Some("2.2"));
    }

    #[test]
    fn zoomed_out_lines_thin_out() {
        // 2 points per second: a beat is 1 point, a bar 4, four bars 16.
        let lines = grid(0.0, 4).lines(120.0, 0.0, 60.0, 2.0);
        assert!(lines.iter().all(|l| l.bar));
        assert_eq!(lines[1].secs - lines[0].secs, 8.0);
        // Labels every 8 bars (32 points).
        let labeled: Vec<&str> = lines.iter().filter_map(|l| l.label.as_deref()).collect();
        assert_eq!(labeled, vec!["1", "9", "17", "25"]);
    }

    #[test]
    fn estimate_follows_the_bpm_and_the_user_wins() {
        let mut grid = Grid::default();
        let onsets = OnsetEnvelope::default();
        grid.update_estimate(Some(120.0), &onsets);
        assert_eq!(grid.start(), None, "no audio, no estimate");
        grid.set_start(2.5);
        assert_eq!(grid.start(), Some(2.5));
        grid.set_start(-1.0);
        assert_eq!(grid.start(), Some(0.0));
        grid.reset_start();
        assert_eq!(grid.user_start(), None);
    }
}
