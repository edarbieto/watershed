//! The visible part of the song: zoom and horizontal scroll. Times are in seconds.

/// Narrowest view, in seconds. The waveform peaks have one bucket per 256 frames
/// (~6 ms), so closer than this would only show blocks.
const MIN_SPAN_SECS: f64 = 2.0;
/// A position change larger than this between frames is a jump (seek, loop), not
/// playback moving on.
const JUMP_SECS: f64 = 0.5;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct View {
    pub start: f64,
    pub span: f64,
    /// Length of the song.
    duration: f64,
}

impl View {
    /// The whole song.
    pub fn new(duration: f64) -> Self {
        let duration = duration.max(0.0);
        Self {
            start: 0.0,
            span: duration,
            duration,
        }
    }

    pub fn duration(&self) -> f64 {
        self.duration
    }

    pub fn end(&self) -> f64 {
        self.start + self.span
    }

    pub fn is_full(&self) -> bool {
        self.span >= self.duration
    }

    pub fn contains(&self, secs: f64) -> bool {
        (self.start..=self.end()).contains(&secs)
    }

    /// Zooms by `factor` (above 1 zooms in) keeping `anchor` at the same place on
    /// screen.
    pub fn zoom(&mut self, factor: f64, anchor: f64) {
        if factor <= 0.0 || self.duration <= 0.0 {
            return;
        }
        let min_span = MIN_SPAN_SECS.min(self.duration);
        let span = (self.span / factor).clamp(min_span, self.duration);
        let fraction = if self.span > 0.0 {
            ((anchor - self.start) / self.span).clamp(0.0, 1.0)
        } else {
            0.0
        };
        self.span = span;
        self.start = anchor - fraction * span;
        self.clamp();
    }

    /// Moves the view by `secs` (positive is later in the song).
    pub fn pan(&mut self, secs: f64) {
        self.start += secs;
        self.clamp();
    }

    /// Centers the view on `secs`.
    pub fn center_on(&mut self, secs: f64) {
        self.start = secs - self.span / 2.0;
        self.clamp();
    }

    /// Keeps the playhead in view, like Reaper's continuous scrolling: while playing
    /// the view stays centered on it, except near the start and end of the song,
    /// where the view stops and the playhead moves. While paused the view is free,
    /// but a jump outside it (seek, marker key) centers it on the playhead.
    pub fn follow(&mut self, last: f64, position: f64, playing: bool) {
        if self.is_full() {
            return;
        }
        let jumped_outside = (position - last).abs() > JUMP_SECS && !self.contains(position);
        if playing || jumped_outside {
            self.center_on(position);
        }
    }

    fn clamp(&mut self) {
        self.start = self.start.clamp(0.0, (self.duration - self.span).max(0.0));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zoom_keeps_the_anchor_in_place() {
        let mut view = View::new(100.0);
        view.zoom(4.0, 50.0);
        assert_eq!((view.start, view.span), (37.5, 25.0));
        // 40 s is at 10% of the view; it stays there.
        view.zoom(2.0, 40.0);
        assert!((view.start - 38.75).abs() < 1e-9);
        assert_eq!(view.span, 12.5);
    }

    #[test]
    fn zoom_is_limited() {
        let mut view = View::new(100.0);
        view.zoom(1000.0, 99.0);
        assert_eq!(view.span, MIN_SPAN_SECS);
        // 99 s stays at 99% of the width.
        assert!((view.start - 97.02).abs() < 1e-9);
        view.zoom(0.001, 10.0);
        assert!(view.is_full());
        assert_eq!(view.start, 0.0);
    }

    #[test]
    fn pan_stays_inside_the_song() {
        let mut view = View::new(100.0);
        view.zoom(10.0, 0.0);
        view.pan(-5.0);
        assert_eq!(view.start, 0.0);
        view.pan(500.0);
        assert_eq!(view.end(), 100.0);
        view.center_on(50.0);
        assert_eq!(view.start, 45.0);
    }

    #[test]
    fn playing_keeps_the_playhead_centered() {
        let mut view = View::new(100.0);
        view.zoom(10.0, 0.0);
        // Panned elsewhere, then play: the view comes back to the playhead.
        view.pan(50.0);
        view.follow(20.0, 20.02, true);
        assert!((view.start - 15.02).abs() < 1e-9);
        // Near the start and the end the view stops and the playhead moves.
        view.follow(2.0, 2.02, true);
        assert_eq!(view.start, 0.0);
        view.follow(98.0, 98.02, true);
        assert_eq!(view.end(), 100.0);
    }

    #[test]
    fn paused_view_is_free_until_a_jump() {
        let mut view = View::new(100.0);
        view.zoom(10.0, 0.0);
        view.pan(50.0);
        // Paused: the view stays where the user left it.
        view.follow(20.0, 20.0, false);
        assert_eq!(view.start, 50.0);
        // A jump (seek, marker key) outside the view centers it there.
        view.follow(20.0, 80.0, false);
        assert_eq!(view.start, 75.0);
    }
}
