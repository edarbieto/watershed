//! Approximate tempo (BPM) of a song.
//!
//! Two steps: an onset envelope, which peaks where notes and hits start (the rise in
//! energy from one short frame to the next, summed over frequencies: "spectral
//! flux"), and the beat period, the delay at which that envelope best matches
//! itself (autocorrelation). Given a tempo, [`OnsetEnvelope::first_beat`] finds
//! where the beats of a steady grid fall, and [`OnsetEnvelope::track_beats`] follows
//! every beat of a song whose tempo moves.

use realfft::RealFftPlanner;

/// The rhythm ends after the last onset this strong, as a fraction of the 95th
/// percentile of all onsets.
const END_THRESHOLD: f64 = 0.3;
/// Beats whose pace goes on after the rhythm ends: two bars of 4.
const END_BEATS: usize = 8;
/// An anchor holds a beat when an onset at least this strong (scaled like the
/// tracker's) is within [`ANCHOR_SNAP_SECS`] of it.
const ANCHOR_ONSET: f64 = 1.0;
const ANCHOR_SNAP_SECS: f64 = 0.06;
/// The score of a beat that cannot be reached from the anchor before it.
const UNREACHABLE: f64 = -1e12;
/// How much an anchor weighs in the smoothing, so it stays where the user put it.
const ANCHOR_WEIGHT: f64 = 1e6;
/// Weight of a vocal stem in the sum of a song's envelopes, against 1 for the
/// other stems. Singing follows the beat loosely: on its own the vocal stem gave a
/// wrong tempo on several test songs, and in Change's soft bridge, where it is the
/// loudest stem, it pulled the tempo from 130 to 140.
pub const VOCALS_TEMPO_WEIGHT: f32 = 0.25;
/// A bar the user marked, for [`OnsetEnvelope::track_beats`]: bar 1, or where the
/// tempo or the time signature changes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Anchor {
    /// Seconds from the start of the song.
    pub time: f64,
    /// Beats from here on per beat of the BPM: 2.0 where the beat becomes an eighth
    /// note in a song counted in quarter notes.
    pub pace: f64,
    /// The tempo from here on, in the same beats as the BPM given to the tracker,
    /// when the user set one: the tracker looks for the beats around it instead.
    pub tempo: Option<f64>,
}

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
/// Where an attack shows in the envelope, in hops before the end of the window it
/// enters (measured with clicks, see the tests).
const ATTACK_LEAD: f64 = 1.5;
/// Shorter audio gives no estimate.
const MIN_SECS: f64 = 10.0;
/// Step of the beat phase search, in envelope frames.
const PHASE_STEP: f64 = 0.25;
/// The music starts at the first onset this strong, as a fraction of the 95th
/// percentile of all onsets.
const START_THRESHOLD: f64 = 0.3;
/// The first beat may come this early before the first onset, in beats, so a beat
/// detected a hair late still counts as the first.
const FIRST_BEAT_SLACK: f64 = 0.33;
/// How strongly the beat tracker keeps to the tempo: the penalty for a gap between
/// beats of `lag` frames is `TIGHTNESS · ln(lag / period)²`, against onsets scaled to
/// a standard deviation of 1. Ellis's value; at 100 it locked onto double tempo.
const TIGHTNESS: f64 = 680.0;
/// A beat on an onset weaker than this (scaled like the tracker's) is a beat in
/// silence, and the gap before it pays [`COAST_TIGHTNESS`] instead: with nothing
/// to hear, the tempo holds. With [`TIGHTNESS`] alone, the tracker ran 7 % fast
/// over the first bars of Tool's Forty Six & 2 to land on the riff's offbeats.
const COAST_ONSET: f64 = 0.5;
const COAST_TIGHTNESS: f64 = 20_000.0;
/// Seconds over which the beat tracker scales the onsets, so quiet sections count
/// as much as loud ones.
const LOCAL_SECS: f64 = 8.0;
/// The local scale never goes below this fraction of the whole song's, so silence
/// stays silent.
const LOCAL_FLOOR: f64 = 0.05;
/// The beat tracker looks for the song's tempo this far around the one it is
/// given, as a fraction: a tapped or typed BPM can be a few percent off.
const TRACK_SPAN: f64 = 0.1;
/// The tempo curve looks this far around the song's tempo, as a fraction.
const CURVE_SPAN: f64 = 0.15;
/// Step between the tempos it tries, in natural log (about 0.25 %).
const CURVE_STEP: f64 = 0.0025;
/// Seconds of onsets each point of the curve is measured over, and between points.
/// Short enough for a sudden drop: with 6–8 s, the windows across the start of
/// Change's soft bridge (140 to 130 BPM) mixed both tempos and the beats gained one.
const CURVE_WINDOW_SECS: f64 = 5.0;
const CURVE_HOP_SECS: f64 = 1.0;
/// Beat periods summed to score a tempo in a window.
const CURVE_BEATS: usize = 4;
/// Cost of a change of tempo between two points of the curve: `CURVE_JUMP ·
/// ln(ratio)²`, against scores of 0 to 1. A 10 % jump costs about 0.5, a drift
/// of 0.5 % per second next to nothing.
const CURVE_JUMP: f64 = 55.0;
/// Beats each tracked beat is smoothed over.
const SMOOTH_BEATS: usize = 8;
/// Weight in the smoothing of a beat with no onset, against onsets scaled to a
/// standard deviation of 1: enough to keep a line through a silent stretch.
const SILENT_WEIGHT: f64 = 0.01;

/// How much the onsets of some audio rise, per short frame.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct OnsetEnvelope {
    /// Frames per second.
    pub rate: f64,
    /// Seconds in the audio that frame 0 stands for: an attack shows in the frame
    /// whose window it enters, a little before the window's start plus its length.
    pub start: f64,
    pub values: Vec<f32>,
}

impl OnsetEnvelope {
    /// The envelope of interleaved stereo `samples`, mixed down to mono.
    pub fn compute(samples: &[f32], sample_rate: u32) -> Self {
        let hop = ((f64::from(sample_rate) / ENVELOPE_RATE).round() as usize).max(1);
        let size = (hop * 4).next_power_of_two();
        let rate = f64::from(sample_rate) / hop as f64;
        let start = (size as f64 - ATTACK_LEAD * hop as f64) / f64::from(sample_rate);
        let total = samples.len() / 2;
        if total < size {
            return Self {
                rate,
                start,
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
        Self {
            rate,
            start,
            values,
        }
    }

    /// Adds another envelope at the same rate, e.g. of another stem of the song.
    pub fn add(&mut self, other: &Self) {
        self.add_weighted(other, 1.0);
    }

    /// Adds another envelope at the same rate, its values times `weight`.
    pub fn add_weighted(&mut self, other: &Self, weight: f32) {
        if self.values.is_empty() {
            self.rate = other.rate;
            self.start = other.start;
        }
        if self.values.len() < other.values.len() {
            self.values.resize(other.values.len(), 0.0);
        }
        for (value, add) in self.values.iter_mut().zip(&other.values) {
            *value += add * weight;
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
        Some(refine(&acf, rate, max_lag, best?, REFINE_SPAN))
    }

    /// Seconds of the first beat at `bpm`: the beats are placed where a pulse
    /// train at that tempo lines up best with the onsets, and the first one is at
    /// (or just before) the first clear onset, so silence at the start is skipped.
    /// `None` when [`OnsetEnvelope::bpm`] would give none.
    pub fn first_beat(&self, bpm: f64) -> Option<f64> {
        let rate = self.rate;
        if rate <= 0.0 || bpm <= 0.0 || (self.values.len() as f64) < MIN_SECS * rate {
            return None;
        }
        let onsets = peaks_only(&self.values, (MEAN_WINDOW_SECS * rate) as usize);
        let period = 60.0 / bpm * rate;
        let len = onsets.len() as f64;

        let mut best = None;
        let mut best_score = 0.0;
        let mut phase = 0.0;
        while phase < period {
            let mut score = 0.0;
            let mut at = phase;
            while at < len {
                score += interpolate(&onsets, at);
                at += period;
            }
            if score > best_score {
                best_score = score;
                best = Some(phase);
            }
            phase += PHASE_STEP;
        }
        let phase = best?;

        let mut sorted = onsets.clone();
        sorted.sort_by(f64::total_cmp);
        let strong = sorted[(sorted.len() * 95 / 100).min(sorted.len() - 1)];
        let first_onset = onsets
            .iter()
            .position(|&v| v >= START_THRESHOLD * strong)
            .unwrap_or(0) as f64;
        let beats = ((first_onset - phase) / period - FIRST_BEAT_SLACK)
            .ceil()
            .max(0.0);
        let first = self.start + (phase + beats * period) / rate;
        // The first window cannot see an attack at the very start: a beat there,
        // before the first one found, counts too.
        let earlier = first - 60.0 / bpm;
        Some(if (-0.005..self.start).contains(&earlier) {
            earlier.max(0.0)
        } else {
            first
        })
    }

    /// Seconds where the song's rhythm ends: after the last strong onset (at least
    /// [`END_THRESHOLD`] of the 95th percentile, the loudest within
    /// [`END_WINDOW_SECS`]), only quieter ones follow: notes ringing out, feedback, a
    /// fade. `None` for an envelope too short to tell.
    pub fn rhythm_end(&self) -> Option<f64> {
        let rate = self.rate;
        if rate <= 0.0 || (self.values.len() as f64) < MIN_SECS * rate {
            return None;
        }
        let onsets = peaks_only(&self.values, (MEAN_WINDOW_SECS * rate) as usize);
        let mut sorted = onsets.clone();
        sorted.sort_by(f64::total_cmp);
        let strong = sorted[(sorted.len() * 95 / 100).min(sorted.len() - 1)];
        let last = onsets.iter().rposition(|&v| v >= END_THRESHOLD * strong)?;
        Some(self.start + last as f64 / rate)
    }

    /// Seconds of every beat over the whole envelope, for a song whose tempo moves
    /// around `bpm` (within 25 % or so). Steady after [`OnsetEnvelope::rhythm_end`].
    /// First a tempo curve ([`tempo_curve`]), so a slow intro has its own pace;
    /// then beat tracking by dynamic programming (Ellis, 2007): each frame scores
    /// its onset plus the best score of a beat between half and twice the local
    /// period earlier, minus a penalty that grows as that gap strays from it; the
    /// best path is followed back from the end. The beats found fall on envelope
    /// frames (10 ms), so they are smoothed by a straight line
    /// through each [`SMOOTH_BEATS`] around them: the tempo can drift, the jitter
    /// goes. From each of `anchors` on, the beats go at its pace; an anchor on a
    /// note also holds a beat right there, and the beats after it are counted from
    /// it: which pulse is the beat is the user's call, a riff with strong offbeats
    /// can fool the tracker. Empty when [`OnsetEnvelope::bpm`] would give none.
    pub fn track_beats(&self, bpm: f64, anchors: &[Anchor]) -> Vec<f64> {
        let rate = self.rate;
        if rate <= 0.0 || bpm <= 0.0 || (self.values.len() as f64) < MIN_SECS * rate {
            return Vec::new();
        }
        let mut onsets = peaks_only(&self.values, (MEAN_WINDOW_SECS * rate) as usize);
        // On a common scale, so the penalty weighs the same on any song, and in
        // any section: a guitar alone counts as much as the whole band.
        let n = onsets.len() as f64;
        let mean = onsets.iter().sum::<f64>() / n;
        let deviation = (onsets.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / n).sqrt();
        if deviation <= 0.0 {
            return Vec::new();
        }
        let local = local_deviation(&onsets, (LOCAL_SECS * rate) as usize);
        // How strong the onsets are around each frame, against the whole song's.
        let strength: Vec<f64> = local.iter().map(|l| (l / deviation).min(1.0)).collect();
        for (value, local) in onsets.iter_mut().zip(local) {
            *value /= local.max(LOCAL_FLOOR * deviation);
        }

        // The tempo given picks the beat (not half or double); the audio, the exact
        // pace: a tempo a few percent off would pull the beats away from the song's.
        let slowest = 60.0 / (bpm * (1.0 - TRACK_SPAN)) * rate;
        let max_lag = (onsets.len() / 2).min((slowest * REFINE_BEATS as f64).ceil() as usize);
        let acf = autocorrelation(&onsets, max_lag + 2);
        let bpm = refine(&acf, rate, max_lag, bpm, TRACK_SPAN);

        // Each beat is measured against the tempo of its own section: an intro can
        // be slower than the rest by 10 %. From each anchor on, at its pace and
        // around its tempo, if it has one.
        let frame_of = |time: f64| ((time - self.start) * rate).round().max(0.0) as usize;
        let snap = (ANCHOR_SNAP_SECS * rate).round() as usize;
        let mut anchors: Vec<Anchor> = anchors
            .iter()
            .copied()
            .filter(|a| a.time.is_finite() && a.pace.is_finite() && a.pace > 0.0)
            .collect();
        anchors.sort_by(|a, b| a.time.total_cmp(&b.time));
        // The gap that ends on an anchor still goes at the pace and tempo before it.
        let mut paces = vec![1.0; onsets.len()];
        let mut tempos = vec![bpm; onsets.len()];
        for anchor in &anchors {
            let from = (frame_of(anchor.time) + snap + 1).min(onsets.len());
            paces[from..].fill(anchor.pace);
            if let Some(tempo) = anchor.tempo.filter(|t| t.is_finite() && *t > 0.0) {
                tempos[from..].fill(tempo);
            }
        }
        let mut periods = tempo_curve(&onsets, &strength, rate, &tempos);
        for (period, pace) in periods.iter_mut().zip(&paces) {
            *period /= pace;
        }
        let longest = periods.iter().fold(0.0f64, |a, &p| a.max(p));
        let ln_lag: Vec<f64> = (0..=(longest * 2.0).ceil() as usize + 1)
            .map(|lag| (lag.max(1) as f64).ln())
            .collect();
        // An anchor holds a beat only when the user put it on a note: a bar 1
        // placed anywhere just to line a steady grid up with a later section says
        // nothing about where the song's first beat is.
        let held: Vec<(usize, f64)> = anchors
            .iter()
            .filter(|a| a.time >= self.start)
            .map(|a| (frame_of(a.time), a.time))
            .filter(|&(a, _)| {
                a < onsets.len()
                    && onsets[a.saturating_sub(snap)..(a + snap + 1).min(onsets.len())]
                        .iter()
                        .any(|&v| v >= ANCHOR_ONSET)
            })
            .collect();
        let mut score = onsets.clone();
        let mut previous = vec![usize::MAX; onsets.len()];
        for t in 0..onsets.len() {
            let period = periods[t];
            let ln_period = period.ln();
            let min_lag = ((period / 2.0).round() as usize).max(1);
            let mut max_lag = ((period * 2.0).round() as usize).min(t);
            // After an anchor, every path goes back through it.
            let after_anchor = held[..held.partition_point(|&(a, _)| a < t)]
                .last()
                .map(|&(a, _)| a);
            if let Some(a) = after_anchor {
                max_lag = max_lag.min(t - a);
            }
            let tightness = if onsets[t] < COAST_ONSET {
                COAST_TIGHTNESS
            } else {
                TIGHTNESS
            };
            let mut best = f64::MIN;
            for lag in min_lag..=max_lag {
                let candidate = score[t - lag] - tightness * (ln_lag[lag] - ln_period).powi(2);
                if candidate > best {
                    best = candidate;
                    previous[t] = t - lag;
                }
            }
            if best > f64::MIN {
                score[t] += best;
            } else if after_anchor.is_some() {
                score[t] = UNREACHABLE;
            }
        }

        // The path ends at the best score within the last period.
        let period = periods.last().copied().unwrap_or(1.0);
        let tail = onsets.len().saturating_sub(period.ceil() as usize);
        let mut at = (tail..onsets.len())
            .max_by(|&a, &b| score[a].total_cmp(&score[b]))
            .unwrap_or(0);
        let mut frames = vec![at];
        while previous[at] != usize::MAX {
            at = previous[at];
            frames.push(at);
        }
        frames.reverse();

        let mut raw: Vec<f64> = frames
            .iter()
            .map(|&f| self.start + (f as f64 + peak_offset(&onsets, f)) / rate)
            .collect();
        let mut weights: Vec<f64> = frames.iter().map(|&f| onsets[f] + SILENT_WEIGHT).collect();
        for &(a, time) in &held {
            if let Some(i) = frames.iter().position(|&f| f == a) {
                raw[i] = time;
                weights[i] = ANCHOR_WEIGHT;
            }
        }
        // Smoothed apart where the pace or the tempo changes, from the beat at the
        // anchor on: a line through quarter and eighth notes together would fit
        // neither. That beat also ends the run before it.
        let mut runs = vec![0];
        let mut pace = 1.0;
        for anchor in &anchors {
            let first = frames.partition_point(|&f| f + snap < frame_of(anchor.time));
            let changes = anchor.pace != pace || anchor.tempo.is_some();
            if changes && first > runs[runs.len() - 1] && first < frames.len() {
                runs.push(first);
            }
            pace = anchor.pace;
        }
        runs.push(frames.len());
        let mut beats = Vec::with_capacity(raw.len());
        for run in runs.windows(2) {
            let to = (run[1] + 1).min(frames.len());
            let smoothed = smooth(&raw[run[0]..to], &weights[run[0]..to], SMOOTH_BEATS);
            beats.extend_from_slice(&smoothed[..run[1] - run[0]]);
        }
        if let Some(end) = self.rhythm_end() {
            hold_after(&mut beats, end);
        }
        beats
    }
}

/// Past `end` (seconds), where nothing but notes ringing out can be tracked, the
/// beats go on steadily at the pace of the [`END_BEATS`] before it (their median
/// gap, so one odd beat in a last fill does not set it), as far as they went.
fn hold_after(beats: &mut Vec<f64>, end: f64) {
    let last = beats.partition_point(|&t| t <= end);
    if last < 2 || last == beats.len() {
        return;
    }
    let from = last.saturating_sub(END_BEATS + 1);
    let mut gaps: Vec<f64> = beats[from..last].windows(2).map(|w| w[1] - w[0]).collect();
    gaps.sort_by(f64::total_cmp);
    let gap = gaps[gaps.len() / 2];
    let until = beats[beats.len() - 1];
    beats.truncate(last);
    while beats[beats.len() - 1] + gap <= until + gap / 2.0 {
        beats.push(beats[beats.len() - 1] + gap);
    }
}

/// The local beat period at each frame of `onsets` (scaled as the beat tracker
/// does), in frames, within [`CURVE_SPAN`] of `tempos` (BPM at each frame: the
/// song's, or the one the user set from a point on). The tempo of each window of
/// [`CURVE_WINDOW_SECS`] scores like [`refine`], relative to the window's best and
/// times its confidence: how strong its onsets are (`strength`, 0 to 1), so a
/// guitar alone counts but a fade-out barely does, and how much its best tempo
/// stands out, so noise (feedback ringing out) has no say. Then the path through
/// the windows with the best total is chosen, paying for each change of tempo
/// (Viterbi): where no window has a say, the tempo holds. The path goes through
/// ratios to `tempos`, so a tempo the user set moves it for free.
fn tempo_curve(onsets: &[f64], strength: &[f64], rate: f64, tempos: &[f64]) -> Vec<f64> {
    let candidates: Vec<f64> = {
        let (low, high) = (1.0 - CURVE_SPAN, 1.0 + CURVE_SPAN);
        let steps = ((high / low).ln() / CURVE_STEP).ceil() as usize;
        (0..=steps)
            .map(|i| low * (CURVE_STEP * i as f64).exp())
            .collect()
    };
    let lowest = tempos.iter().fold(f64::INFINITY, |a, &t| a.min(t));
    let slowest = 60.0 / (lowest * candidates[0]) * rate;
    let max_lag = (slowest * CURVE_BEATS as f64).ceil() as usize;
    let window = ((CURVE_WINDOW_SECS * rate) as usize).max(max_lag * 2);
    let hop = ((CURVE_HOP_SECS * rate) as usize).max(1);

    let centers: Vec<usize> = (0..onsets.len().div_ceil(hop)).map(|i| i * hop).collect();
    let scores: Vec<Vec<f64>> = centers
        .iter()
        .map(|&center| {
            let start = center.saturating_sub(window / 2);
            let end = (start + window).min(onsets.len());
            let start = end.saturating_sub(window);
            let acf = autocorrelation(&onsets[start..end], max_lag + 2);
            let tempo = tempos[center.min(tempos.len() - 1)];
            let scores: Vec<f64> = candidates
                .iter()
                .map(|&ratio| {
                    let beat = 60.0 / (tempo * ratio) * rate;
                    (1..=CURVE_BEATS)
                        .map(|k| interpolate(&acf, k as f64 * beat))
                        .sum()
                })
                .collect();
            let best = scores.iter().fold(0.0f64, |a, &s| a.max(s));
            let mean = scores.iter().sum::<f64>() / scores.len() as f64;
            let strength = strength[start..end].iter().sum::<f64>() / (end - start) as f64;
            // How much the best tempo stands out: noise matches all about the same.
            let contrast = if best > 0.0 {
                (best - mean) / best
            } else {
                0.0
            };
            let weight = strength * contrast;
            if best > 0.0 {
                scores.iter().map(|s| s / best * weight).collect()
            } else {
                vec![0.0; scores.len()]
            }
        })
        .collect();

    // Viterbi over the windows.
    let jump = |a: usize, b: usize| CURVE_JUMP * (CURVE_STEP * a.abs_diff(b) as f64).powi(2);
    let mut total = scores[0].clone();
    let mut from = vec![vec![0usize; candidates.len()]; centers.len()];
    for (w, window_scores) in scores.iter().enumerate().skip(1) {
        let next: Vec<f64> = (0..candidates.len())
            .map(|j| {
                let (i, best) = (0..candidates.len())
                    .map(|i| (i, total[i] - jump(i, j)))
                    .max_by(|a, b| a.1.total_cmp(&b.1))
                    .unwrap_or((j, 0.0));
                from[w][j] = i;
                best + window_scores[j]
            })
            .collect();
        total = next;
    }
    let mut path = vec![0usize; centers.len()];
    path[centers.len() - 1] = (0..candidates.len())
        .max_by(|&a, &b| total[a].total_cmp(&total[b]))
        .unwrap_or(0);
    for w in (1..centers.len()).rev() {
        path[w - 1] = from[w][path[w]];
    }

    // Per frame, the ratio between the window centers, times the frame's tempo.
    let ratio = |w: usize| candidates[path[w]];
    (0..onsets.len())
        .map(|t| {
            let w = t / hop;
            let ratio = if w + 1 >= centers.len() {
                ratio(w.min(centers.len() - 1))
            } else {
                let frac = (t - centers[w]) as f64 / hop as f64;
                ratio(w) * (1.0 - frac) + ratio(w + 1) * frac
            };
            60.0 / (tempos[t] * ratio) * rate
        })
        .collect()
}

/// The standard deviation of `values` over the `window` around each of them.
fn local_deviation(values: &[f64], window: usize) -> Vec<f64> {
    let half = window.max(1) / 2;
    let mut sums = vec![(0.0f64, 0.0f64)];
    for &v in values {
        let (sum, squares) = sums[sums.len() - 1];
        sums.push((sum + v, squares + v * v));
    }
    (0..values.len())
        .map(|i| {
            let (start, end) = (i.saturating_sub(half), (i + half + 1).min(values.len()));
            let n = (end - start) as f64;
            let mean = (sums[end].0 - sums[start].0) / n;
            let squares = (sums[end].1 - sums[start].1) / n;
            (squares - mean * mean).max(0.0).sqrt()
        })
        .collect()
}

/// The tempo within `span` (a fraction) of `guess` whose first [`REFINE_BEATS`]
/// beat periods best match the autocorrelation `acf` of an envelope at `rate`.
fn refine(acf: &[f64], rate: f64, max_lag: usize, guess: f64, span: f64) -> f64 {
    let at = |lag: f64| interpolate(acf, lag);
    let mut best = guess;
    let mut best_score = f64::MIN;
    let mut bpm = guess * (1.0 - span);
    while bpm <= guess * (1.0 + span) {
        let beat = 60.0 / bpm * rate;
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
    best
}

/// Where the peak around `values[i]` really is, in frames from `i` (-0.5 to 0.5):
/// the top of the parabola through it and its neighbours.
fn peak_offset(values: &[f64], i: usize) -> f64 {
    let (Some(&before), Some(&after)) = (
        i.checked_sub(1).and_then(|j| values.get(j)),
        values.get(i + 1),
    ) else {
        return 0.0;
    };
    let at = values[i];
    let curve = before - 2.0 * at + after;
    if curve >= 0.0 {
        return 0.0;
    }
    (0.5 * (before - after) / curve).clamp(-0.5, 0.5)
}

/// Each of `beats` moved onto the straight line that best fits the `window` beats
/// around it (fewer at the ends), by least squares over the beat number. Each beat
/// counts as much as its `weights`, so beats in silence, placed only by the tempo,
/// do not pull the ones on real onsets.
fn smooth(beats: &[f64], weights: &[f64], window: usize) -> Vec<f64> {
    let half = window / 2;
    (0..beats.len())
        .map(|i| {
            let range = i.saturating_sub(half)..(i + half + 1).min(beats.len());
            let points = || range.clone().map(|k| (k as f64, beats[k], weights[k]));
            let total: f64 = points().map(|(_, _, w)| w).sum();
            let mean_k = points().map(|(k, _, w)| k * w).sum::<f64>() / total;
            let mean_t = points().map(|(_, t, w)| t * w).sum::<f64>() / total;
            let (mut cov, mut var) = (0.0, 0.0);
            for (k, t, w) in points() {
                cov += w * (k - mean_k) * (t - mean_t);
                var += w * (k - mean_k).powi(2);
            }
            if var <= 0.0 {
                return beats[i];
            }
            mean_t + cov / var * (i as f64 - mean_k)
        })
        .collect()
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
        clicks_from(bpm, sample_rate, secs, 0.0)
    }

    /// [`clicks`] with only the quiet noise before `first` seconds.
    fn clicks_from(bpm: f64, sample_rate: u32, secs: f64, first: f64) -> Vec<f32> {
        let frames = (secs * f64::from(sample_rate)) as usize;
        let lead = (first * f64::from(sample_rate)) as usize;
        let beat = 60.0 / bpm * f64::from(sample_rate);
        let mut seed = 1u32;
        let mut noise = move || {
            seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            (seed >> 8) as f32 / (1 << 23) as f32 - 1.0
        };
        let mut samples = Vec::with_capacity(frames * 2);
        for i in 0..frames {
            if i < lead {
                let s = noise() * 0.01;
                samples.extend([s, s]);
                continue;
            }
            let beats = (i - lead) as f64 / beat;
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
    fn finds_the_first_beat_after_silence() {
        for (bpm, rate, first) in [
            (120.0, 48_000, 2.0),
            (97.0, 44_100, 3.37),
            (143.5, 48_000, 0.0),
        ] {
            let audio = clicks_from(bpm, rate, 30.0, first);
            let found = OnsetEnvelope::compute(&audio, rate)
                .first_beat(bpm)
                .expect("a first beat");
            assert!(
                (found - first).abs() < 0.005,
                "{bpm} BPM at {rate} Hz from {first}: got {found}"
            );
        }
    }

    /// Interleaved stereo clicks at the given times in seconds, over quiet noise.
    fn clicks_at(times: &[f64], sample_rate: u32, secs: f64) -> Vec<f32> {
        let rate = f64::from(sample_rate);
        let mut seed = 7u32;
        let mut noise = move || {
            seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            (seed >> 8) as f32 / (1 << 23) as f32 - 1.0
        };
        let mut next = 0;
        let mut last = f64::NEG_INFINITY;
        (0..(secs * rate) as usize)
            .flat_map(|i| {
                let t = i as f64 / rate;
                while next < times.len() && times[next] <= t {
                    last = times[next];
                    next += 1;
                }
                let s = noise() * (0.01 + (-(t - last) * 60.0).exp() as f32);
                [s, s]
            })
            .collect()
    }

    /// The largest distance from each of `expected` (seconds) to the nearest of
    /// `found`, between `from` and `to` seconds.
    fn worst_miss(expected: &[f64], found: &[f64], from: f64, to: f64) -> f64 {
        expected
            .iter()
            .filter(|t| (from..to).contains(*t))
            .map(|t| found.iter().map(|f| (f - t).abs()).fold(f64::MAX, f64::min))
            .fold(0.0, f64::max)
    }

    #[test]
    fn tracks_steady_beats() {
        let beat = 60.0 / 120.0;
        let times: Vec<f64> = (0..60).map(|k| 1.0 + k as f64 * beat).collect();
        let audio = clicks_at(&times, 48_000, 31.0);
        let found = OnsetEnvelope::compute(&audio, 48_000).track_beats(120.0, &[]);
        let miss = worst_miss(&times, &found, 1.0, 30.0);
        assert!(miss < 0.003, "missed by {miss} s");
    }

    /// An anchor at `time` that keeps the pace.
    fn anchor(time: f64) -> Anchor {
        Anchor {
            time,
            pace: 1.0,
            tempo: None,
        }
    }

    #[test]
    fn changes_pace_at_an_anchor() {
        // 16 quarter notes at 100 BPM, then bars of 7/8 grouped 2 + 2 + 3, whose
        // notes are not a quarter note apart: only the eighth notes fit them.
        let quarter = 0.6;
        let mut notes: Vec<f64> = (0..16).map(|k| 1.0 + k as f64 * quarter).collect();
        let change = 1.0 + 16.0 * quarter;
        let eighths: Vec<f64> = (0..49).map(|k| change + k as f64 * quarter / 2.0).collect();
        notes.extend(
            eighths
                .iter()
                .enumerate()
                .filter(|(k, _)| [0, 2, 4].contains(&(k % 7)))
                .map(|(_, t)| *t),
        );
        let envelope = OnsetEnvelope::compute(&clicks_at(&notes, 48_000, 26.0), 48_000);
        let anchors = [
            anchor(1.0),
            Anchor {
                time: change,
                pace: 2.0,
                tempo: None,
            },
        ];
        let found = envelope.track_beats(100.0, &anchors);
        let miss = worst_miss(&notes[..16], &found, 0.0, change);
        assert!(miss < 0.01, "quarter notes missed by {miss} s");
        let miss = worst_miss(&eighths, &found, change, 24.5);
        assert!(miss < 0.01, "eighth notes missed by {miss} s");
        let within = |from: f64, to: f64| found.iter().filter(|t| (from..to).contains(*t)).count();
        assert_eq!(within(0.9, change - 0.1), 16);
        assert_eq!(within(change - 0.1, 24.5), 47);
    }

    #[test]
    fn follows_a_tempo_the_user_set() {
        // 120 BPM, then from 15 s 80 BPM: a third slower, beyond the tempo curve's
        // reach around 120. A tempo a little off is enough to find it.
        let mut notes: Vec<f64> = (0..28).map(|k| 1.0 + k as f64 * 0.5).collect();
        let change = 1.0 + 28.0 * 0.5;
        let slow: Vec<f64> = (0..20).map(|k| change + k as f64 * 0.75).collect();
        notes.extend(&slow);
        let envelope = OnsetEnvelope::compute(&clicks_at(&notes, 48_000, 30.0), 48_000);
        let without = envelope.track_beats(120.0, &[]);
        assert!(worst_miss(&slow, &without, change, 28.5) > 0.05);
        let anchors = [Anchor {
            time: change,
            pace: 1.0,
            tempo: Some(85.0),
        }];
        let found = envelope.track_beats(120.0, &anchors);
        let miss = worst_miss(&notes, &found, 1.0, 28.5);
        assert!(miss < 0.01, "missed by {miss} s");
        let within = found
            .iter()
            .filter(|t| (change - 0.1..28.5).contains(*t))
            .count();
        assert_eq!(within, 19);
    }

    #[test]
    fn counts_from_bar_one() {
        // Eighth notes at 120 BPM: the beats and the offbeats sound alike, so only
        // bar 1 tells them apart.
        let beat = 0.5;
        let notes: Vec<f64> = (0..80).map(|k| 1.25 + k as f64 * beat / 2.0).collect();
        let envelope = OnsetEnvelope::compute(&clicks_at(&notes, 48_000, 22.0), 48_000);
        for bar_one in [1.25, 1.5] {
            let found = envelope.track_beats(120.0, &[anchor(bar_one)]);
            let beats: Vec<f64> = (0..36).map(|k| bar_one + k as f64 * beat).collect();
            let miss = worst_miss(&beats, &found, 0.0, 20.0);
            assert!(miss < 0.01, "from {bar_one}: missed by {miss} s");
        }
        // A bar 1 off any note says nothing.
        let free = envelope.track_beats(120.0, &[]);
        assert_eq!(envelope.track_beats(120.0, &[anchor(1.37)]), free);
    }

    #[test]
    fn follows_a_song_that_speeds_up() {
        // From 120 to 132 BPM over 40 s, steadily, starting with the first estimate
        // in between.
        let mut times = vec![0.5];
        while *times.last().unwrap() < 40.0 {
            let t = *times.last().unwrap();
            times.push(t + 60.0 / (120.0 + 12.0 * t / 40.0));
        }
        let audio = clicks_at(&times, 44_100, 41.0);
        let envelope = OnsetEnvelope::compute(&audio, 44_100);
        let bpm = envelope.bpm().expect("an estimate");
        let found = envelope.track_beats(bpm, &[]);
        let miss = worst_miss(&times, &found, 1.0, 39.0);
        assert!(miss < 0.003, "{bpm} BPM: missed by {miss} s");
        // Also from a tempo a little off, as a tap can give.
        let found = envelope.track_beats(bpm * 1.06, &[]);
        let miss = worst_miss(&times, &found, 1.0, 39.0);
        assert!(miss < 0.003, "{bpm} × 1.06 BPM: missed by {miss} s");
        // A steady grid would be off by far more.
        let first = envelope.first_beat(bpm).unwrap();
        let steady: Vec<f64> = (0..100).map(|k| first + k as f64 * 60.0 / bpm).collect();
        assert!(worst_miss(&times, &steady, 1.0, 39.0) > 0.05);
    }

    #[test]
    fn follows_a_quiet_slow_intro_into_the_band() {
        // A quiet intro at 122 BPM for 12 s, then a loud band at 134 BPM.
        let mut times = vec![0.25];
        while *times.last().unwrap() < 45.0 {
            let t = *times.last().unwrap();
            times.push(t + 60.0 / if t < 12.0 { 122.0 } else { 134.0 });
        }
        let rate = 48_000;
        let mut audio = clicks_at(&times, rate, 46.0);
        for sample in &mut audio[..12 * 2 * rate as usize] {
            *sample *= 0.05;
        }
        let envelope = OnsetEnvelope::compute(&audio, rate);
        let found = envelope.track_beats(134.0, &[]);
        let intro = worst_miss(&times, &found, 0.5, 11.0);
        let band = worst_miss(&times, &found, 15.0, 44.0);
        assert!(
            intro < 0.01 && band < 0.003,
            "intro {intro} s, band {band} s"
        );
    }

    /// With 8 s windows the beats gained two here; at 120 they still gain one.
    const BRIDGE_BPM: f64 = 125.0;

    #[test]
    fn follows_a_sudden_drop_without_gaining_a_beat() {
        // A loud band at 140 BPM, a soft bridge at 125 from 20 s, the band again
        // at 140 from 40 s. In the bridge a riff plays on beats 1 and 3 and
        // halfway through beat 2: dotted notes, which fit no beat well.
        let tempo = |t: f64| {
            if (20.0..40.0).contains(&t) {
                BRIDGE_BPM
            } else {
                140.0
            }
        };
        let mut beats = vec![0.5];
        while *beats.last().unwrap() < 59.0 {
            let t = *beats.last().unwrap();
            beats.push(t + 60.0 / tempo(t));
        }
        let mut clicks = Vec::new();
        for (k, w) in beats.windows(2).enumerate() {
            if !(20.0..40.0).contains(&w[0]) || k % 2 == 0 {
                clicks.push(w[0]);
            } else if k % 4 == 1 {
                clicks.push((w[0] + w[1]) / 2.0);
            }
        }
        let rate = 44_100;
        let mut audio = clicks_at(&clicks, rate, 60.0);
        let bridge = 20 * 2 * rate as usize..40 * 2 * rate as usize;
        for sample in &mut audio[bridge] {
            *sample *= 0.1;
        }
        let found = OnsetEnvelope::compute(&audio, rate).track_beats(140.0, &[]);
        for (from, to) in [(1.0, 19.0), (42.0, 58.0)] {
            let miss = worst_miss(&beats, &found, from, to);
            assert!(miss < 0.01, "{from}–{to} s: missed by {miss} s");
        }
        // As many beats as the song has, so the bars after the bridge line up.
        let count = |beats: &[f64]| beats.iter().filter(|t| (1.0..58.0).contains(*t)).count();
        assert_eq!(count(&found), count(&beats));
    }

    #[test]
    fn rings_out_at_the_last_pace() {
        // Speeding up to 132 BPM over 30 s, then a chord ringing out for 12 s.
        let mut times = vec![0.5];
        while *times.last().unwrap() < 30.0 {
            let t = *times.last().unwrap();
            times.push(t + 60.0 / (120.0 + 12.0 * t / 30.0));
        }
        let rate = 48_000;
        let mut audio = clicks_at(&times, rate, 42.0);
        let tail = 30 * rate as usize;
        for (i, frame) in audio.chunks_mut(2).skip(tail).enumerate() {
            let t = i as f64 / f64::from(rate);
            let ring = 0.3 * (-t / 4.0).exp() * (std::f64::consts::TAU * 220.0 * t).sin();
            frame[0] += ring as f32;
            frame[1] += ring as f32;
        }
        let envelope = OnsetEnvelope::compute(&audio, rate);
        let end = envelope.rhythm_end().unwrap();
        assert!((29.5..30.6).contains(&end), "ends at {end}");
        let found = envelope.track_beats(126.0, &[]);
        let after: Vec<f64> = found.iter().copied().filter(|&t| t > 31.0).collect();
        assert!(after.len() > 10 && *after.last().unwrap() > 40.0);
        for w in after.windows(2) {
            let bpm = 60.0 / (w[1] - w[0]);
            assert!((bpm - 132.0).abs() < 1.0, "{bpm} BPM after the end");
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
        assert!(OnsetEnvelope::default().track_beats(120.0, &[]).is_empty());
        let silence = OnsetEnvelope::compute(&vec![0.0; 48_000 * 2 * 20], 48_000);
        assert!(silence.track_beats(120.0, &[]).is_empty());
    }

    #[test]
    fn envelopes_add_up() {
        let mut sum = OnsetEnvelope::default();
        let a = OnsetEnvelope {
            rate: 100.0,
            start: 0.0,
            values: vec![1.0, 2.0],
        };
        let b = OnsetEnvelope {
            rate: 100.0,
            start: 0.0,
            values: vec![1.0, 1.0, 1.0],
        };
        sum.add(&a);
        sum.add(&b);
        assert_eq!(sum.rate, 100.0);
        assert_eq!(sum.values, vec![2.0, 3.0, 1.0]);
    }
}
