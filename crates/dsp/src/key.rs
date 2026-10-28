//! Approximate key of a song ("A minor", "Eb major").
//!
//! Two steps: a chroma, how much energy each of the 12 pitch classes (C, C#, D, …)
//! gets over the whole song, and a comparison of it against the typical profile of
//! each of the 24 major and minor keys (Krumhansl–Kessler): the key whose profile
//! correlates best wins. The chroma is kept at 10-cent resolution so a song not
//! tuned to A = 440 Hz is still read right: the tuning is measured first.
//!
//! A major key and its relative minor (C major, A minor) use the same notes, so they
//! are the usual confusion; callers should offer both.

use std::fmt;
use std::str::FromStr;

use realfft::RealFftPlanner;

/// Analysis window, about 0.34 s at 48 kHz: fine enough to tell semitones apart in
/// the bass.
const WINDOW_SECS: f64 = 0.34;
/// Lowest and highest frequencies counted: from the bass to where the overtones of
/// most notes are weak.
const MIN_FREQUENCY: f32 = 55.0;
const MAX_FREQUENCY: f32 = 2000.0;
/// Steps per semitone in the fine chroma: 10 cents each.
const STEPS: usize = 10;
const BINS: usize = 12 * STEPS;
/// Frames quieter than this (RMS) are skipped.
const MIN_RMS: f32 = 0.003;

/// Krumhansl–Kessler key profiles: how strongly each scale degree, from the tonic
/// up in semitones, belongs to a key, as rated by listeners.
const MAJOR_PROFILE: [f32; 12] = [
    6.35, 2.23, 3.48, 2.33, 4.38, 4.09, 2.52, 5.19, 2.39, 3.66, 2.29, 2.88,
];
const MINOR_PROFILE: [f32; 12] = [
    6.33, 2.68, 3.52, 5.38, 2.60, 3.53, 2.54, 4.75, 3.98, 2.69, 3.34, 3.17,
];

/// A major or minor key.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Key {
    /// Pitch class of the tonic: 0 is C, 1 C#, … 11 B.
    pub tonic: u8,
    pub minor: bool,
}

impl Key {
    pub fn new(tonic: i32, minor: bool) -> Self {
        Self {
            tonic: tonic.rem_euclid(12) as u8,
            minor,
        }
    }

    /// The key with the same notes: A minor for C major and back.
    pub fn relative(self) -> Self {
        if self.minor {
            Self::new(i32::from(self.tonic) + 3, false)
        } else {
            Self::new(i32::from(self.tonic) - 3, true)
        }
    }

    /// The key `semitones` higher (lower when negative).
    pub fn transposed(self, semitones: i32) -> Self {
        Self::new(i32::from(self.tonic) + semitones, self.minor)
    }

    /// All 24 keys: the major ones from C, then the minor ones.
    pub fn all() -> impl Iterator<Item = Key> {
        [false, true]
            .into_iter()
            .flat_map(|minor| (0..12).map(move |tonic| Key::new(tonic, minor)))
    }
}

/// Tonic names as usually written: flats for the flat major keys (Eb major) and
/// sharps for most minor keys (F# minor). ASCII, so any font shows them.
const MAJOR_NAMES: [&str; 12] = [
    "C", "Db", "D", "Eb", "E", "F", "F#", "G", "Ab", "A", "Bb", "B",
];
const MINOR_NAMES: [&str; 12] = [
    "C", "C#", "D", "Eb", "E", "F", "F#", "G", "G#", "A", "Bb", "B",
];

/// Short form: `"Eb"` for E-flat major, `"F#m"` for F-sharp minor.
impl fmt::Display for Key {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.minor {
            write!(f, "{}m", MINOR_NAMES[usize::from(self.tonic)])
        } else {
            f.write_str(MAJOR_NAMES[usize::from(self.tonic)])
        }
    }
}

/// Parses the short form, with sharps or flats: `"Eb"`, `"D#"`, `"F#m"`, `"Gbm"`.
impl FromStr for Key {
    type Err = ();

    fn from_str(text: &str) -> Result<Self, ()> {
        let text = text.trim();
        let (note, minor) = match text.strip_suffix('m') {
            Some(note) => (note, true),
            None => (text, false),
        };
        let mut chars = note.chars();
        let letter = chars.next().ok_or(())?;
        let base = match letter.to_ascii_uppercase() {
            'C' => 0,
            'D' => 2,
            'E' => 4,
            'F' => 5,
            'G' => 7,
            'A' => 9,
            'B' => 11,
            _ => return Err(()),
        };
        let accidental = match chars.as_str() {
            "" => 0,
            "#" => 1,
            "b" => -1,
            _ => return Err(()),
        };
        Ok(Key::new(base + accidental, minor))
    }
}

/// The estimate for a song.
#[derive(Clone, Debug, PartialEq)]
pub struct KeyEstimate {
    pub key: Key,
    /// Correlation of the chroma with the key's profile, -1 to 1. Above about 0.8
    /// the song clearly sits in one key.
    pub confidence: f32,
    /// The runner-up key, often the relative or a fifth away.
    pub second: Key,
    /// How far the song's tuning is from A = 440 Hz, in cents.
    pub tuning_cents: f32,
}

/// Energy per pitch class over some audio, at 10-cent resolution.
#[derive(Clone, Debug, PartialEq)]
pub struct Chroma {
    /// Index 0 is C exactly at A = 440 Hz tuning, then up in 10-cent steps.
    fine: [f32; BINS],
}

impl Default for Chroma {
    fn default() -> Self {
        Self { fine: [0.0; BINS] }
    }
}

impl Chroma {
    /// The chroma of interleaved stereo `samples`, mixed down to mono.
    pub fn compute(samples: &[f32], sample_rate: u32) -> Self {
        let mut chroma = Self::default();
        let size = ((f64::from(sample_rate) * WINDOW_SECS) as usize).next_power_of_two();
        let hop = size / 2;
        let total = samples.len() / 2;
        if total < size {
            return chroma;
        }

        // Where each FFT bin goes in the fine chroma, for bins in range.
        let bin_hz = sample_rate as f32 / size as f32;
        let targets: Vec<(usize, usize)> = (1..=size / 2)
            .filter_map(|bin| {
                let hz = bin as f32 * bin_hz;
                if !(MIN_FREQUENCY..=MAX_FREQUENCY).contains(&hz) {
                    return None;
                }
                // Semitones above C, at A = 440 Hz (A is 9 semitones above C).
                let semitones = 12.0 * (hz / 440.0).log2() + 9.0;
                let step = (semitones * STEPS as f32).round() as i64;
                Some((bin, step.rem_euclid(BINS as i64) as usize))
            })
            .collect();

        let fft = RealFftPlanner::<f32>::new().plan_fft_forward(size);
        let window: Vec<f32> = (0..size)
            .map(|i| 0.5 - 0.5 * (std::f32::consts::TAU * i as f32 / size as f32).cos())
            .collect();
        let mut input = fft.make_input_vec();
        let mut spectrum = fft.make_output_vec();
        let mut scratch = fft.make_scratch_vec();
        let mut frame_chroma = [0.0f32; BINS];

        for frame in 0..=(total - size) / hop {
            let stereo = samples[frame * hop * 2..].as_chunks::<2>().0;
            let mut energy = 0.0;
            for ((x, s), w) in input.iter_mut().zip(stereo).zip(&window) {
                let mono = 0.5 * (s[0] + s[1]);
                energy += mono * mono;
                *x = mono * w;
            }
            if (energy / size as f32).sqrt() < MIN_RMS {
                continue;
            }
            // Sizes always match, so this cannot fail.
            let _ = fft.process_with_scratch(&mut input, &mut spectrum, &mut scratch);
            frame_chroma.fill(0.0);
            for &(bin, step) in &targets {
                // Plain magnitudes: on the test songs, compressing them (square root)
                // let noise and bleed pull the estimate off the clear cases.
                frame_chroma[step] += spectrum[bin].norm();
            }
            // Every frame counts the same, loud or soft.
            let sum: f32 = frame_chroma.iter().sum();
            if sum > 0.0 {
                for (total, value) in chroma.fine.iter_mut().zip(&frame_chroma) {
                    *total += value / sum;
                }
            }
        }
        chroma
    }

    /// Adds another chroma, e.g. of another stem of the song.
    pub fn add(&mut self, other: &Self) {
        for (value, add) in self.fine.iter_mut().zip(&other.fine) {
            *value += add;
        }
    }

    /// How far the tuning is from A = 440 Hz, in cents (-50 to 50): where, within a
    /// semitone, the energy concentrates.
    fn tuning_steps(&self) -> f32 {
        // A circular mean over the position within the semitone.
        let (mut x, mut y) = (0.0f32, 0.0f32);
        for (i, value) in self.fine.iter().enumerate() {
            let angle = std::f32::consts::TAU * (i % STEPS) as f32 / STEPS as f32;
            x += value * angle.cos();
            y += value * angle.sin();
        }
        y.atan2(x) / std::f32::consts::TAU * STEPS as f32
    }

    /// Energy per pitch class (0 is C), with the measured tuning taken out.
    pub fn pitch_classes(&self) -> [f32; 12] {
        let offset = self.tuning_steps().round() as i64;
        let mut classes = [0.0f32; 12];
        for (i, value) in self.fine.iter().enumerate() {
            // Steps within half a semitone of a note count for it.
            let centered = i as i64 - offset + STEPS as i64 / 2;
            let class = centered.div_euclid(STEPS as i64).rem_euclid(12) as usize;
            classes[class] += value;
        }
        classes
    }

    /// The most likely key, or `None` for silence.
    pub fn key(&self) -> Option<KeyEstimate> {
        let classes = self.pitch_classes();
        if classes.iter().sum::<f32>() <= 0.0 {
            return None;
        }
        let mut scores: Vec<(Key, f32)> = Key::all()
            .map(|key| {
                let profile = if key.minor {
                    &MINOR_PROFILE
                } else {
                    &MAJOR_PROFILE
                };
                // Rotate the profile so its tonic lands on the key's tonic.
                let rotated: [f32; 12] =
                    std::array::from_fn(|i| profile[(i + 12 - usize::from(key.tonic)) % 12]);
                (key, correlation(&classes, &rotated))
            })
            .collect();
        scores.sort_by(|a, b| b.1.total_cmp(&a.1));
        Some(KeyEstimate {
            key: scores[0].0,
            confidence: scores[0].1,
            second: scores[1].0,
            tuning_cents: self.tuning_steps() * (100 / STEPS) as f32,
        })
    }
}

/// Pearson correlation of two profiles.
fn correlation(a: &[f32; 12], b: &[f32; 12]) -> f32 {
    let mean = |v: &[f32; 12]| v.iter().sum::<f32>() / 12.0;
    let (ma, mb) = (mean(a), mean(b));
    let (mut num, mut da, mut db) = (0.0, 0.0, 0.0);
    for (x, y) in a.iter().zip(b) {
        num += (x - ma) * (y - mb);
        da += (x - ma) * (x - ma);
        db += (y - mb) * (y - mb);
    }
    if da == 0.0 || db == 0.0 {
        0.0
    } else {
        num / (da * db).sqrt()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RATE: u32 = 48_000;

    /// Stereo audio playing chords, each a list of MIDI notes, for `secs` each.
    /// Notes have a few harmonics, like real instruments. `cents` detunes them all.
    fn chords(chords: &[&[i32]], secs: f32, cents: f32) -> Vec<f32> {
        let per_chord = (secs * RATE as f32) as usize;
        let mut samples = Vec::with_capacity(per_chord * chords.len() * 2);
        for chord in chords {
            for i in 0..per_chord {
                let t = i as f32 / RATE as f32;
                let mut s = 0.0;
                for &note in *chord {
                    let hz = 440.0 * 2f32.powf((note as f32 - 69.0 + cents / 100.0) / 12.0);
                    for harmonic in 1..=4 {
                        let h = harmonic as f32;
                        s += (std::f32::consts::TAU * hz * h * t).sin() / h;
                    }
                }
                let s = 0.05 * s;
                samples.extend([s, s]);
            }
        }
        samples
    }

    fn estimate(samples: &[f32]) -> KeyEstimate {
        Chroma::compute(samples, RATE).key().expect("a key")
    }

    #[test]
    fn names_round_trip() {
        for key in Key::all() {
            assert_eq!(key.to_string().parse::<Key>(), Ok(key), "{key}");
        }
        assert_eq!("D#".parse::<Key>(), "Eb".parse::<Key>());
        assert_eq!("Gbm".parse::<Key>(), "F#m".parse::<Key>());
        assert_eq!("H".parse::<Key>(), Err(()));
        assert_eq!(Key::new(0, false).relative().to_string(), "Am");
        assert_eq!(Key::new(9, true).relative().to_string(), "C");
        assert_eq!(Key::new(11, false).transposed(2).to_string(), "Db");
    }

    #[test]
    fn finds_a_major_key() {
        // I–IV–V–I in C, with the bass.
        let c = [48, 60, 64, 67];
        let f = [41, 60, 65, 69];
        let g = [43, 59, 62, 67];
        let samples = chords(&[&c, &f, &g, &c], 1.0, 0.0);
        let estimate = estimate(&samples);
        assert_eq!(estimate.key.to_string(), "C", "{estimate:?}");
        assert!(estimate.tuning_cents.abs() < 6.0, "{estimate:?}");
    }

    #[test]
    fn finds_a_minor_key() {
        // i–iv–V–i in A minor (E major has the leading tone G#).
        let am = [45, 57, 60, 64];
        let dm = [38, 57, 62, 65];
        let e = [40, 56, 59, 64];
        let samples = chords(&[&am, &dm, &e, &am], 1.0, 0.0);
        let estimate = estimate(&samples);
        assert_eq!(estimate.key.to_string(), "Am", "{estimate:?}");
    }

    #[test]
    fn reads_songs_not_tuned_to_a440() {
        // The same C major cadence a third of a semitone sharp (A = 448.6 Hz):
        // read with plain A = 440 bins, many notes would smear into the next one.
        let c = [48, 60, 64, 67];
        let f = [41, 60, 65, 69];
        let g = [43, 59, 62, 67];
        let samples = chords(&[&c, &f, &g, &c], 1.0, 35.0);
        let estimate = estimate(&samples);
        assert_eq!(estimate.key.to_string(), "C", "{estimate:?}");
        assert!((estimate.tuning_cents - 35.0).abs() < 10.0, "{estimate:?}");
    }

    #[test]
    fn silence_has_no_key() {
        let chroma = Chroma::compute(&vec![0.0; RATE as usize * 4], RATE);
        assert_eq!(chroma.key(), None);
    }
}
