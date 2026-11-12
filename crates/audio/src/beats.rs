//! The song's beats: where each one falls, from bar 1 on, and how they group into
//! bars. A steady grid is one beat every so often; a tempo map follows the beats the
//! song really has.

/// Beat times from bar 1 on, in any unit (seconds in the UI, frames in the engine).
/// Beat 0 is bar 1. Outside the list the beats go on at the pace of its first and
/// last beat, so there are beats before bar 1 (for a count-in) and after the end.
#[derive(Clone, Debug, PartialEq)]
pub struct Beats {
    /// At least two, rising.
    times: Vec<f64>,
    /// The first starts at beat 0; the others at rising beats.
    meters: Vec<Meter>,
}

/// One bar of evenly spaced count-in clicks, the first accented, that need not be on
/// the grid.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LeadBar {
    /// The first click.
    pub start: f64,
    pub beat: f64,
    pub per_bar: u32,
}

impl LeadBar {
    /// The clicks: time and accent.
    pub fn clicks(self) -> impl Iterator<Item = (f64, bool)> {
        (0..self.per_bar).map(move |k| (self.start + f64::from(k) * self.beat, k == 0))
    }
}

/// What a count-in clicks: maybe a [`LeadBar`], then the grid's beats from `grid.0` up
/// to before `grid.1` (none when they are equal), the last ones over the song.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CountInClicks {
    pub lead: Option<LeadBar>,
    pub grid: (f64, f64),
}

impl CountInClicks {
    /// Where the first click falls.
    pub fn start(&self) -> f64 {
        self.lead.map_or(self.grid.0, |lead| lead.start)
    }
}

/// Bars of `per_bar` beats from beat `from` on, up to the next meter: where the
/// time signature changes. A bar cut short by the next meter is a bar too.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Meter {
    pub from: usize,
    pub per_bar: u32,
}

impl Beats {
    /// Beats at `times`, `per_bar` to a bar, or `None` with fewer than two, or times
    /// that are not finite and rising.
    pub fn new(times: Vec<f64>, per_bar: u32) -> Option<Self> {
        Self::with_meters(times, vec![Meter { from: 0, per_bar }])
    }

    /// Beats at `times` grouped by `meters`, or `None` as [`Beats::new`], or when
    /// the meters do not start at beat 0 and rise.
    pub fn with_meters(times: Vec<f64>, mut meters: Vec<Meter>) -> Option<Self> {
        let valid = times.len() >= 2
            && times.iter().all(|t| t.is_finite())
            && times.windows(2).all(|w| w[1] > w[0])
            && meters.first().is_some_and(|m| m.from == 0)
            && meters.windows(2).all(|w| w[1].from > w[0].from);
        for meter in &mut meters {
            meter.per_bar = meter.per_bar.max(1);
        }
        valid.then_some(Self { times, meters })
    }

    /// A beat every `beat` from `start` until past `end`.
    pub fn steady(start: f64, beat: f64, per_bar: u32, end: f64) -> Option<Self> {
        if !(beat > 0.0 && start.is_finite() && end.is_finite()) {
            return None;
        }
        let count = ((end - start) / beat).ceil().max(1.0) as usize + 1;
        Self::new(
            (0..count).map(|k| start + k as f64 * beat).collect(),
            per_bar,
        )
    }

    /// The listed beat times, from bar 1.
    pub fn times(&self) -> &[f64] {
        &self.times
    }

    /// The meter beat `index` is in (the first one before bar 1).
    fn meter(&self, index: i64) -> Meter {
        let after = self
            .meters
            .partition_point(|m| m.from as i64 <= index)
            .max(1);
        self.meters[after - 1]
    }

    /// Beats per bar where beat `index` is.
    pub fn per_bar(&self, index: i64) -> u32 {
        self.meter(index).per_bar
    }

    /// The bar beat `index` is in (0 is bar 1, negative before it) and its beat in
    /// that bar (0 is the first).
    pub fn bar_of(&self, index: i64) -> (i64, u32) {
        let meter = self.meter(index);
        let mut bar = 0;
        for w in self
            .meters
            .windows(2)
            .take_while(|w| w[1].from <= meter.from)
        {
            bar += (w[1].from - w[0].from).div_ceil(w[0].per_bar as usize) as i64;
        }
        let per_bar = i64::from(meter.per_bar);
        let beats = index - meter.from as i64;
        (
            bar + beats.div_euclid(per_bar),
            beats.rem_euclid(per_bar) as u32,
        )
    }

    /// The time of beat `index` (0 is bar 1, negative before it).
    pub fn time(&self, index: i64) -> f64 {
        let last = self.times.len() - 1;
        if index < 0 {
            self.times[0] + index as f64 * (self.times[1] - self.times[0])
        } else if index as usize > last {
            let beat = self.times[last] - self.times[last - 1];
            self.times[last] + (index as usize - last) as f64 * beat
        } else {
            self.times[index as usize]
        }
    }

    /// The index of the first beat at or after `time`.
    pub fn first_from(&self, time: f64) -> i64 {
        let last = self.times.len() - 1;
        if time <= self.times[0] {
            let beat = self.times[1] - self.times[0];
            ((time - self.times[0]) / beat).ceil() as i64
        } else if time > self.times[last] {
            let beat = self.times[last] - self.times[last - 1];
            last as i64 + ((time - self.times[last]) / beat).ceil() as i64
        } else {
            self.times.partition_point(|&t| t < time) as i64
        }
    }

    /// The beats from `range.start` up to before `range.end`: index and time.
    pub fn within(&self, range: std::ops::Range<f64>) -> impl Iterator<Item = (i64, f64)> + '_ {
        (self.first_from(range.start)..)
            .map(|index| (index, self.time(index)))
            .take_while(move |(_, time)| *time < range.end)
    }

    /// Whether beat `index` starts a bar.
    pub fn accented(&self, index: i64) -> bool {
        let meter = self.meter(index);
        (index - meter.from as i64).rem_euclid(i64::from(meter.per_bar)) == 0
    }

    /// The index of the last beat at or before `time`.
    fn beat_from(&self, time: f64) -> i64 {
        let index = self.first_from(time);
        if self.time(index) > time {
            index - 1
        } else {
            index
        }
    }

    /// The index of the bar line (a beat that starts a bar) at or before `time`:
    /// where the bar `time` is in starts.
    fn bar_line_at(&self, time: f64) -> i64 {
        let mut index = self.beat_from(time);
        while !self.accented(index) {
            index -= 1;
        }
        index
    }

    /// The index of the bar line after the one at `index`.
    fn next_bar_line(&self, index: i64) -> i64 {
        let mut next = index + 1;
        while !self.accented(next) {
            next += 1;
        }
        next
    }

    /// A count-in that lands on a bar line, for playback starting at `time`, in the
    /// meter and tempo of the bar `time` is in (the bar about to play). From a bar line
    /// it counts one bar shaped like the one starting there. From the second half of a
    /// bar (by time) it counts that bar's own beats from its start, and the song comes
    /// in at `time` over the last ones, up to the next bar line, which clicks as the
    /// "1". From the first half it counts one bar more before that. `tolerance` treats
    /// a `time` that close to a bar line as on it.
    pub fn count_in_to_bar(&self, time: f64, tolerance: f64) -> CountInClicks {
        let line = self.bar_line_at(time + tolerance);
        let at = self.time(line);
        let per_bar = self.per_bar(line);
        let beat = self.time(line + 1) - at;
        let lead = LeadBar {
            start: at - f64::from(per_bar) * beat,
            beat,
            per_bar,
        };
        if time - at <= tolerance {
            return CountInClicks {
                lead: Some(lead),
                grid: (at, at),
            };
        }
        let next = self.time(self.next_bar_line(line));
        let late = (time - at) / (next - at) >= 0.5;
        CountInClicks {
            lead: (!late).then_some(lead),
            grid: (at, next + tolerance),
        }
    }

    /// One whole bar of clicks ending at `time`, in the meter of the bar `time` is in
    /// and the tempo of its beat, wherever `time` falls in that bar. `tolerance` as in
    /// [`Beats::count_in_to_bar`].
    pub fn count_in_bar(&self, time: f64, tolerance: f64) -> LeadBar {
        let per_bar = self.per_bar(self.bar_line_at(time + tolerance));
        let index = self.beat_from(time + tolerance);
        let beat = self.time(index + 1) - self.time(index);
        LeadBar {
            start: time - f64::from(per_bar) * beat,
            beat,
            per_bar,
        }
    }

    /// The length of the beat that `time` falls in.
    pub fn beat_at(&self, time: f64) -> f64 {
        let index = self.first_from(time).max(1);
        self.time(index) - self.time(index - 1)
    }

    /// The same beats with every time multiplied by `factor`, e.g. seconds to frames.
    pub fn scaled(&self, factor: f64) -> Option<Self> {
        Self::with_meters(
            self.times.iter().map(|t| t * factor).collect(),
            self.meters.clone(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn steady_beats_go_on_both_ways() {
        let beats = Beats::steady(100.0, 50.0, 3, 200.0).unwrap();
        let found: Vec<(i64, f64)> = beats.within(0.0..360.0).collect();
        let expected = [-2, -1, 0, 1, 2, 3, 4, 5].map(|i| (i, 100.0 + i as f64 * 50.0));
        assert_eq!(found, expected);
        assert!(beats.accented(0) && beats.accented(3) && beats.accented(-3));
        assert!(!beats.accented(-1));
        assert_eq!(Beats::steady(0.0, 0.0, 4, 10.0), None);
    }

    #[test]
    fn a_tempo_map_keeps_its_own_beats() {
        // Speeding up: beats 10, 8 and 6 apart.
        let beats = Beats::new(vec![0.0, 10.0, 18.0, 24.0], 2).unwrap();
        assert_eq!(beats.first_from(10.0), 1);
        assert_eq!(beats.first_from(10.5), 2);
        assert_eq!(beats.first_from(-15.0), -1);
        assert_eq!(beats.first_from(25.0), 4);
        assert_eq!(beats.time(-2), -20.0);
        assert_eq!(beats.time(5), 36.0);
        let found: Vec<f64> = beats.within(-10.0..30.0).map(|(_, t)| t).collect();
        assert_eq!(found, vec![-10.0, 0.0, 10.0, 18.0, 24.0]);
        assert_eq!(beats.beat_at(20.0), 6.0);
        assert_eq!(beats.beat_at(-5.0), 10.0);
        assert_eq!(beats.scaled(2.0).unwrap().time(3), 48.0);
    }

    #[test]
    fn bars_change_with_the_meter() {
        // Bar 1 in 4/4, a bar cut short to 2 beats, then 3 to a bar from beat 6.
        let meters = vec![
            Meter {
                from: 0,
                per_bar: 4,
            },
            Meter {
                from: 6,
                per_bar: 3,
            },
        ];
        let beats = Beats::with_meters((0..12).map(f64::from).collect(), meters).unwrap();
        let accented: Vec<i64> = (-4..12).filter(|&i| beats.accented(i)).collect();
        assert_eq!(accented, [-4, 0, 4, 6, 9]);
        let bars: Vec<(i64, u32)> = [-1, 0, 5, 6, 8, 9, 20].map(|i| beats.bar_of(i)).into();
        assert_eq!(
            bars,
            [(-1, 3), (0, 0), (1, 1), (2, 0), (2, 2), (3, 0), (6, 2)]
        );
        assert_eq!(beats.scaled(2.0).unwrap().bar_of(9), (3, 0));
        let unordered = vec![
            Meter {
                from: 0,
                per_bar: 4,
            },
            Meter {
                from: 0,
                per_bar: 3,
            },
        ];
        assert_eq!(Beats::with_meters(vec![0.0, 1.0], unordered), None);
    }

    /// The first click and the grid range of a count-in, as start, grid start and
    /// grid end, and the beats per bar of its lead bar (0 without one).
    fn plan(beats: &Beats, time: f64, tolerance: f64) -> (f64, f64, f64, u32) {
        let clicks = beats.count_in_to_bar(time, tolerance);
        let per_bar = clicks.lead.map_or(0, |lead| lead.per_bar);
        (clicks.start(), clicks.grid.0, clicks.grid.1, per_bar)
    }

    #[test]
    fn count_in_lands_on_a_bar_line() {
        // 4/4, a beat every 10 from bar 1 at 100.
        let beats = Beats::steady(100.0, 10.0, 4, 300.0).unwrap();
        // On a bar line (or within the tolerance): one bar before it, no grid beats.
        assert_eq!(plan(&beats, 140.0, 0.1), (100.0, 140.0, 140.0, 4));
        assert_eq!(plan(&beats, 140.05, 0.1), (100.0, 140.0, 140.0, 4));
        assert_eq!(plan(&beats, 139.95, 0.1), (100.0, 140.0, 140.0, 4));
        // Beat 4: "1 2 3", the song on the 4, then the 1 at 180.
        assert_eq!(plan(&beats, 170.0, 0.1), (140.0, 140.0, 180.1, 0));
        // Halfway counts as late.
        assert_eq!(plan(&beats, 160.0, 0.1), (140.0, 140.0, 180.1, 0));
        // Beat 2: "1 2 3 4 | 1", the song on the 2.
        assert_eq!(plan(&beats, 150.0, 0.1), (100.0, 140.0, 180.1, 4));
        // Before bar 1, at the grid's pace: a pickup ending at bar 1.
        assert_eq!(plan(&beats, 85.0, 0.1), (60.0, 60.0, 100.1, 0));
        assert_eq!(plan(&beats, 65.0, 0.1), (20.0, 60.0, 100.1, 4));
        let lead: Vec<(f64, bool)> = beats.count_in_bar(65.0, 0.1).clicks().collect();
        assert_eq!(
            lead,
            [(25.0, true), (35.0, false), (45.0, false), (55.0, false)]
        );

        // As in "46 & 2": 4/4, then 7/8 from beat 4, back to 4/4 at beat 11.
        let meters = [(0, 4), (4, 7), (11, 4)]
            .map(|(from, per_bar)| Meter { from, per_bar })
            .to_vec();
        let beats = Beats::with_meters((0..30).map(f64::from).collect(), meters).unwrap();
        // On the bar line where the 7/8 starts: a bar of 7, not the 4/4 before it.
        assert_eq!(plan(&beats, 4.0, 0.01), (-3.0, 4.0, 4.0, 7));
        assert_eq!(beats.count_in_bar(4.0, 0.01).per_bar, 7);
        // By time, so the 7/8 bar is late from halfway through its 4th beat.
        assert_eq!(plan(&beats, 7.5, 0.01), (4.0, 4.0, 11.01, 0));
        // Early in the 7/8 bar: the extra bar is a 7/8 one too.
        assert_eq!(plan(&beats, 6.0, 0.01), (-3.0, 4.0, 11.01, 7));
        // On the 4/4 after it: a bar of 4.
        assert_eq!(plan(&beats, 11.0, 0.01), (7.0, 11.0, 11.0, 4));
        assert_eq!(beats.count_in_bar(11.0, 0.01).per_bar, 4);
        assert_eq!(beats.count_in_bar(10.5, 0.01).per_bar, 7);
    }

    #[test]
    fn needs_two_rising_beats() {
        assert_eq!(Beats::new(vec![1.0], 4), None);
        assert_eq!(Beats::new(vec![1.0, 1.0], 4), None);
        assert_eq!(Beats::new(vec![0.0, f64::NAN], 4), None);
    }
}
