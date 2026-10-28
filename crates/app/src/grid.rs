//! The beat grid: bars and beats from the song's BPM, a time signature and where
//! bar 1 starts. Steady, or dynamic: following the song's own beats, tracked from the
//! audio, for songs whose tempo moves. Nothing is drawn before bar 1, so a free intro
//! stays clear. Times are in seconds of the song.

use watershed_audio::Beats;
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
/// The tempo moves when the local tempo spreads over more than this fraction of
/// its median, from the 5th to the 95th percentile. Steady songs measured 1–2 %
/// (a band playing to no click included), Deftones' "Change" 6 %.
const MOVING_SPREAD: f64 = 0.03;
/// The beats are tracked again when the BPM changes by more than this fraction.
/// The tracker finds the exact pace itself (within 10 %), so dragging the BPM does
/// not need it, and it takes 20–40 ms.
const RETRACK: f64 = 0.05;

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
    /// Follow the song's own beats instead of a steady BPM.
    pub dynamic: bool,
    /// The beats tracked in the audio.
    tracked: Option<Tracked>,
}

/// The song's own beats and how much its tempo moves.
#[derive(Debug)]
struct Tracked {
    /// The BPM they were tracked around.
    bpm: f64,
    /// The user's bar 1 they were counted from, if any.
    bar_one: Option<f64>,
    /// Seconds of every beat, from the start of the song.
    beats: Vec<f64>,
    /// The local tempo's 5th and 95th percentile, in BPM, when they spread more
    /// than [`MOVING_SPREAD`].
    moving: Option<(f64, f64)>,
}

impl Tracked {
    fn new(bpm: f64, bar_one: Option<f64>, beats: Vec<f64>) -> Self {
        let mut tempos: Vec<f64> = beats.windows(2).map(|w| 60.0 / (w[1] - w[0])).collect();
        tempos.sort_by(f64::total_cmp);
        let at = |q: f64| tempos[((tempos.len() - 1) as f64 * q).round() as usize];
        let moving = (!tempos.is_empty())
            .then(|| (at(0.05), at(0.5), at(0.95)))
            .filter(|(low, median, high)| (high - low) / median > MOVING_SPREAD)
            .map(|(low, _, high)| (low, high));
        Self {
            bpm,
            bar_one,
            beats,
            moving,
        }
    }
}

impl Default for Grid {
    fn default() -> Self {
        Self {
            enabled: true,
            beats_per_bar: 4,
            beat_unit: 4,
            user_start: None,
            estimate: None,
            dynamic: false,
            tracked: None,
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
            dynamic: session.dynamic_tempo,
            tracked: None,
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

    /// Estimates bar 1 again if the BPM changed since, and tracks the beats again if
    /// it changed by more than [`RETRACK`], or, when dynamic, from the user's bar 1
    /// if it moved (not while `dragging`: tracking takes too long to do every frame).
    pub fn update_estimate(&mut self, bpm: Option<f64>, onsets: &OnsetEnvelope, dragging: bool) {
        let Some(bpm) = bpm else {
            self.estimate = None;
            self.tracked = None;
            return;
        };
        if self.estimate.is_none_or(|(for_bpm, _)| for_bpm != bpm) {
            self.estimate = onsets.first_beat(bpm).map(|start| (bpm, start));
        }
        let bar_one = if self.dynamic { self.user_start } else { None };
        let retrack = self.tracked.as_ref().is_none_or(|tracked| {
            (bpm / tracked.bpm - 1.0).abs() > RETRACK
                || (self.dynamic && !dragging && tracked.bar_one != bar_one)
        });
        if retrack {
            let beats = onsets.track_beats(bpm, bar_one);
            self.tracked = Some(Tracked::new(bpm, bar_one, beats));
        }
    }

    /// The slowest and fastest local tempo (5th and 95th percentile, in BPM at the
    /// original speed) when the song's tempo moves.
    pub fn tempo_range(&self) -> Option<(f64, f64)> {
        self.tracked.as_ref()?.moving
    }

    /// Whether the song's own beats are known, so the grid can follow them.
    pub fn can_follow(&self) -> bool {
        self.tracked
            .as_ref()
            .is_some_and(|tracked| tracked.beats.len() >= 2)
    }

    /// The beats of the grid at `bpm` in seconds, from bar 1, listed up to `end`:
    /// the song's own when dynamic (bar 1 on the beat nearest to its start), else
    /// steady. `None` when off or bar 1 is unknown.
    pub fn beats(&self, bpm: f64, end: f64) -> Option<Beats> {
        if !self.enabled {
            return None;
        }
        let start = self.start()?;
        match &self.tracked {
            Some(tracked) if self.dynamic && tracked.beats.len() >= 2 => {
                let beats = &tracked.beats;
                let after = beats.partition_point(|&t| t < start);
                let nearest = match (after.checked_sub(1), beats.get(after)) {
                    (Some(before), Some(&next)) if start - beats[before] < next - start => before,
                    (Some(before), None) => before,
                    _ => after,
                };
                Beats::new(beats[nearest..].to_vec(), self.beats_per_bar)
                    .or_else(|| Beats::new(beats[nearest - 1..].to_vec(), self.beats_per_bar))
            }
            _ => Beats::steady(start, 60.0 / bpm, self.beats_per_bar, end),
        }
    }
}

/// The lines of `beats` between `from` and `to` seconds, thinned out for
/// `points_per_sec` so they never crowd.
pub fn lines(beats: &Beats, from: f64, to: f64, points_per_sec: f64) -> Vec<Line> {
    if points_per_sec <= 0.0 || to <= from {
        return Vec::new();
    }
    let start = beats.time(0);
    // Spacing from the beat at the left of the view, close enough for a tempo that
    // moves a few percent.
    let beat = beats.beat_at(from.max(start));
    let per_bar = beats.per_bar() as usize;
    let points = |secs: f64| (secs * points_per_sec) as f32;
    let bar_secs = beat * per_bar as f64;
    // Every beat, or every n-th bar with n a power of two.
    let step = if points(beat) >= MIN_LINE_SPACING {
        1
    } else {
        let mut bars = 1;
        while points(bar_secs * bars as f64) < MIN_LINE_SPACING {
            bars *= 2;
        }
        bars * per_bar
    };
    let mut label_bars = 1;
    while points(bar_secs * label_bars as f64) < MIN_BAR_LABEL_SPACING {
        label_bars *= 2;
    }
    let beat_labels = points(beat) >= MIN_BEAT_LABEL_SPACING;

    let first = (beats.first_from(from.max(start)).max(0) as usize).div_ceil(step) * step;
    let mut lines = Vec::new();
    let mut index = first;
    loop {
        let secs = beats.time(index as i64);
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

    /// The lines of `grid`'s steady beats at `bpm`.
    fn steady_lines(grid: Grid, bpm: f64, from: f64, to: f64, points_per_sec: f64) -> Vec<Line> {
        lines(&grid.beats(bpm, to).unwrap(), from, to, points_per_sec)
    }

    #[test]
    fn beats_and_bars_from_bar_one() {
        // 120 BPM in 3/4 from 1 s: a beat every 0.5 s, a bar every 1.5 s.
        let lines = steady_lines(grid(1.0, 3), 120.0, 0.0, 3.0, 100.0);
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
        let lines = steady_lines(grid(10.0, 4), 120.0, 0.0, 9.9, 100.0);
        assert!(lines.is_empty());
        // A view starting mid-song begins at the next beat.
        let lines = steady_lines(grid(10.0, 4), 120.0, 12.2, 13.0, 100.0);
        assert_eq!(lines[0].secs, 12.5);
        assert_eq!(lines[0].label.as_deref(), Some("2.2"));
    }

    #[test]
    fn zoomed_out_lines_thin_out() {
        // 2 points per second: a beat is 1 point, a bar 4, four bars 16.
        let lines = steady_lines(grid(0.0, 4), 120.0, 0.0, 60.0, 2.0);
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
        grid.update_estimate(Some(120.0), &onsets, false);
        assert_eq!(grid.start(), None, "no audio, no estimate");
        grid.set_start(2.5);
        assert_eq!(grid.start(), Some(2.5));
        grid.set_start(-1.0);
        assert_eq!(grid.start(), Some(0.0));
        grid.reset_start();
        assert_eq!(grid.user_start(), None);
    }

    #[test]
    fn dynamic_grid_follows_the_tracked_beats() {
        // Speeding up from 120 BPM; bar 1 set a little after the third beat.
        let tracked = vec![0.0, 0.5, 0.98, 1.44, 1.88, 2.3];
        let mut grid = grid(1.0, 2);
        grid.tracked = Some(Tracked::new(120.0, None, tracked));
        let steady = grid.beats(120.0, 3.0).unwrap();
        assert_eq!(steady.time(1), 1.5);
        grid.dynamic = true;
        let beats = grid.beats(120.0, 3.0).unwrap();
        assert_eq!((beats.time(0), beats.time(1)), (0.98, 1.44));
        let found: Vec<(f64, bool)> = lines(&beats, 0.0, 2.0, 100.0)
            .iter()
            .map(|l| (l.secs, l.bar))
            .collect();
        assert_eq!(found, vec![(0.98, true), (1.44, false), (1.88, true)]);
        // Off: no beats at all.
        grid.enabled = false;
        assert_eq!(grid.beats(120.0, 3.0), None);
    }

    #[test]
    fn tells_when_the_tempo_moves() {
        let steady: Vec<f64> = (0..100).map(|k| k as f64 * 0.5).collect();
        assert_eq!(Tracked::new(120.0, None, steady).moving, None);
        // From 120 to 132 BPM.
        let mut beats = vec![0.0];
        for k in 0..100 {
            let bpm = 120.0 + 12.0 * k as f64 / 100.0;
            beats.push(beats[k] + 60.0 / bpm);
        }
        let (low, high) = Tracked::new(120.0, None, beats).moving.unwrap();
        assert!(
            (low - 120.6).abs() < 0.2 && (high - 131.4).abs() < 0.2,
            "{low} {high}"
        );
        assert_eq!(Tracked::new(120.0, None, Vec::new()).moving, None);
    }

    #[test]
    fn dynamic_beats_count_from_the_users_bar_one_once_it_settles() {
        let onsets = OnsetEnvelope::default();
        let tracked_from = |grid: &Grid| grid.tracked.as_ref().map(|t| t.bar_one);
        let mut grid = grid(1.0, 4);
        grid.update_estimate(Some(120.0), &onsets, false);
        assert_eq!(tracked_from(&grid), Some(None), "steady: tracked freely");
        grid.dynamic = true;
        grid.update_estimate(Some(120.0), &onsets, false);
        assert_eq!(tracked_from(&grid), Some(Some(1.0)));
        grid.set_start(1.5);
        grid.update_estimate(Some(120.0), &onsets, true);
        assert_eq!(tracked_from(&grid), Some(Some(1.0)), "not while dragging");
        grid.update_estimate(Some(120.0), &onsets, false);
        assert_eq!(tracked_from(&grid), Some(Some(1.5)));
    }
}
