//! Undo and redo: snapshots of a song's session state (mixer, loops, markers, grid,
//! tempo, pitch, deleted tracks…), kept in memory only. The playhead and the zoom are
//! not part of it.
//!
//! The app shows the history the state every frame. A change becomes one step once it
//! has settled: nothing pressed, no text being typed, and no further change for
//! [`SETTLE`]. So a slider drag, a held key or a run of taps is one step.

use std::time::{Duration, Instant};

/// How long a change must stay unchanged before it becomes a step.
pub const SETTLE: Duration = Duration::from_millis(400);
/// Steps kept; the oldest go first.
const MAX_STEPS: usize = 200;

#[derive(Debug)]
pub struct History<S> {
    undo: Vec<S>,
    redo: Vec<S>,
    /// The state of the last step, the one undo goes back from.
    current: S,
    /// The state seen last, maybe not a step yet, and since when.
    seen: S,
    seen_at: Instant,
}

impl<S: Clone + PartialEq> History<S> {
    pub fn new(state: S, now: Instant) -> Self {
        Self {
            undo: Vec::new(),
            redo: Vec::new(),
            current: state.clone(),
            seen: state,
            seen_at: now,
        }
    }

    /// The state as it is now. `settled`: nothing is being dragged or typed. Returns
    /// how long until a pending change becomes a step, to look again then.
    pub fn observe(&mut self, state: S, settled: bool, now: Instant) -> Option<Duration> {
        if state != self.seen {
            self.seen = state;
            self.seen_at = now;
        }
        if self.seen == self.current {
            return None;
        }
        let waited = now.saturating_duration_since(self.seen_at);
        if settled && waited >= SETTLE {
            self.commit();
            None
        } else {
            Some(SETTLE.saturating_sub(waited).max(Duration::from_millis(50)))
        }
    }

    /// Makes the change seen last a step.
    fn commit(&mut self) {
        if self.seen == self.current {
            return;
        }
        let previous = std::mem::replace(&mut self.current, self.seen.clone());
        self.undo.push(previous);
        if self.undo.len() > MAX_STEPS {
            self.undo.remove(0);
        }
        self.redo.clear();
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty() || self.seen != self.current
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty() && self.seen == self.current
    }

    /// The state to go back to, if any. A change not yet a step becomes one first,
    /// so undo right after a change undoes it.
    pub fn undo(&mut self) -> Option<S> {
        self.commit();
        let previous = self.undo.pop()?;
        let undone = std::mem::replace(&mut self.current, previous.clone());
        self.redo.push(undone);
        self.seen = previous.clone();
        Some(previous)
    }

    /// The state undo left, if nothing changed since.
    pub fn redo(&mut self) -> Option<S> {
        if self.seen != self.current {
            return None;
        }
        let next = self.redo.pop()?;
        let redone = std::mem::replace(&mut self.current, next.clone());
        self.undo.push(redone);
        self.seen = next.clone();
        Some(next)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn later(start: Instant, ms: u64) -> Instant {
        start + Duration::from_millis(ms)
    }

    #[test]
    fn a_settled_change_is_a_step() {
        let t = Instant::now();
        let mut h = History::new(0, t);
        assert!(!h.can_undo());
        // Changing: not a step yet, but undo would take it back.
        assert!(h.observe(1, true, t).is_some());
        assert!(h.can_undo());
        assert!(h.observe(1, true, later(t, 500)).is_none());
        assert_eq!(h.undo(), Some(0));
        assert_eq!(h.redo(), Some(1));
        assert_eq!(h.redo(), None);
    }

    #[test]
    fn a_drag_is_one_step() {
        let t = Instant::now();
        let mut h = History::new(0, t);
        // Values while the pointer is down, then released.
        for (i, v) in [1, 2, 3, 4].into_iter().enumerate() {
            h.observe(v, false, later(t, 100 * i as u64));
        }
        h.observe(4, false, later(t, 2000));
        assert!(h.observe(4, true, later(t, 2100)).is_none());
        assert_eq!(h.undo(), Some(0));
        assert_eq!(h.undo(), None);
    }

    #[test]
    fn quick_changes_are_one_step() {
        let t = Instant::now();
        let mut h = History::new(0, t);
        h.observe(1, true, t);
        h.observe(2, true, later(t, 200));
        h.observe(3, true, later(t, 400));
        h.observe(3, true, later(t, 900));
        assert_eq!(h.undo(), Some(0));
    }

    #[test]
    fn undo_right_after_a_change() {
        let t = Instant::now();
        let mut h = History::new(0, t);
        h.observe(1, true, t);
        h.observe(1, true, later(t, 500));
        h.observe(2, true, later(t, 600));
        // 2 is not a step yet: undo takes it back first.
        assert_eq!(h.undo(), Some(1));
        assert_eq!(h.undo(), Some(0));
        assert_eq!(h.redo(), Some(1));
        assert_eq!(h.redo(), Some(2));
    }

    #[test]
    fn a_new_change_drops_the_redo_steps() {
        let t = Instant::now();
        let mut h = History::new(0, t);
        h.observe(1, true, t);
        h.observe(1, true, later(t, 500));
        assert_eq!(h.undo(), Some(0));
        // The app applies 0; the next frame shows it, so nothing changes.
        assert!(h.observe(0, true, later(t, 600)).is_none());
        assert!(h.can_redo());
        h.observe(5, true, later(t, 700));
        assert!(!h.can_redo());
        h.observe(5, true, later(t, 1200));
        assert_eq!(h.redo(), None);
        assert_eq!(h.undo(), Some(0));
    }

    #[test]
    fn keeps_the_last_steps() {
        let t = Instant::now();
        let mut h = History::new(0, t);
        for v in 1..=MAX_STEPS + 10 {
            h.observe(v, true, later(t, 1000 * v as u64));
            h.observe(v, true, later(t, 1000 * v as u64 + 500));
        }
        let mut undone = 0;
        while h.undo().is_some() {
            undone += 1;
        }
        assert_eq!(undone, MAX_STEPS);
    }
}
