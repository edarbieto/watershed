//! A-B loop state: the current loop and the saved ones. Times are in seconds, so
//! they survive resampling to another device rate.

use watershed_audio::MIN_LOOP_SECS;

/// A loop the user saved, to come back to later.
#[derive(Clone, Debug, PartialEq)]
pub struct SavedLoop {
    pub name: String,
    pub start: f64,
    pub end: f64,
}

#[derive(Debug, Default)]
pub struct Loops {
    /// Loop start and end being edited. Either may be missing.
    pub a: Option<f64>,
    pub b: Option<f64>,
    /// Whether playback loops between A and B.
    pub enabled: bool,
    pub saved: Vec<SavedLoop>,
    /// Number for the next default name, "Loop N".
    saved_count: usize,
}

impl Loops {
    /// A and B when both are set and far enough apart to loop.
    pub fn region(&self) -> Option<(f64, f64)> {
        let (a, b) = (self.a?, self.b?);
        (b >= a + MIN_LOOP_SECS).then_some((a, b))
    }

    /// The loop playback should follow, if any.
    pub fn active(&self) -> Option<(f64, f64)> {
        self.region().filter(|_| self.enabled)
    }

    /// Starts a new loop here: sets A and clears B, even if a loop was set.
    pub fn set_a(&mut self, secs: f64) {
        self.a = Some(secs);
        self.b = None;
        self.enabled = false;
    }

    /// Forgets the current A and B. Saved loops stay.
    pub fn clear(&mut self) {
        self.a = None;
        self.b = None;
        self.enabled = false;
    }

    /// Sets the loop end and starts looping. Clears A if it is no longer before B.
    pub fn set_b(&mut self, secs: f64) {
        self.b = Some(secs);
        if self.region().is_none() {
            self.a = None;
        }
        self.enabled = self.region().is_some();
    }

    /// Sets both ends at once, e.g. from a drag on the waveform, and starts looping.
    /// Selections too short to loop are ignored.
    pub fn select(&mut self, start: f64, end: f64) {
        if end >= start + MIN_LOOP_SECS {
            self.a = Some(start);
            self.b = Some(end);
            self.enabled = true;
        }
    }

    pub fn toggle(&mut self) {
        self.enabled = !self.enabled && self.region().is_some();
    }

    /// Saves the current loop under a default name. Returns false if there is none.
    pub fn save(&mut self) -> bool {
        let Some((start, end)) = self.region() else {
            return false;
        };
        self.saved_count += 1;
        self.saved.push(SavedLoop {
            name: format!("Loop {}", self.saved_count),
            start,
            end,
        });
        true
    }

    /// Makes a saved loop the current one and starts looping.
    pub fn recall(&mut self, index: usize) {
        if let Some(saved) = self.saved.get(index) {
            self.a = Some(saved.start);
            self.b = Some(saved.end);
            self.enabled = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_then_b_starts_looping() {
        let mut loops = Loops::default();
        loops.set_a(10.0);
        assert_eq!(loops.active(), None);
        loops.set_b(20.0);
        assert_eq!(loops.active(), Some((10.0, 20.0)));
        loops.toggle();
        assert_eq!(loops.active(), None);
        loops.toggle();
        assert_eq!(loops.active(), Some((10.0, 20.0)));
    }

    #[test]
    fn a_always_starts_a_new_loop() {
        let mut loops = Loops::default();
        loops.select(10.0, 20.0);
        loops.set_a(15.0);
        assert_eq!((loops.a, loops.b, loops.enabled), (Some(15.0), None, false));
    }

    #[test]
    fn clear_keeps_saved_loops() {
        let mut loops = Loops::default();
        loops.select(10.0, 20.0);
        loops.save();
        loops.clear();
        assert_eq!((loops.a, loops.b, loops.active()), (None, None, None));
        assert_eq!(loops.saved.len(), 1);
    }

    #[test]
    fn b_before_a_clears_a() {
        let mut loops = Loops::default();
        loops.select(10.0, 20.0);
        loops.set_b(5.0);
        assert_eq!((loops.a, loops.b, loops.enabled), (None, Some(5.0), false));
        // A toggle without a full loop does nothing.
        loops.toggle();
        assert!(!loops.enabled);
    }

    #[test]
    fn short_selections_are_ignored() {
        let mut loops = Loops::default();
        loops.select(10.0, 20.0);
        loops.select(30.0, 30.1);
        assert_eq!(loops.active(), Some((10.0, 20.0)));
    }

    #[test]
    fn save_and_recall() {
        let mut loops = Loops::default();
        assert!(!loops.save());
        loops.select(10.0, 20.0);
        assert!(loops.save());
        loops.select(30.0, 40.0);
        assert!(loops.save());
        assert_eq!(loops.saved[1].name, "Loop 2");

        loops.toggle();
        loops.recall(0);
        assert_eq!(loops.active(), Some((10.0, 20.0)));
    }
}
