use signalsmith_stretch::Stretch;

use crate::mixer::{MAX_BLOCK, Voice};

/// Tempo range the engine accepts, as a playback speed factor.
pub const MIN_TEMPO: f32 = 0.25;
pub const MAX_TEMPO: f32 = 2.0;
/// Transposition range the engine accepts, in semitones.
pub const MAX_SEMITONES: f32 = 24.0;

/// Tempo and pitch change on the stereo mix, with Signalsmith Stretch.
///
/// It reads the song through a [`Voice`] at `tempo` song frames per output frame and
/// tracks which song frame is being heard, since the stretcher delays the audio by
/// about 120 ms. Nothing here allocates after [`Stretcher::new`].
pub(crate) struct Stretcher {
    stretch: Stretch,
    /// Input fed before the seek point after a seek, so the output starts right away:
    /// one block plus one interval, as the library recommends.
    preroll: usize,
    input_latency: usize,
    output_latency: usize,
    /// Next song frame to feed. Fractional so that slow tempos do not drift.
    source: f64,
    /// Lowest position to report after a seek, while the latency is filling up.
    floor: usize,
    /// Transposition currently applied.
    semitones: f32,
    /// Stereo scratch buffer for the mixed input.
    input: Vec<f32>,
}

impl Stretcher {
    pub fn new(sample_rate: u32) -> Self {
        // Same values as Signalsmith's default preset, which does not expose them.
        let rate = sample_rate as f32;
        let block = (rate * 0.12) as usize;
        let interval = (rate * 0.03) as usize;
        let stretch = Stretch::new(2, block, interval);
        let preroll = block + interval;
        let max_input = (MAX_BLOCK as f32 * MAX_TEMPO).ceil() as usize + 1;
        Self {
            input_latency: stretch.input_latency(),
            output_latency: stretch.output_latency(),
            stretch,
            preroll,
            source: 0.0,
            floor: 0,
            semitones: 0.0,
            input: vec![0.0; preroll.max(max_input) * 2],
        }
    }

    /// Clears the stretcher and positions it so that `frame` is heard next.
    pub fn seek(&mut self, voice: &mut Voice, frame: usize, tempo: f32) {
        self.stretch.reset();
        // The stretcher works on input from `input_latency` frames ago, so feeding up
        // to `frame + input_latency` centers its first output on `frame`.
        let end = frame + self.input_latency;
        let start = end.saturating_sub(self.preroll);
        let pad = self.preroll - (end - start);
        let input = &mut self.input[..self.preroll * 2];
        input.fill(0.0);
        voice.mix_into(start, &mut input[pad * 2..]);
        self.stretch.seek(&*input, f64::from(tempo));
        self.source = end as f64;
        self.floor = frame;
    }

    /// Fills `out` (interleaved stereo) and advances the song by `tempo` frames per
    /// output frame. Past the end of the song the input is silence.
    pub fn render(&mut self, voice: &mut Voice, tempo: f32, semitones: f32, out: &mut [f32]) {
        if semitones != self.semitones {
            self.stretch.set_transpose_factor_semitones(semitones, None);
            self.semitones = semitones;
        }
        let from = self.source as usize;
        self.source += (out.len() / 2) as f64 * f64::from(tempo);
        let to = self.source as usize;
        let input = &mut self.input[..(to - from) * 2];
        input.fill(0.0);
        voice.mix_into(from, input);
        self.stretch.process(&*input, out);
    }

    /// The song frame being heard, given `extra` frames of output latency added after
    /// the stretcher (e.g. a limiter). Output latency counts at the current tempo.
    pub fn heard(&self, tempo: f32, extra: usize) -> usize {
        let behind =
            self.input_latency as f64 + (self.output_latency + extra) as f64 * f64::from(tempo);
        ((self.source - behind).max(0.0) as usize).max(self.floor)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::Song;

    const RATE: u32 = 48_000;

    /// A song with one track: silence until `onset`, then a 440 Hz sine.
    fn voice(frames: usize, onset: usize) -> Voice {
        let samples = (0..frames)
            .flat_map(|i| {
                let phase = i as f32 * 440.0 * std::f32::consts::TAU / RATE as f32;
                let s = if i < onset { 0.0 } else { 0.5 * phase.sin() };
                [s, s]
            })
            .collect();
        Voice::new(Arc::new(Song::new(RATE, vec![("sine".into(), samples)])))
    }

    fn rms(samples: &[f32]) -> f32 {
        (samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32).sqrt()
    }

    #[test]
    fn seek_reports_the_target_and_sound_starts_soon() {
        let mut voice = voice(RATE as usize * 4, 0);
        let mut stretcher = Stretcher::new(RATE);
        stretcher.seek(&mut voice, 48_000, 1.0);
        assert_eq!(stretcher.heard(1.0, 0), 48_000);

        // Sound arrives within the output latency (about 60 ms).
        let mut out = vec![0.0; 512 * 2];
        let mut first_loud = None;
        for block in 0..20 {
            stretcher.render(&mut voice, 1.0, 0.0, &mut out);
            if first_loud.is_none() && rms(&out) > 0.1 {
                first_loud = Some(block * 512);
            }
        }
        let first_loud = first_loud.expect("no output");
        assert!(first_loud <= stretcher.output_latency, "{first_loud}");
    }

    #[test]
    fn position_follows_tempo() {
        let mut voice = voice(RATE as usize * 4, 0);
        let mut stretcher = Stretcher::new(RATE);
        stretcher.seek(&mut voice, 0, 0.5);
        let mut out = vec![0.0; 500 * 2];
        // Past the initial latency, then measure one second of output.
        for _ in 0..96 {
            stretcher.render(&mut voice, 0.5, 0.0, &mut out);
        }
        let before = stretcher.heard(0.5, 0);
        for _ in 0..96 {
            stretcher.render(&mut voice, 0.5, 0.0, &mut out);
        }
        assert_eq!(stretcher.heard(0.5, 0) - before, 24_000);
    }

    #[test]
    fn reported_position_matches_what_is_heard() {
        // Silence, then a sine starting at one second.
        let onset = RATE as usize;
        let mut voice = voice(RATE as usize * 3, onset);

        for tempo in [0.5, 1.0, 1.25] {
            let mut stretcher = Stretcher::new(RATE);
            stretcher.seek(&mut voice, onset - RATE as usize / 2, tempo);
            let mut out = vec![0.0; 64 * 2];
            let heard_at_onset = loop {
                stretcher.render(&mut voice, tempo, 0.0, &mut out);
                if rms(&out) > 0.1 {
                    break stretcher.heard(tempo, 0);
                }
            };
            // Within 10 ms of the true onset (measured: about one 64-frame block).
            let error = heard_at_onset.abs_diff(onset);
            assert!(error < RATE as usize / 100, "tempo {tempo}: off by {error}");
        }
    }

    #[test]
    fn keeps_the_level_at_unity() {
        let mut voice = voice(RATE as usize * 4, 0);
        let mut stretcher = Stretcher::new(RATE);
        stretcher.seek(&mut voice, 0, 1.0);
        let mut out = vec![0.0; 512 * 2];
        for _ in 0..40 {
            stretcher.render(&mut voice, 0.75, -2.0, &mut out);
        }
        // A 0.5 amplitude sine has an RMS of about 0.354.
        let level = rms(&out);
        assert!((level - 0.354).abs() < 0.05, "rms {level}");
    }
}
