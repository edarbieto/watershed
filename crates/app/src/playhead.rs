//! Smooth playhead for drawing. The engine updates its position once per audio
//! callback (about 10 ms, more and less regular on Bluetooth), so drawn as is it
//! moves in steps, which shows when zoomed in. This clock runs at the speed of
//! playback between frames and is pulled gently towards the engine's position.

/// How quickly the clock follows the engine, in seconds. Longer is smoother but
/// lags more after a tempo change.
const TIME_CONSTANT_SECS: f64 = 0.1;
/// A difference larger than this is a jump (seek, loop), taken at once.
const SNAP_SECS: f64 = 0.15;

#[derive(Debug, Default)]
pub struct Playhead {
    secs: f64,
}

impl Playhead {
    pub fn secs(&self) -> f64 {
        self.secs
    }

    /// Advances the clock by `dt` seconds of real time. `reported` is the engine's
    /// position and `tempo` the song seconds played per real second.
    pub fn update(&mut self, reported: f64, dt: f64, playing: bool, tempo: f64) -> f64 {
        let predicted = self.secs + dt * tempo;
        let error = reported - predicted;
        self.secs = if !playing || error.abs() > SNAP_SECS {
            reported
        } else {
            predicted + error * (1.0 - (-dt / TIME_CONSTANT_SECS).exp())
        };
        self.secs
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The engine's position in steps of `block` seconds, as callbacks report it.
    fn reported(time: f64, block: f64) -> f64 {
        (time / block).floor() * block
    }

    #[test]
    fn smooths_callback_steps() {
        let mut playhead = Playhead::default();
        let (dt, block) = (1.0 / 120.0, 0.0116);
        let mut time = 0.0;
        let mut worst_step: f64 = 0.0;
        let mut last = 0.0;
        for frame in 0..600 {
            time += dt;
            let secs = playhead.update(reported(time, block), dt, true, 1.0);
            if frame > 120 {
                // Once settled it moves by about one frame of time, never in blocks.
                worst_step = worst_step.max((secs - last - dt).abs());
                assert!((secs - time).abs() < block, "lags by {}", time - secs);
            }
            last = secs;
        }
        assert!(worst_step < 0.002, "uneven by {worst_step}");
    }

    #[test]
    fn follows_tempo() {
        let mut playhead = Playhead::default();
        let dt = 1.0 / 60.0;
        let mut time = 0.0;
        for _ in 0..300 {
            time += dt * 0.5;
            playhead.update(time, dt, true, 0.5);
        }
        assert!((playhead.secs() - time).abs() < 1e-3);
    }

    #[test]
    fn jumps_and_pauses_are_immediate() {
        let mut playhead = Playhead::default();
        playhead.update(10.0, 0.01, true, 1.0);
        assert_eq!(playhead.secs(), 10.0);
        playhead.update(42.0, 0.01, false, 1.0);
        assert_eq!(playhead.secs(), 42.0);
    }
}
