//! Approximate chords of a song, one per bar or half bar of the beat grid.
//!
//! Three steps. A **chromagram**: the chroma of [`crate::Chroma`] (how much energy
//! each of the 12 pitch classes gets), but per short frame instead of over the whole
//! song. For each **span** of the grid (a bar or half a bar), the frames inside it are
//! summed and compared with the template of each major and minor triad (its three
//! notes); the bass stem, when there is one, also votes for the root. Last, a
//! **Viterbi** pass picks the sequence of chords with the best total score, where
//! every change of chord costs a little (more in the middle of a bar than on a bar
//! line), so one odd span does not make the chords flicker.

use std::fmt;

use crate::key::{self, BINS, Chroma, Mapping};

/// Frames per analysis window: a hop of a quarter window, about 85 ms.
const OVERLAP: usize = 4;
/// Exponent applied to a span's pitch classes before comparing them: below 1 it
/// lets weaker chord tones count against one loud note.
const COMPRESSION: f32 = 0.5;
/// How much the bass stem's pitch class at the root adds to a chord's score (the
/// template match is 0 to 1).
const BASS_WEIGHT: f32 = 0.45;
/// Cost of changing chord on a bar line, and in the middle of a bar.
const CHANGE_ON_BAR: f32 = 0.04;
const CHANGE_IN_BAR: f32 = 0.12;
/// Frequencies of the bass analysis, in Hz: from below the lowest E of a bass
/// (41 Hz) to about the D above middle C, so mostly fundamentals.
const BASS_RANGE: (f32, f32) = (35.0, 300.0);
/// Spans quieter than this fraction of the median span have no chord.
const SILENT: f32 = 0.1;

/// Chord qualities: triads for now.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Quality {
    Major,
    Minor,
}

impl Quality {
    const ALL: [Quality; 2] = [Quality::Major, Quality::Minor];

    /// Semitones of the chord tones above the root.
    fn intervals(self) -> &'static [usize] {
        match self {
            Quality::Major => &[0, 4, 7],
            Quality::Minor => &[0, 3, 7],
        }
    }
}

/// A chord: a root pitch class (0 is C, 1 C#, … 11 B) and a quality.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Chord {
    pub root: u8,
    pub quality: Quality,
}

const SHARP_NAMES: [&str; 12] = [
    "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
];
const FLAT_NAMES: [&str; 12] = [
    "C", "Db", "D", "Eb", "E", "F", "Gb", "G", "Ab", "A", "Bb", "B",
];

impl Chord {
    pub fn new(root: i32, quality: Quality) -> Self {
        Self {
            root: root.rem_euclid(12) as u8,
            quality,
        }
    }

    /// The chord `semitones` higher (lower when negative).
    pub fn transposed(self, semitones: i32) -> Self {
        Self::new(i32::from(self.root) + semitones, self.quality)
    }

    /// The short name, `"Bb"` or `"F#m"`, spelled with flats or sharps. ASCII, so any
    /// font shows it.
    pub fn name(self, flats: bool) -> String {
        let names = if flats { &FLAT_NAMES } else { &SHARP_NAMES };
        let root = names[usize::from(self.root)];
        match self.quality {
            Quality::Major => root.to_owned(),
            Quality::Minor => format!("{root}m"),
        }
    }

    /// All 24 triads: the major ones from C, then the minor ones.
    fn all() -> impl Iterator<Item = Chord> {
        Quality::ALL
            .into_iter()
            .flat_map(|quality| (0..12).map(move |root| Chord::new(root, quality)))
    }
}

/// With sharps; [`Chord::name`] spells it for a key.
impl fmt::Display for Chord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.name(false))
    }
}

/// The chroma of every short frame of some audio.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Chromagram {
    /// Seconds between frames.
    hop: f64,
    /// Seconds of the middle of the first frame.
    first: f64,
    /// Fine chroma (10-cent steps) per frame, plain magnitudes.
    frames: Vec<[f32; BINS]>,
}

impl Chromagram {
    /// The chromagram of interleaved stereo `samples`, and the whole-song chroma
    /// [`Chroma::compute`] gives, from the same analysis.
    pub fn compute(samples: &[f32], sample_rate: u32) -> (Self, Chroma) {
        Self::compute_in(samples, sample_rate, key::FULL_RANGE, Mapping::Bins)
    }

    /// The chromagram of a bass stem, from its lowest notes: there the fundamentals
    /// are what counts, and an overtone (the fifth, from the third harmonic) must not
    /// outweigh a low root. Spectral peaks at their exact frequency, since a semitone
    /// down there is about one FFT bin wide.
    pub fn compute_bass(samples: &[f32], sample_rate: u32) -> Self {
        Self::compute_in(samples, sample_rate, BASS_RANGE, Mapping::Peaks).0
    }

    fn compute_in(
        samples: &[f32],
        sample_rate: u32,
        range: (f32, f32),
        mapping: Mapping,
    ) -> (Self, Chroma) {
        let size = key::window_size(sample_rate);
        let mut gram = Self {
            hop: (size / OVERLAP) as f64 / f64::from(sample_rate),
            first: (size / 2) as f64 / f64::from(sample_rate),
            frames: Vec::new(),
        };
        let mut chroma = Chroma::default();
        // The key uses a hop of half a window: every other frame here.
        key::scan(
            samples,
            sample_rate,
            OVERLAP,
            range,
            mapping,
            |index, rms, frame| {
                if index % (OVERLAP / 2) == 0 {
                    chroma.add_frame(rms, frame);
                }
                gram.frames.push(*frame);
            },
        );
        (gram, chroma)
    }

    /// Adds another chromagram, e.g. of another stem of the song, frame by frame.
    pub fn add(&mut self, other: &Self) {
        if self.frames.is_empty() {
            *self = other.clone();
            return;
        }
        for (i, frame) in other.frames.iter().enumerate() {
            // Stems normally share the rate, so frames line up one to one.
            let time = other.first + i as f64 * other.hop;
            let Some(index) = self.index(time) else {
                continue;
            };
            if index >= self.frames.len() {
                self.frames.resize(index + 1, [0.0; BINS]);
            }
            for (value, add) in self.frames[index].iter_mut().zip(frame) {
                *value += add;
            }
        }
    }

    /// The frame nearest to `time`, if not before the first.
    fn index(&self, time: f64) -> Option<usize> {
        let index = ((time - self.first) / self.hop).round();
        (index >= 0.0 && self.hop > 0.0).then_some(index as usize)
    }

    /// The pitch classes summed over `start..end` seconds, `offset` steps of tuning
    /// taken out, and the number of frames summed.
    fn span(&self, start: f64, end: f64, offset: i64) -> ([f32; 12], usize) {
        let mut sum = [0.0f32; BINS];
        let from = ((start - self.first) / self.hop).ceil().max(0.0) as usize;
        let to = (((end - self.first) / self.hop).ceil().max(0.0) as usize).min(self.frames.len());
        let mut count = 0;
        let mut add = |frame: &[f32; BINS]| {
            for (s, v) in sum.iter_mut().zip(frame) {
                *s += v;
            }
            count += 1;
        };
        if from < to {
            self.frames[from..to].iter().for_each(&mut add);
        } else if let Some(frame) = self
            .index(0.5 * (start + end))
            .and_then(|i| self.frames.get(i))
        {
            // A span shorter than a hop: the frame at its middle.
            add(frame);
        }
        (key::fold(&sum, offset), count)
    }
}

/// A stretch of the grid that gets one chord: a bar or half a bar.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Span {
    pub start: f64,
    pub end: f64,
    /// It starts on a bar line: changing chord there is cheaper.
    pub on_bar: bool,
}

/// Spans of half a bar from beat times and the indices of the beats that start a
/// bar (rising). A bar of 4 or more beats is split at its middle beat (rounded
/// down: 7/8 as 3 + 4); shorter bars stay whole. The last beat closes the last span.
pub fn half_bars(beats: &[f64], bar_starts: &[usize]) -> Vec<Span> {
    let mut spans = Vec::new();
    for (k, &first) in bar_starts.iter().enumerate() {
        let last = bar_starts
            .get(k + 1)
            .copied()
            .unwrap_or(beats.len() - 1)
            .min(beats.len() - 1);
        if first >= last {
            continue;
        }
        let count = last - first;
        let mut push = |from: usize, to: usize, on_bar: bool| {
            spans.push(Span {
                start: beats[from],
                end: beats[to],
                on_bar,
            });
        };
        if count >= 4 {
            let middle = first + count / 2;
            push(first, middle, true);
            push(middle, last, false);
        } else {
            push(first, last, true);
        }
    }
    spans
}

/// The chord of each span, `None` where the music is (almost) silent. `treble` is
/// the chromagram of the pitched stems, `bass` the bass stem's, and `tuning` the
/// whole-song chroma, whose measured tuning is taken out.
pub fn estimate_chords(
    treble: &Chromagram,
    bass: Option<&Chromagram>,
    tuning: &Chroma,
    spans: &[Span],
) -> Vec<Option<Chord>> {
    if spans.is_empty() {
        return Vec::new();
    }
    let offset = tuning.tuning_offset();
    let chords: Vec<Chord> = Chord::all().collect();
    let templates: Vec<[f32; 12]> = chords.iter().map(|c| template(*c)).collect();

    // The score of every chord in every span, and how loud each span is.
    let mut levels = Vec::with_capacity(spans.len());
    let scores: Vec<Vec<f32>> = spans
        .iter()
        .map(|span| {
            let (classes, count) = treble.span(span.start, span.end, offset);
            levels.push(classes.iter().sum::<f32>() / count.max(1) as f32);
            let shape = normalized(classes.map(|c| c.powf(COMPRESSION)));
            let bass = bass
                .map(|b| b.span(span.start, span.end, offset).0)
                .map(|b| {
                    let sum: f32 = b.iter().sum();
                    if sum > 0.0 { b.map(|v| v / sum) } else { b }
                });
            chords
                .iter()
                .zip(&templates)
                .map(|(chord, t)| {
                    let mut score: f32 = shape.iter().zip(t).map(|(a, b)| a * b).sum();
                    if let Some(bass) = &bass {
                        score += BASS_WEIGHT * bass[usize::from(chord.root)];
                    }
                    score
                })
                .collect()
        })
        .collect();
    let mut sorted = levels.clone();
    sorted.sort_by(f32::total_cmp);
    let silent = SILENT * sorted[sorted.len() / 2];

    // Viterbi over the spans, silent ones excluded: they break the sequence.
    let mut result = vec![None; spans.len()];
    let mut run = Vec::new();
    // A silent level after the last span closes the last run.
    for (i, &level) in levels.iter().chain([&0.0]).enumerate() {
        if level > silent {
            run.push(i);
            continue;
        }
        for (i, best) in viterbi(&run, spans, &scores) {
            result[i] = Some(chords[best]);
        }
        run.clear();
    }
    result
}

/// The best chord index for each span of `run` (consecutive spans), with a cost per
/// change.
fn viterbi(run: &[usize], spans: &[Span], scores: &[Vec<f32>]) -> Vec<(usize, usize)> {
    let Some(&first) = run.first() else {
        return Vec::new();
    };
    let states = scores[first].len();
    let mut total = scores[first].clone();
    let mut back: Vec<Vec<usize>> = Vec::with_capacity(run.len());
    for &i in &run[1..] {
        let cost = if spans[i].on_bar {
            CHANGE_ON_BAR
        } else {
            CHANGE_IN_BAR
        };
        let (best, best_total) = argmax(&total);
        let mut from = Vec::with_capacity(states);
        let next: Vec<f32> = (0..states)
            .map(|s| {
                let (prev, value) = if total[s] >= best_total - cost {
                    (s, total[s])
                } else {
                    (best, best_total - cost)
                };
                from.push(prev);
                value + scores[i][s]
            })
            .collect();
        back.push(from);
        total = next;
    }
    let mut state = argmax(&total).0;
    let mut path = vec![state; run.len()];
    for (k, from) in back.iter().enumerate().rev() {
        state = from[state];
        path[k] = state;
    }
    run.iter().copied().zip(path).collect()
}

fn argmax(values: &[f32]) -> (usize, f32) {
    values.iter().copied().enumerate().fold(
        (0, f32::MIN),
        |best, (i, v)| if v > best.1 { (i, v) } else { best },
    )
}

/// A chord's notes as a unit-length 12-step profile.
fn template(chord: Chord) -> [f32; 12] {
    let mut t = [0.0f32; 12];
    for &interval in chord.quality.intervals() {
        t[(usize::from(chord.root) + interval) % 12] = 1.0;
    }
    normalized(t)
}

/// `v` scaled to unit length (unchanged when all zero).
fn normalized(v: [f32; 12]) -> [f32; 12] {
    let length = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    if length > 0.0 {
        v.map(|x| x / length)
    } else {
        v
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RATE: u32 = 48_000;

    /// Stereo audio playing chords (MIDI notes with a few harmonics), `secs` each.
    fn play(chords: &[&[i32]], secs: f32) -> Vec<f32> {
        let per_chord = (secs * RATE as f32) as usize;
        let mut samples = Vec::with_capacity(per_chord * chords.len() * 2);
        for chord in chords {
            for i in 0..per_chord {
                let t = i as f32 / RATE as f32;
                let mut s = 0.0;
                for &note in *chord {
                    let hz = 440.0 * 2f32.powf((note as f32 - 69.0) / 12.0);
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

    /// Beats every half second, bars of 4.
    fn grid(secs: f64) -> Vec<Span> {
        let beats: Vec<f64> = (0..=(secs * 2.0) as usize)
            .map(|i| i as f64 * 0.5)
            .collect();
        let bars: Vec<usize> = (0..beats.len()).step_by(4).collect();
        half_bars(&beats, &bars)
    }

    fn names(chords: &[Option<Chord>]) -> Vec<String> {
        chords
            .iter()
            .map(|c| c.map_or("-".to_owned(), |c| c.to_string()))
            .collect()
    }

    #[test]
    fn half_bars_split_long_bars_only() {
        let beats: Vec<f64> = (0..=10).map(f64::from).collect();
        // A bar of 4, a bar of 3, a bar of 3 cut short by the end.
        let spans = half_bars(&beats, &[0, 4, 7]);
        let starts: Vec<(f64, f64, bool)> =
            spans.iter().map(|s| (s.start, s.end, s.on_bar)).collect();
        assert_eq!(
            starts,
            [
                (0.0, 2.0, true),
                (2.0, 4.0, false),
                (4.0, 7.0, true),
                (7.0, 10.0, true)
            ]
        );
    }

    #[test]
    fn finds_triads_bar_by_bar() {
        // C – Am – F – G, one bar (2 s) each, with the bass.
        let c = [48, 60, 64, 67];
        let am = [45, 60, 64, 69];
        let f = [41, 60, 65, 69];
        let g = [43, 59, 62, 67];
        let samples = play(&[&c, &am, &f, &g], 2.0);
        let (gram, chroma) = Chromagram::compute(&samples, RATE);
        let chords = estimate_chords(&gram, None, &chroma, &grid(8.0));
        assert_eq!(
            names(&chords),
            ["C", "C", "Am", "Am", "F", "F", "G", "G"],
            "{chords:?}"
        );
    }

    #[test]
    fn changes_in_the_middle_of_a_bar() {
        // A bar of C, then a bar of half F, half G.
        let c = [48, 60, 64, 67];
        let f = [41, 60, 65, 69];
        let g = [43, 59, 62, 67];
        let samples = play(&[&c, &c, &f, &g], 1.0);
        let (gram, chroma) = Chromagram::compute(&samples, RATE);
        let chords = estimate_chords(&gram, None, &chroma, &grid(4.0));
        assert_eq!(names(&chords), ["C", "C", "F", "G"], "{chords:?}");
    }

    #[test]
    fn silence_has_no_chord() {
        let c = [48, 60, 64, 67];
        let mut samples = play(&[&c, &c], 2.0);
        samples.extend(vec![0.0; RATE as usize * 4 * 2]);
        let (gram, chroma) = Chromagram::compute(&samples, RATE);
        let chords = estimate_chords(&gram, None, &chroma, &grid(8.0));
        // The first silent span still hears the chord's end through the window.
        assert_eq!(names(&chords)[..4], ["C", "C", "C", "C"], "{chords:?}");
        assert_eq!(names(&chords)[5..], ["-", "-", "-"], "{chords:?}");
    }

    #[test]
    fn a_low_bass_note_is_its_root_at_any_rate() {
        // A1 (55 Hz) with its harmonics: the third one is an E, which plain FFT bins
        // could rank above the A this low.
        for rate in [44_100, 48_000] {
            let samples: Vec<f32> = (0..rate * 2)
                .flat_map(|i| {
                    let t = i as f32 / rate as f32;
                    let s: f32 = (1..=4)
                        .map(|h| (std::f32::consts::TAU * 55.0 * h as f32 * t).sin() / h as f32)
                        .sum();
                    [0.1 * s; 2]
                })
                .collect();
            let gram = Chromagram::compute_bass(&samples, rate);
            let (classes, _) = gram.span(0.5, 1.5, 0);
            let loudest = (0..12).max_by(|&a, &b| classes[a].total_cmp(&classes[b]));
            assert_eq!(loudest, Some(9), "{rate} Hz: {classes:?}");
        }
    }

    #[test]
    fn the_same_chroma_for_the_key() {
        let c = [48, 60, 64, 67];
        let samples = play(&[&c, &c], 2.0);
        let (_, chroma) = Chromagram::compute(&samples, RATE);
        assert_eq!(chroma, Chroma::compute(&samples, RATE));
    }

    #[test]
    fn spells_for_the_key() {
        let bb = Chord::new(10, Quality::Major);
        assert_eq!(bb.name(true), "Bb");
        assert_eq!(bb.name(false), "A#");
        assert_eq!(Chord::new(6, Quality::Minor).name(false), "F#m");
        assert_eq!(bb.transposed(3).to_string(), "C#");
    }
}
