//! The song's beats: where each one falls, from bar 1 on. A steady grid is one beat
//! every so often; a tempo map follows the beats the song really has.

/// Beat times from bar 1 on, in any unit (seconds in the UI, frames in the engine).
/// Beat 0 is bar 1. Outside the list the beats go on at the pace of its first and
/// last beat, so there are beats before bar 1 (for a count-in) and after the end.
#[derive(Clone, Debug, PartialEq)]
pub struct Beats {
    /// At least two, rising.
    times: Vec<f64>,
    per_bar: u32,
}

impl Beats {
    /// Beats at `times`, or `None` with fewer than two, or times that are not finite
    /// and rising.
    pub fn new(times: Vec<f64>, per_bar: u32) -> Option<Self> {
        let valid = times.len() >= 2
            && times.iter().all(|t| t.is_finite())
            && times.windows(2).all(|w| w[1] > w[0]);
        valid.then(|| Self {
            times,
            per_bar: per_bar.max(1),
        })
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

    pub fn per_bar(&self) -> u32 {
        self.per_bar
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
        index.rem_euclid(i64::from(self.per_bar)) == 0
    }

    /// The length of the bar that ends at the first beat at or after `time`.
    pub fn bar_before(&self, time: f64) -> f64 {
        let index = self.first_from(time);
        self.time(index) - self.time(index - i64::from(self.per_bar))
    }

    /// The length of the beat that `time` falls in.
    pub fn beat_at(&self, time: f64) -> f64 {
        let index = self.first_from(time).max(1);
        self.time(index) - self.time(index - 1)
    }

    /// The same beats with every time multiplied by `factor`, e.g. seconds to frames.
    pub fn scaled(&self, factor: f64) -> Option<Self> {
        Self::new(
            self.times.iter().map(|t| t * factor).collect(),
            self.per_bar,
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
        assert_eq!(beats.bar_before(100.0), 150.0);
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
        assert_eq!(beats.bar_before(24.0), 14.0);
        assert_eq!(beats.beat_at(20.0), 6.0);
        assert_eq!(beats.beat_at(-5.0), 10.0);
        assert_eq!(beats.scaled(2.0).unwrap().time(3), 48.0);
    }

    #[test]
    fn needs_two_rising_beats() {
        assert_eq!(Beats::new(vec![1.0], 4), None);
        assert_eq!(Beats::new(vec![1.0, 1.0], 4), None);
        assert_eq!(Beats::new(vec![0.0, f64::NAN], 4), None);
    }
}
