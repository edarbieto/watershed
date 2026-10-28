//! Approximate tempo (BPM) of a song.
//!
//! Two steps: an onset envelope, which peaks where notes and hits start (the rise in
//! energy from one short frame to the next, summed over frequencies: "spectral
//! flux"), and the beat period, the delay at which that envelope best matches
//! itself (autocorrelation). Only a number comes out, no beat positions.

use realfft::RealFftPlanner;

/// Envelope frames per second, about.
const ENVELOPE_RATE: f64 = 100.0;
/// Compression of the magnitudes before the flux, so quiet hits count too.
const LOG_COMPRESSION: f32 = 1000.0;
/// Seconds of envelope averaged and subtracted, so only the peaks remain.
const MEAN_WINDOW_SECS: f64 = 0.5;
const MIN_BPM: f64 = 40.0;
const MAX_BPM: f64 = 240.0;
/// Center and width (in octaves) of the preference for usual tempos. Without it,
/// half and double the beat match the envelope about as well as the beat itself.
const PRIOR_BPM: f64 = 120.0;
const PRIOR_OCTAVES: f64 = 1.0;
/// BPM step of the candidate search.
const BPM_STEP: f64 = 0.05;
/// The refinement looks this far around the first guess, as a fraction of it.
const REFINE_SPAN: f64 = 0.03;
/// Multiples of the beat period summed by the refinement: the delay of the
/// sixteenth beat is measured with sixteen times the precision of the first.
const REFINE_BEATS: usize = 16;
/// Shorter audio gives no estimate.
const MIN_SECS: f64 = 10.0;

/// How much the onsets of some audio rise, per short frame.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct OnsetEnvelope {
    /// Frames per second.
    pub rate: f64,
    pub values: Vec<f32>,
}

impl OnsetEnvelope {
    /// The envelope of interleaved stereo `samples`, mixed down to mono.
    pub fn compute(samples: &[f32], sample_rate: u32) -> Self {
        let hop = ((f64::from(sample_rate) / ENVELOPE_RATE).round() as usize).max(1);
        let size = (hop * 4).next_power_of_two();
        let rate = f64::from(sample_rate) / hop as f64;
        let total = samples.len() / 2;
        if total < size {
            return Self {
                rate,
                values: Vec::new(),
            };
        }

        let fft = RealFftPlanner::<f32>::new().plan_fft_forward(size);
        let window: Vec<f32> = (0..size)
            .map(|i| {
                let phase = std::f32::consts::TAU * i as f32 / size as f32;
                0.5 - 0.5 * phase.cos()
            })
            .collect();
        let mut input = fft.make_input_vec();
        let mut spectrum = fft.make_output_vec();
        let mut scratch = fft.make_scratch_vec();
        let bins = spectrum.len();
        let mut previous = vec![0.0f32; bins];
        let mut current = vec![0.0f32; bins];
        let scale = 2.0 / size as f32;

        let frames = (total - size) / hop + 1;
        let mut values = Vec::with_capacity(frames);
        for frame in 0..frames {
            let start = frame * hop * 2;
            let stereo = samples[start..].as_chunks::<2>().0;
            for ((x, s), w) in input.iter_mut().zip(stereo).zip(&window) {
                *x = 0.5 * (s[0] + s[1]) * w;
            }
            // Sizes always match, so this cannot fail.
            let _ = fft.process_with_scratch(&mut input, &mut spectrum, &mut scratch);
            for (level, bin) in current.iter_mut().zip(&spectrum) {
                *level = (1.0 + LOG_COMPRESSION * bin.norm() * scale).ln();
            }
            let flux = if frame == 0 {
                0.0
            } else {
                current
                    .iter()
                    .zip(&previous)
                    .map(|(c, p)| (c - p).max(0.0))
                    .sum()
            };
            values.push(flux);
            std::mem::swap(&mut current, &mut previous);
        }
        Self { rate, values }
    }

    /// Adds another envelope at the same rate, e.g. of another stem of the song.
    pub fn add(&mut self, other: &Self) {
        if self.values.is_empty() {
            self.rate = other.rate;
        }
        if self.values.len() < other.values.len() {
            self.values.resize(other.values.len(), 0.0);
        }
        for (value, add) in self.values.iter_mut().zip(&other.values) {
            *value += add;
        }
    }

    /// The tempo in beats per minute, or `None` for silence or audio too short to
    /// tell. Can be off by a factor of two (70 instead of 140).
    pub fn bpm(&self) -> Option<f64> {
        let rate = self.rate;
        if rate <= 0.0 || (self.values.len() as f64) < MIN_SECS * rate {
            return None;
        }
        let onsets = peaks_only(&self.values, (MEAN_WINDOW_SECS * rate) as usize);
        let max_lag = (onsets.len() / 2).min((60.0 / MIN_BPM * rate) as usize * REFINE_BEATS);
        let acf = autocorrelation(&onsets, max_lag + 2);
        if acf[0] <= 0.0 {
            return None;
        }
        let at = |lag: f64| interpolate(&acf, lag);
        let period = |bpm: f64| 60.0 / bpm * rate;

        let mut best = None;
        let mut best_score = 0.0;
        let mut bpm = MIN_BPM;
        while bpm <= MAX_BPM {
            let prior = (-0.5 * ((bpm / PRIOR_BPM).log2() / PRIOR_OCTAVES).powi(2)).exp();
            let score = prior * at(period(bpm));
            if score > best_score {
                best_score = score;
                best = Some(bpm);
            }
            bpm += BPM_STEP;
        }
        let guess = best?;

        let mut best = guess;
        let mut best_score = f64::MIN;
        let mut bpm = guess * (1.0 - REFINE_SPAN);
        while bpm <= guess * (1.0 + REFINE_SPAN) {
            let beat = period(bpm);
            let score: f64 = (1..=REFINE_BEATS)
                .map(|k| k as f64 * beat)
                .take_while(|&lag| lag < max_lag as f64)
                .map(at)
                .sum();
            if score > best_score {
                best_score = score;
                best = bpm;
            }
            bpm += BPM_STEP / 5.0;
        }
        Some(best)
    }
}

/// `values` minus their local mean, negatives dropped.
fn peaks_only(values: &[f32], window: usize) -> Vec<f64> {
    let half = window.max(1) / 2;
    let mut prefix = Vec::with_capacity(values.len() + 1);
    prefix.push(0.0f64);
    for &v in values {
        prefix.push(prefix.last().copied().unwrap_or_default() + f64::from(v));
    }
    (0..values.len())
        .map(|i| {
            let start = i.saturating_sub(half);
            let end = (i + half + 1).min(values.len());
            let mean = (prefix[end] - prefix[start]) / (end - start) as f64;
            (f64::from(values[i]) - mean).max(0.0)
        })
        .collect()
}

/// Autocorrelation for delays `0..lags`, each divided by the number of products
/// summed, so long delays are not penalized for overlapping less.
fn autocorrelation(values: &[f64], lags: usize) -> Vec<f64> {
    (0..lags.min(values.len()))
        .map(|lag| {
            let n = values.len() - lag;
            let sum: f64 = values[..n]
                .iter()
                .zip(&values[lag..])
                .map(|(a, b)| a * b)
                .sum();
            sum / n as f64
        })
        .collect()
}

/// `values` at a fractional index, linearly interpolated; zero past the end.
fn interpolate(values: &[f64], index: f64) -> f64 {
    let i = index.floor() as usize;
    let frac = index - i as f64;
    match (values.get(i), values.get(i + 1)) {
        (Some(a), Some(b)) => a + (b - a) * frac,
        (Some(a), None) => *a,
        _ => 0.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Interleaved stereo clicks: a short decaying noise burst on every beat, over
    /// quiet noise. Every fourth beat is louder, like a downbeat.
    fn clicks(bpm: f64, sample_rate: u32, secs: f64) -> Vec<f32> {
        let frames = (secs * f64::from(sample_rate)) as usize;
        let beat = 60.0 / bpm * f64::from(sample_rate);
        let mut seed = 1u32;
        let mut noise = move || {
            seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            (seed >> 8) as f32 / (1 << 23) as f32 - 1.0
        };
        let mut samples = Vec::with_capacity(frames * 2);
        for i in 0..frames {
            let beats = i as f64 / beat;
            let since = (beats.fract() * beat) / f64::from(sample_rate);
            let accent = if (beats as usize).is_multiple_of(4) {
                1.0
            } else {
                0.6
            };
            let click = accent * (-since * 60.0).exp() as f32;
            let s = noise() * (0.01 + click);
            samples.extend([s, s]);
        }
        samples
    }

    fn estimate(bpm: f64, sample_rate: u32) -> f64 {
        OnsetEnvelope::compute(&clicks(bpm, sample_rate, 30.0), sample_rate)
            .bpm()
            .expect("an estimate")
    }

    #[test]
    fn finds_the_tempo_of_clicks() {
        for (bpm, rate) in [
            (120.0, 48_000),
            (97.0, 44_100),
            (143.5, 48_000),
            (72.0, 44_100),
        ] {
            let found = estimate(bpm, rate);
            assert!(
                (found - bpm).abs() < 0.5,
                "{bpm} BPM at {rate} Hz: got {found}"
            );
        }
    }

    #[test]
    fn silence_and_short_audio_give_none() {
        assert_eq!(
            OnsetEnvelope::compute(&vec![0.0; 48_000 * 2 * 20], 48_000).bpm(),
            None
        );
        let short = clicks(120.0, 48_000, 5.0);
        assert_eq!(OnsetEnvelope::compute(&short, 48_000).bpm(), None);
        assert_eq!(OnsetEnvelope::default().bpm(), None);
    }

    #[test]
    fn envelopes_add_up() {
        let mut sum = OnsetEnvelope::default();
        let a = OnsetEnvelope {
            rate: 100.0,
            values: vec![1.0, 2.0],
        };
        let b = OnsetEnvelope {
            rate: 100.0,
            values: vec![1.0, 1.0, 1.0],
        };
        sum.add(&a);
        sum.add(&b);
        assert_eq!(sum.rate, 100.0);
        assert_eq!(sum.values, vec![2.0, 3.0, 1.0]);
    }
}
