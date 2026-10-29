//! The beat grid: bars and beats from the song's BPM, a time signature and where
//! bar 1 starts, with time signature changes later on, like a DAW's time signature
//! markers. Steady, or dynamic: following the song's own beats, tracked from the
//! audio, for songs whose tempo moves. Nothing is drawn before bar 1, so a free intro
//! stays clear. Times are in seconds of the song.

use watershed_audio::{Beats, Meter};
use watershed_dsp::{Anchor, OnsetEnvelope};
use watershed_storage::{Session, SignatureChange};

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
/// A signature change closer than this to another one is not added: the existing
/// one is reused, as with markers. Changes this close after bar 1 are left out.
const MIN_CHANGE_GAP: f64 = 0.1;
/// On a steady grid, a beat closer than this fraction of a beat to the next change
/// is left out: the bar before it ends on a longer beat instead of a sliver.
const MIN_LAST_BEAT: f64 = 0.25;

#[derive(Debug)]
pub struct Grid {
    /// Off hides the grid and greys out the BPM, for songs without a steady beat.
    pub enabled: bool,
    /// The time signature from bar 1. Its beat unit is the one the BPM counts.
    pub beats_per_bar: u32,
    pub beat_unit: u32,
    /// Where the time signature changes after bar 1, in time order.
    changes: Vec<SignatureChange>,
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
    /// The bars they were counted from: the user's bar 1 and the signature
    /// changes, when dynamic.
    anchors: Vec<Anchor>,
    /// Seconds of every beat, from the start of the song.
    beats: Vec<f64>,
    /// The local tempo's 5th and 95th percentile, in BPM, when they spread more
    /// than [`MOVING_SPREAD`].
    moving: Option<(f64, f64)>,
}

impl Tracked {
    fn new(bpm: f64, anchors: Vec<Anchor>, beats: Vec<f64>) -> Self {
        let mut tempos: Vec<f64> = beats.windows(2).map(|w| 60.0 / (w[1] - w[0])).collect();
        tempos.sort_by(f64::total_cmp);
        let at = |q: f64| tempos[((tempos.len() - 1) as f64 * q).round() as usize];
        let moving = (!tempos.is_empty())
            .then(|| (at(0.05), at(0.5), at(0.95)))
            .filter(|(low, median, high)| (high - low) / median > MOVING_SPREAD)
            .map(|(low, _, high)| (low, high));
        Self {
            bpm,
            anchors,
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
            changes: Vec::new(),
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
        let mut changes: Vec<SignatureChange> = session
            .signatures
            .iter()
            .filter(|c| c.time.is_finite() && c.time >= 0.0)
            .map(|c| SignatureChange {
                beats_per_bar: c.beats_per_bar.clamp(1, MAX_BEATS_PER_BAR),
                beat_unit: valid_unit(c.beat_unit),
                ..*c
            })
            .collect();
        changes.sort_by(|a, b| a.time.total_cmp(&b.time));
        Self {
            enabled: session.grid,
            beats_per_bar: session.beats_per_bar.clamp(1, MAX_BEATS_PER_BAR),
            beat_unit: valid_unit(session.beat_unit),
            changes,
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

    /// The time signature changes, in time order, to save in the session.
    pub fn changes(&self) -> &[SignatureChange] {
        &self.changes
    }

    pub fn change_mut(&mut self, index: usize) -> Option<&mut SignatureChange> {
        self.changes.get_mut(index)
    }

    /// Adds a signature change at `secs`, with the signature already in force
    /// there, unless one is already there. Returns the index of the change there.
    pub fn add_change(&mut self, secs: f64) -> usize {
        let secs = secs.max(0.0);
        if let Some(index) = self
            .changes
            .iter()
            .position(|c| (c.time - secs).abs() < MIN_CHANGE_GAP)
        {
            return index;
        }
        let index = self.changes.partition_point(|c| c.time < secs);
        let (beats_per_bar, beat_unit) = match index.checked_sub(1) {
            Some(before) => (
                self.changes[before].beats_per_bar,
                self.changes[before].beat_unit,
            ),
            None => (self.beats_per_bar, self.beat_unit),
        };
        self.changes.insert(
            index,
            SignatureChange {
                time: secs,
                beats_per_bar,
                beat_unit,
            },
        );
        index
    }

    /// Whether change `index` is after bar 1, so it changes anything.
    pub fn in_use(&self, index: usize) -> bool {
        let start = self.start().unwrap_or(0.0);
        self.changes
            .get(index)
            .is_some_and(|c| c.time >= start + MIN_CHANGE_GAP)
    }

    pub fn remove_change(&mut self, index: usize) {
        if index < self.changes.len() {
            self.changes.remove(index);
        }
    }

    /// Moves change `index` to `secs`, but not past its neighbours, so the order and
    /// the indices stay.
    pub fn move_change(&mut self, index: usize, secs: f64) {
        let low = index
            .checked_sub(1)
            .map_or(0.0, |before| self.changes[before].time + MIN_CHANGE_GAP);
        let high = self
            .changes
            .get(index + 1)
            .map_or(f64::INFINITY, |next| next.time - MIN_CHANGE_GAP);
        if let Some(change) = self.changes.get_mut(index)
            && low <= high
        {
            change.time = secs.clamp(low, high);
        }
    }

    /// The signature from bar 1 at `start`, then each change after it before `end`:
    /// the stretches of the grid, each to the next one.
    fn segments(&self, start: f64, end: f64) -> Vec<SignatureChange> {
        let bar_one = SignatureChange {
            time: start,
            beats_per_bar: self.beats_per_bar,
            beat_unit: self.beat_unit,
        };
        std::iter::once(bar_one)
            .chain(
                self.changes
                    .iter()
                    .filter(|c| c.time >= start + MIN_CHANGE_GAP && c.time < end)
                    .copied(),
            )
            .collect()
    }

    /// Beats of `beat_unit` per beat of the BPM: 2 for eighth notes in a song
    /// counted in quarter notes. The note value stays the same across changes.
    fn pace(&self, beat_unit: u32) -> f64 {
        f64::from(beat_unit) / f64::from(self.beat_unit)
    }

    /// The bars the tracker counts from when dynamic: the user's bar 1, and each
    /// signature change at its pace.
    fn anchors(&self) -> Vec<Anchor> {
        if !self.dynamic {
            return Vec::new();
        }
        let start = self.start().unwrap_or(0.0);
        let bar_one = self.user_start.map(|time| Anchor { time, pace: 1.0 });
        let changes = self.segments(start, f64::INFINITY).into_iter().skip(1);
        bar_one
            .into_iter()
            .chain(changes.map(|c| Anchor {
                time: c.time,
                pace: self.pace(c.beat_unit),
            }))
            .collect()
    }

    /// Estimates bar 1 again if the BPM changed since, and tracks the beats again if
    /// it changed by more than [`RETRACK`], or, when dynamic, if the user's bar 1 or
    /// the signature changes moved (not while `dragging`: tracking takes too long to
    /// do every frame).
    pub fn update_estimate(&mut self, bpm: Option<f64>, onsets: &OnsetEnvelope, dragging: bool) {
        let Some(bpm) = bpm else {
            self.estimate = None;
            self.tracked = None;
            return;
        };
        if self.estimate.is_none_or(|(for_bpm, _)| for_bpm != bpm) {
            self.estimate = onsets.first_beat(bpm).map(|start| (bpm, start));
        }
        let anchors = self.anchors();
        let retrack = self.tracked.as_ref().is_none_or(|tracked| {
            (bpm / tracked.bpm - 1.0).abs() > RETRACK || (!dragging && tracked.anchors != anchors)
        });
        if retrack {
            let beats = onsets.track_beats(bpm, &anchors);
            self.tracked = Some(Tracked::new(bpm, anchors, beats));
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
    /// the song's own when dynamic (bar 1 and each signature change on the beat
    /// nearest to it), else steady. `None` when off or bar 1 is unknown.
    pub fn beats(&self, bpm: f64, end: f64) -> Option<Beats> {
        if !self.enabled || bpm <= 0.0 {
            return None;
        }
        let start = self.start()?;
        let segments = self.segments(start, end);
        match &self.tracked {
            Some(tracked) if self.dynamic && tracked.beats.len() >= 2 => {
                let beats = &tracked.beats;
                let first = nearest(beats, start);
                let mut meters = vec![Meter {
                    from: 0,
                    per_bar: self.beats_per_bar,
                }];
                for segment in &segments[1..] {
                    let from = nearest(beats, segment.time).saturating_sub(first);
                    let per_bar = segment.beats_per_bar;
                    match meters.last_mut() {
                        // Two changes on one beat: the later one wins.
                        Some(last) if last.from >= from => last.per_bar = per_bar,
                        _ => meters.push(Meter { from, per_bar }),
                    }
                }
                Beats::with_meters(beats[first..].to_vec(), meters)
                    .or_else(|| Beats::new(beats[first - 1..].to_vec(), self.beats_per_bar))
            }
            _ => {
                let mut times = Vec::new();
                let mut meters = Vec::new();
                for (k, segment) in segments.iter().enumerate() {
                    let beat = 60.0 / bpm / self.pace(segment.beat_unit);
                    let count = match segments.get(k + 1) {
                        Some(next) => {
                            ((next.time - segment.time) / beat - MIN_LAST_BEAT).ceil() as usize
                        }
                        // Up to a beat past the end.
                        None => ((end - segment.time) / beat).ceil() as usize + 1,
                    };
                    meters.push(Meter {
                        from: times.len(),
                        per_bar: segment.beats_per_bar,
                    });
                    times.extend((0..count.max(1)).map(|i| segment.time + i as f64 * beat));
                }
                if times.len() < 2 {
                    times.push(times.first()? + 60.0 / bpm);
                }
                Beats::with_meters(times, meters)
            }
        }
    }
}

/// The index of the beat in `beats` (not empty, rising) nearest to `time`.
fn nearest(beats: &[f64], time: f64) -> usize {
    let after = beats.partition_point(|&t| t < time);
    match (after.checked_sub(1), beats.get(after)) {
        (Some(before), Some(&next)) if time - beats[before] < next - time => before,
        (Some(before), None) => before,
        _ => after,
    }
}

/// `unit` if it is one of [`BEAT_UNITS`], else 4.
fn valid_unit(unit: u32) -> u32 {
    if BEAT_UNITS.contains(&unit) { unit } else { 4 }
}

/// The lines of `beats` between `from` and `to` seconds, thinned out for
/// `points_per_sec` so they never crowd.
pub fn lines(beats: &Beats, from: f64, to: f64, points_per_sec: f64) -> Vec<Line> {
    if points_per_sec <= 0.0 || to <= from {
        return Vec::new();
    }
    let start = beats.time(0);
    let first = beats.first_from(from.max(start)).max(0);
    // Spacing from the beat at the left of the view, close enough for a tempo that
    // moves a few percent or a change of time signature.
    let beat = beats.beat_at(from.max(start));
    let points = |secs: f64| (secs * points_per_sec) as f32;
    let bar_secs = beat * f64::from(beats.per_bar(first));
    // Every beat, or every n-th bar with n a power of two.
    let every_beat = points(beat) >= MIN_LINE_SPACING;
    let mut step_bars = 1;
    while !every_beat && points(bar_secs * step_bars as f64) < MIN_LINE_SPACING {
        step_bars *= 2;
    }
    let mut label_bars = 1;
    while points(bar_secs * label_bars as f64) < MIN_BAR_LABEL_SPACING {
        label_bars *= 2;
    }
    let beat_labels = points(beat) >= MIN_BEAT_LABEL_SPACING;

    let mut lines = Vec::new();
    for index in first.. {
        let secs = beats.time(index);
        if secs > to {
            break;
        }
        let (bar, beat_in_bar) = beats.bar_of(index);
        if !every_beat && (beat_in_bar != 0 || bar % step_bars != 0) {
            continue;
        }
        let label = if beat_in_bar == 0 {
            (bar % label_bars == 0).then(|| (bar + 1).to_string())
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
        grid.tracked = Some(Tracked::new(120.0, Vec::new(), tracked));
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
        assert_eq!(Tracked::new(120.0, Vec::new(), steady).moving, None);
        // From 120 to 132 BPM.
        let mut beats = vec![0.0];
        for k in 0..100 {
            let bpm = 120.0 + 12.0 * k as f64 / 100.0;
            beats.push(beats[k] + 60.0 / bpm);
        }
        let (low, high) = Tracked::new(120.0, Vec::new(), beats).moving.unwrap();
        assert!(
            (low - 120.6).abs() < 0.2 && (high - 131.4).abs() < 0.2,
            "{low} {high}"
        );
        assert_eq!(Tracked::new(120.0, Vec::new(), Vec::new()).moving, None);
    }

    #[test]
    fn dynamic_beats_count_from_the_users_bars_once_they_settle() {
        let onsets = OnsetEnvelope::default();
        let tracked_from = |grid: &Grid| {
            let anchors = &grid.tracked.as_ref().unwrap().anchors;
            anchors.iter().map(|a| (a.time, a.pace)).collect::<Vec<_>>()
        };
        let mut grid = grid(1.0, 4);
        grid.update_estimate(Some(120.0), &onsets, false);
        assert_eq!(tracked_from(&grid), [], "steady: tracked freely");
        grid.dynamic = true;
        grid.update_estimate(Some(120.0), &onsets, false);
        assert_eq!(tracked_from(&grid), [(1.0, 1.0)]);
        grid.set_start(1.5);
        grid.update_estimate(Some(120.0), &onsets, true);
        assert_eq!(tracked_from(&grid), [(1.0, 1.0)], "not while dragging");
        grid.update_estimate(Some(120.0), &onsets, false);
        assert_eq!(tracked_from(&grid), [(1.5, 1.0)]);
        // A change to 7/8 counts eighth notes from there.
        let index = grid.add_change(20.0);
        grid.change_mut(index).unwrap().beats_per_bar = 7;
        grid.change_mut(index).unwrap().beat_unit = 8;
        grid.update_estimate(Some(120.0), &onsets, false);
        assert_eq!(tracked_from(&grid), [(1.5, 1.0), (20.0, 2.0)]);
    }

    #[test]
    fn signature_changes_regroup_the_bars() {
        // 120 BPM in 4/4 from 0 s; one bar of 2/4 at 8 s, then 7/8 from 9 s.
        let mut grid = grid(0.0, 4);
        let short = grid.add_change(8.0);
        grid.change_mut(short).unwrap().beats_per_bar = 2;
        let seven = grid.add_change(9.0);
        assert_eq!(
            (grid.changes()[1].beats_per_bar, grid.changes()[1].beat_unit),
            (2, 4),
            "starts with the signature in force"
        );
        let change = grid.change_mut(seven).unwrap();
        (change.beats_per_bar, change.beat_unit) = (7, 8);
        let beats = grid.beats(120.0, 14.0).unwrap();
        let found: Vec<(f64, Option<String>)> = lines(&beats, 7.0, 12.6, 100.0)
            .into_iter()
            .filter(|l| l.bar)
            .map(|l| (l.secs, l.label))
            .collect();
        // Bars 5 (2/4) and 6 (7/8, eighths at 0.25 s: 1.75 s a bar).
        let expected = [(8.0, "5"), (9.0, "6"), (10.75, "7"), (12.5, "8")];
        assert_eq!(found, expected.map(|(s, l)| (s, Some(l.to_string()))));
        assert_eq!(beats.time(beats.first_from(9.0) + 1), 9.25);
        assert_eq!(grid.add_change(9.05), seven, "no change on top of another");
    }

    #[test]
    fn a_change_off_the_grid_cuts_the_bar_before_it() {
        // 120 BPM in 4/4; a change to 3/4 at 3.3 s, 0.3 s into bar 2's beat 3.
        let mut grid = grid(0.0, 4);
        let index = grid.add_change(3.3);
        grid.change_mut(index).unwrap().beats_per_bar = 3;
        let beats = grid.beats(120.0, 6.0).unwrap();
        let bars: Vec<f64> = lines(&beats, 0.0, 6.0, 100.0)
            .iter()
            .filter(|l| l.bar)
            .map(|l| l.secs)
            .collect();
        assert_eq!(bars, [0.0, 2.0, 3.3, 4.8]);
        // A sliver of a beat is left out: 3.05 s is only 0.05 s past 3.0.
        grid.move_change(index, 3.05);
        let beats = grid.beats(120.0, 6.0).unwrap();
        let near: Vec<f64> = beats.within(2.4..3.6).map(|(_, t)| t).collect();
        assert_eq!(near, [2.5, 3.05, 3.55]);
    }

    #[test]
    fn changes_move_between_their_neighbours() {
        let mut grid = Grid::default();
        grid.add_change(10.0);
        grid.add_change(20.0);
        grid.move_change(0, 25.0);
        assert_eq!(grid.changes()[0].time, 20.0 - MIN_CHANGE_GAP);
        grid.move_change(1, -5.0);
        assert_eq!(
            grid.changes()[1].time,
            grid.changes()[0].time + MIN_CHANGE_GAP
        );
        grid.remove_change(0);
        assert_eq!(grid.changes().len(), 1);
    }

    #[test]
    fn dynamic_bars_follow_the_changes() {
        // Tracked beats a quarter note apart, then eighths from 4.0 s.
        let mut tracked: Vec<f64> = (0..8).map(|k| k as f64 * 0.5).collect();
        tracked.extend((0..14).map(|k| 4.0 + k as f64 * 0.25));
        let mut grid = grid(0.0, 4);
        grid.dynamic = true;
        grid.tracked = Some(Tracked::new(120.0, Vec::new(), tracked));
        let index = grid.add_change(4.02);
        let change = grid.change_mut(index).unwrap();
        (change.beats_per_bar, change.beat_unit) = (7, 8);
        let beats = grid.beats(120.0, 8.0).unwrap();
        let bars: Vec<f64> = lines(&beats, 0.0, 8.0, 100.0)
            .iter()
            .filter(|l| l.bar)
            .map(|l| l.secs)
            .collect();
        // Past the tracked beats, at their last pace.
        assert_eq!(bars, [0.0, 2.0, 4.0, 5.75, 7.5]);
    }
}
