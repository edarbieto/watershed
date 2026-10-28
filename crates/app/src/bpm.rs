//! The song's tempo in BPM: estimated on load, corrected by the user (halve, double,
//! type it in or tap it), and saved in the session. Values are at the original
//! speed; the tempo slider scales what is shown.

use std::time::{Duration, Instant};

/// Taps further apart than this start a new count.
pub const TAP_RESET: Duration = Duration::from_secs(2);
/// Taps averaged; older ones are dropped so the user can follow a change.
const MAX_TAPS: usize = 8;
/// Taps needed before the tempo follows them.
const MIN_TAPS: usize = 3;
/// Range of tempos accepted, in BPM.
pub const MIN_BPM: f64 = 20.0;
pub const MAX_BPM: f64 = 400.0;

#[derive(Debug, Default)]
pub struct Bpm {
    /// From the audio, when the song was loaded.
    estimate: Option<f64>,
    /// Set by the user; wins over the estimate.
    user: Option<f64>,
    taps: Vec<Instant>,
}

impl Bpm {
    pub fn new(estimate: Option<f64>, user: Option<f64>) -> Self {
        Self {
            estimate,
            user: user.filter(|bpm| (MIN_BPM..=MAX_BPM).contains(bpm)),
            taps: Vec::new(),
        }
    }

    /// The tempo at the original speed, if known.
    pub fn value(&self) -> Option<f64> {
        self.user.or(self.estimate)
    }

    pub fn estimate(&self) -> Option<f64> {
        self.estimate
    }

    /// The user's value, to save in the session.
    pub fn user(&self) -> Option<f64> {
        self.user
    }

    pub fn set(&mut self, bpm: f64) {
        self.user = Some(bpm.clamp(MIN_BPM, MAX_BPM));
    }

    /// Back to the estimate.
    pub fn reset(&mut self) {
        self.user = None;
    }

    /// Multiplies the tempo, e.g. by 2 or 0.5 to fix a half or double estimate.
    pub fn scale(&mut self, factor: f64) {
        if let Some(bpm) = self.value() {
            self.set(bpm * factor);
        }
    }

    /// A tap at `now`, heard while playing at `tempo` (1.0 is the original speed).
    /// Sets the tempo once enough taps are in.
    pub fn tap(&mut self, now: Instant, tempo: f64) {
        if self
            .taps
            .last()
            .is_some_and(|&last| now.saturating_duration_since(last) > TAP_RESET)
        {
            self.taps.clear();
        }
        if self.taps.len() == MAX_TAPS {
            self.taps.remove(0);
        }
        self.taps.push(now);
        if self.taps.len() >= MIN_TAPS && tempo > 0.0 {
            let span = now.saturating_duration_since(self.taps[0]).as_secs_f64();
            let beat = span / (self.taps.len() - 1) as f64;
            if beat > 0.0 {
                self.set(60.0 / beat / tempo);
            }
        }
    }

    /// Taps so far in the current count.
    pub fn taps(&self, now: Instant) -> usize {
        match self.taps.last() {
            Some(&last) if now.saturating_duration_since(last) <= TAP_RESET => self.taps.len(),
            _ => 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_value_wins_and_resets() {
        let mut bpm = Bpm::new(Some(63.0), None);
        assert_eq!(bpm.value(), Some(63.0));
        bpm.scale(2.0);
        assert_eq!(bpm.value(), Some(126.0));
        assert_eq!(bpm.user(), Some(126.0));
        bpm.reset();
        assert_eq!(bpm.value(), Some(63.0));
        assert_eq!(bpm.user(), None);
    }

    #[test]
    fn out_of_range_values_are_ignored_or_clamped() {
        assert_eq!(Bpm::new(Some(120.0), Some(f64::NAN)).value(), Some(120.0));
        assert_eq!(Bpm::new(None, Some(1000.0)).value(), None);
        let mut bpm = Bpm::new(None, None);
        bpm.scale(2.0);
        assert_eq!(bpm.value(), None);
        bpm.set(1000.0);
        assert_eq!(bpm.value(), Some(MAX_BPM));
    }

    #[test]
    fn taps_set_the_tempo_at_the_original_speed() {
        let start = Instant::now();
        let mut bpm = Bpm::new(Some(140.0), None);
        // Half a second apart is 120 BPM heard; at 75 % speed the song is 160 BPM.
        for i in 0..4 {
            bpm.tap(start + Duration::from_millis(500 * i), 0.75);
            if i < 2 {
                assert_eq!(bpm.value(), Some(140.0), "too few taps");
            }
        }
        let value = bpm.value().unwrap();
        assert!((value - 160.0).abs() < 1e-9, "{value}");
        assert_eq!(bpm.taps(start + Duration::from_millis(1500)), 4);
    }

    #[test]
    fn a_pause_starts_a_new_count() {
        let start = Instant::now();
        let mut bpm = Bpm::new(None, None);
        for i in 0..3 {
            bpm.tap(start + Duration::from_millis(400 * i), 1.0);
        }
        let later = start + Duration::from_secs(10);
        assert_eq!(bpm.taps(later), 0);
        bpm.tap(later, 1.0);
        bpm.tap(later + Duration::from_millis(1000), 1.0);
        assert_eq!(bpm.taps(later + Duration::from_millis(1000)), 2);
        // Still the 150 BPM of the first count: two taps are not enough.
        assert!((bpm.value().unwrap() - 150.0).abs() < 1e-9);
    }
}
