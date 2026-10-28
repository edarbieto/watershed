//! Metronome click and count-in, mixed after the stretcher so they are never
//! time-stretched or transposed.
//!
//! The beats come from the song's grid: bar 1 and the beat length, in song frames.
//! The click follows the song frame being heard, so it lands on the beat the user
//! hears at any tempo. The sounds are synthesized once, when the engine starts.

use std::f32::consts::TAU;
use std::ops::Range;
use std::sync::atomic::{AtomicBool, AtomicU8, AtomicU32, AtomicU64, Ordering};

use crate::AtomicF32;

/// Beats closer than this to a seek target still click: a playhead snapped to a
/// beat can land a frame or two before or after it.
const SEEK_TOLERANCE_SECS: f64 = 0.002;

/// The sound of the click.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ClickSound {
    /// A short, bright tick.
    #[default]
    Click,
    /// A mechanical metronome: a tick, with a bell on the first beat of the bar.
    Metronome,
    /// A woodblock.
    Wood,
}

impl ClickSound {
    pub const ALL: [ClickSound; 3] = [Self::Click, Self::Metronome, Self::Wood];
}

/// Which output channels the click plays on.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ClickChannels {
    Left,
    #[default]
    Both,
    Right,
}

/// The song's beats, in song frames.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Beats {
    /// Frame where bar 1 starts.
    pub start: f64,
    /// Frames per beat.
    pub beat: f64,
    pub per_bar: u32,
}

impl Beats {
    /// The beats in `range`: their index from bar 1 (negative before it) and frame.
    fn within(&self, range: Range<f64>) -> impl Iterator<Item = (i64, f64)> + '_ {
        let first = ((range.start - self.start) / self.beat).ceil() as i64;
        (first..)
            .map(|index| (index, self.start + index as f64 * self.beat))
            .take_while(move |(_, frame)| *frame < range.end)
    }

    fn accented(&self, index: i64) -> bool {
        index.rem_euclid(i64::from(self.per_bar.max(1))) == 0
    }

    fn bar(&self) -> f64 {
        self.beat * f64::from(self.per_bar.max(1))
    }
}

/// Click settings shared with the UI. A torn read (a new start with an old beat
/// length) lasts one callback at most, which is inaudible.
#[derive(Debug, Default)]
pub(crate) struct ClickParams {
    /// Metronome while playing.
    pub on: AtomicBool,
    /// One bar of clicks before playback starts.
    pub count_in: AtomicBool,
    /// [`Beats`], with `beat` 0 when the song has no grid.
    start: AtomicU64,
    beat: AtomicU64,
    per_bar: AtomicU32,
    pub sound: AtomicU8,
    pub channels: AtomicU8,
    pub gain: AtomicF32,
}

impl ClickParams {
    pub fn set_beats(&self, beats: Option<Beats>) {
        let beats = beats.filter(|b| b.beat > 0.0 && b.beat.is_finite() && b.start.is_finite());
        let Beats {
            start,
            beat,
            per_bar,
        } = beats.unwrap_or(Beats {
            start: 0.0,
            beat: 0.0,
            per_bar: 1,
        });
        self.start.store(start.to_bits(), Ordering::Relaxed);
        self.beat.store(beat.to_bits(), Ordering::Relaxed);
        self.per_bar.store(per_bar.max(1), Ordering::Relaxed);
    }

    pub fn beats(&self) -> Option<Beats> {
        let beat = f64::from_bits(self.beat.load(Ordering::Relaxed));
        (beat > 0.0).then(|| Beats {
            start: f64::from_bits(self.start.load(Ordering::Relaxed)),
            beat,
            per_bar: self.per_bar.load(Ordering::Relaxed),
        })
    }

    pub fn sound(&self) -> ClickSound {
        ClickSound::ALL[usize::from(self.sound.load(Ordering::Relaxed)).min(2)]
    }

    pub fn channels(&self) -> ClickChannels {
        match self.channels.load(Ordering::Relaxed) {
            0 => ClickChannels::Left,
            2 => ClickChannels::Right,
            _ => ClickChannels::Both,
        }
    }
}

pub(crate) fn sound_index(sound: ClickSound) -> u8 {
    match sound {
        ClickSound::Click => 0,
        ClickSound::Metronome => 1,
        ClickSound::Wood => 2,
    }
}

pub(crate) fn channels_index(channels: ClickChannels) -> u8 {
    match channels {
        ClickChannels::Left => 0,
        ClickChannels::Both => 1,
        ClickChannels::Right => 2,
    }
}

/// A count-in in progress: a silent bar before `target`, timed like the song.
#[derive(Clone, Copy, Debug)]
pub(crate) struct CountIn {
    /// Song frame "heard" so far, counting up from one bar before where playback
    /// starts.
    pub heard: f64,
}

/// Plays the click on the audio thread. Nothing here allocates after
/// [`Clicker::new`].
pub(crate) struct Clicker {
    /// Indexed by sound, then accent (1 for the first beat of a bar). Mono.
    sounds: [[Vec<f32>; 2]; 3],
    /// The click sounding now: sound, accent and next sample.
    voice: Option<(usize, usize, usize)>,
    /// Song frame heard at the end of the previous block, while playing.
    last: Option<f64>,
    /// After a seek, beats from here on click once the heard position reaches it.
    waiting: bool,
    /// Beats before bar 1 click up to here: the end of a count-in.
    count_in_end: f64,
    tolerance: f64,
}

impl Clicker {
    pub fn new(sample_rate: u32) -> Self {
        let sounds = ClickSound::ALL.map(|s| [false, true].map(|a| synthesize(s, a, sample_rate)));
        Self {
            sounds,
            voice: None,
            last: None,
            waiting: false,
            count_in_end: f64::NEG_INFINITY,
            tolerance: SEEK_TOLERANCE_SECS * f64::from(sample_rate),
        }
    }

    /// Playback will start at `frame`: beats from there on click.
    pub fn seek(&mut self, frame: usize) {
        self.last = Some(frame as f64 - self.tolerance);
        self.waiting = true;
        self.count_in_end = f64::NEG_INFINITY;
    }

    /// Playback stopped; a click already sounding rings out.
    pub fn stop(&mut self) {
        self.last = None;
    }

    /// Starts a count-in before `target`, or `None` without a grid.
    pub fn count_in(&mut self, beats: Option<Beats>, target: usize) -> Option<CountIn> {
        let beats = beats?;
        self.count_in_end = target as f64;
        Some(CountIn {
            heard: target as f64 - beats.bar(),
        })
    }

    /// The count-in is over and the song is heard from `heard` on.
    pub fn count_in_done(&mut self, heard: f64) {
        self.last = Some(heard);
        self.waiting = false;
    }

    /// Clicks for a block of `out` (interleaved stereo) that played `advanced` song
    /// frames and ended with `heard` being heard.
    pub fn play(
        &mut self,
        params: &ClickParams,
        heard: f64,
        advanced: f64,
        looped: Option<&Range<usize>>,
        out: &mut [f32],
    ) {
        let block = heard - advanced..heard;
        // Beats from `floor` on click.
        let floor = match self.last {
            Some(last) if heard >= last => {
                self.waiting = false;
                last
            }
            // Still filling the stretcher's latency after a seek.
            Some(_) if self.waiting => {
                self.ring(params, out);
                return;
            }
            // Jumped back to the start of a loop.
            _ => block.start.max(looped.map_or(0, |r| r.start) as f64),
        };
        self.last = Some(heard);
        // The metronome starts at bar 1; the end of a count-in may come before it.
        let on = params.on.load(Ordering::Relaxed);
        let count_in_end = self.count_in_end;
        self.render(
            params,
            params.beats(),
            block,
            floor,
            |index, frame| (on && index >= 0) || frame < count_in_end,
            out,
        );
    }

    /// Lets a click already sounding ring out over `out`, starting no new one.
    pub fn ring(&mut self, params: &ClickParams, out: &mut [f32]) {
        self.render(params, None, 0.0..0.0, 0.0, |_, _| false, out);
    }

    /// Clicks for a count-in block over `out`, advancing it by `tempo` song frames
    /// per output frame.
    pub fn play_count_in(
        &mut self,
        params: &ClickParams,
        count_in: &mut CountIn,
        tempo: f32,
        out: &mut [f32],
    ) {
        let from = count_in.heard;
        count_in.heard += (out.len() / 2) as f64 * f64::from(tempo);
        let block = from..count_in.heard;
        self.render(params, params.beats(), block, from, |_, _| true, out);
    }

    /// Adds the click to `out`, which plays the song frames in `block`. A new click
    /// starts at each beat from `floor` to the end of the block that `keep` accepts.
    fn render(
        &mut self,
        params: &ClickParams,
        beats: Option<Beats>,
        block: Range<f64>,
        floor: f64,
        keep: impl Fn(i64, f64) -> bool,
        out: &mut [f32],
    ) {
        let frames = out.len() / 2;
        let gain = params.gain.load();
        let (left, right) = match params.channels() {
            ClickChannels::Left => (gain, 0.0),
            ClickChannels::Both => (gain, gain),
            ClickChannels::Right => (0.0, gain),
        };
        let sound = sound_index(params.sound()) as usize;

        let gains = [left, right];
        let mut done = 0;
        if let Some(beats) = beats
            && block.end > block.start
        {
            let span = block.end - block.start;
            for (index, frame) in beats.within(floor..block.end) {
                if !keep(index, frame) {
                    continue;
                }
                let offset = (frame - block.start) / span * frames as f64;
                let at = (offset.max(0.0).round() as usize).clamp(done, frames);
                self.add_voice(&mut out[done * 2..at * 2], gains);
                done = at;
                self.voice = Some((sound, usize::from(beats.accented(index)), 0));
            }
        }
        self.add_voice(&mut out[done * 2..], gains);
    }

    /// Adds the click sounding now to `out`, as far as it lasts.
    fn add_voice(&mut self, out: &mut [f32], [left, right]: [f32; 2]) {
        let Some((sound, accent, next)) = &mut self.voice else {
            return;
        };
        let samples = &self.sounds[*sound][*accent][*next..];
        let n = (out.len() / 2).min(samples.len());
        for (frame, sample) in out.as_chunks_mut::<2>().0.iter_mut().zip(&samples[..n]) {
            frame[0] += sample * left;
            frame[1] += sample * right;
        }
        *next += n;
        if n == samples.len() {
            self.voice = None;
        }
    }
}

/// One click, mono, peaking at 1.0 when accented and 0.7 otherwise.
fn synthesize(sound: ClickSound, accent: bool, sample_rate: u32) -> Vec<f32> {
    let rate = sample_rate as f32;
    // (frequency, decay time constant in seconds, level)
    let (partials, noise, length): (Vec<(f32, f32, f32)>, f32, f32) = match (sound, accent) {
        (ClickSound::Click, false) => (vec![(2000.0, 0.004, 1.0)], 0.3, 0.04),
        (ClickSound::Click, true) => (vec![(3000.0, 0.004, 1.0)], 0.3, 0.04),
        (ClickSound::Metronome, false) => {
            (vec![(1250.0, 0.008, 1.0), (3400.0, 0.003, 0.6)], 0.5, 0.06)
        }
        // The tick plus a bell, like the bell of a mechanical metronome.
        (ClickSound::Metronome, true) => (
            vec![
                (1250.0, 0.008, 1.0),
                (3400.0, 0.003, 0.6),
                (2093.0, 0.12, 0.5),
                (5776.0, 0.04, 0.15),
            ],
            0.5,
            0.5,
        ),
        // A woodblock: a strong mode and a weaker inharmonic one.
        (ClickSound::Wood, false) => (vec![(880.0, 0.025, 1.0), (2420.0, 0.008, 0.4)], 0.2, 0.15),
        (ClickSound::Wood, true) => (vec![(1175.0, 0.025, 1.0), (3230.0, 0.008, 0.4)], 0.2, 0.15),
    };
    let len = (length * rate) as usize;
    // A noise burst for the strike, 1 ms; deterministic so every click is the same.
    let mut seed = 0x9e37_79b9_u32;
    let mut samples: Vec<f32> = (0..len)
        .map(|i| {
            let t = i as f32 / rate;
            let tone: f32 = partials
                .iter()
                .map(|&(f, decay, level)| level * (TAU * f * t).sin() * (-t / decay).exp())
                .sum();
            seed ^= seed << 13;
            seed ^= seed >> 17;
            seed ^= seed << 5;
            let white = seed as f32 / u32::MAX as f32 * 2.0 - 1.0;
            let strike = noise * white * (-t / 0.001).exp();
            // 0.2 ms attack and a short fade at the end, so neither edge clicks.
            let attack = (t / 0.0002).min(1.0);
            let release = ((len - i) as f32 / (rate * 0.002)).min(1.0);
            (tone + strike) * attack * release
        })
        .collect();
    let peak = samples.iter().fold(0.0f32, |p, s| p.max(s.abs()));
    let level = if accent { 1.0 } else { 0.7 };
    if peak > 0.0 {
        samples.iter_mut().for_each(|s| *s *= level / peak);
    }
    samples
}

#[cfg(test)]
mod tests {
    use super::*;

    const RATE: u32 = 48_000;
    const BLOCK: usize = 256;

    fn params(beats: Beats) -> ClickParams {
        let params = ClickParams {
            gain: AtomicF32::new(1.0),
            ..Default::default()
        };
        params.on.store(true, Ordering::Relaxed);
        params.set_beats(Some(beats));
        params
    }

    /// Where clicks start in `out` (interleaved stereo), and whether each is accented.
    /// A click starts with an exact zero, after a stretch of silence.
    fn clicks(out: &[f32]) -> Vec<(usize, bool)> {
        let left: Vec<f32> = out.iter().step_by(2).copied().collect();
        let mut found = Vec::new();
        let mut zeros = usize::MAX / 2;
        for (i, &s) in left.iter().enumerate() {
            if s == 0.0 {
                zeros += 1;
                continue;
            }
            if zeros >= 200 {
                let peak = left[i..(i + 200).min(left.len())]
                    .iter()
                    .fold(0.0f32, |p, s| p.max(s.abs()));
                found.push((i - 1, peak > 0.85));
            }
            zeros = 0;
        }
        found
    }

    /// Plays `blocks` blocks heard from `from`, `tempo` song frames per output frame.
    fn play(clicker: &mut Clicker, params: &ClickParams, from: f64, tempo: f64) -> Vec<f32> {
        let mut out = vec![0.0; 400 * BLOCK * 2];
        let mut heard = from;
        for block in out.chunks_mut(BLOCK * 2) {
            heard += BLOCK as f64 * tempo;
            clicker.play(params, heard, BLOCK as f64 * tempo, None, block);
        }
        out
    }

    #[test]
    fn beats_from_bar_one() {
        let beats = Beats {
            start: 100.0,
            beat: 50.0,
            per_bar: 3,
        };
        let found: Vec<(i64, f64)> = beats.within(0.0..260.0).collect();
        let expected = [-2, -1, 0, 1, 2, 3].map(|i| (i, 100.0 + i as f64 * 50.0));
        assert_eq!(found, expected);
        assert!(beats.accented(0) && beats.accented(3) && beats.accented(-3));
        assert!(!beats.accented(-1));
    }

    #[test]
    fn clicks_land_on_the_beats_at_any_tempo() {
        // 120 BPM from bar 1 at 0.5 s, in 3/4.
        let beats = Beats {
            start: 24_000.0,
            beat: 24_000.0,
            per_bar: 3,
        };
        for tempo in [0.5, 1.0, 1.25] {
            let params = params(beats);
            let mut clicker = Clicker::new(RATE);
            clicker.seek(0);
            let found = clicks(&play(&mut clicker, &params, 0.0, tempo));
            let expected: Vec<(usize, bool)> = (0..)
                .map(|k| (((24_000 + k * 24_000) as f64 / tempo) as usize, k % 3 == 0))
                .take_while(|&(f, _)| f < 400 * BLOCK - 200)
                .collect();
            assert_eq!(found.len(), expected.len(), "tempo {tempo}: {found:?}");
            for (f, e) in found.iter().zip(&expected) {
                assert!(
                    f.0.abs_diff(e.0) <= 1 && f.1 == e.1,
                    "tempo {tempo}: {f:?} vs {e:?}"
                );
            }
        }
    }

    #[test]
    fn nothing_before_bar_one() {
        let beats = Beats {
            start: 48_000.0,
            beat: 12_000.0,
            per_bar: 4,
        };
        let mut clicker = Clicker::new(RATE);
        clicker.seek(0);
        let found = clicks(&play(&mut clicker, &params(beats), 0.0, 1.0));
        assert_eq!(found[0], (48_000, true));
    }

    #[test]
    fn a_seek_onto_a_beat_clicks_it() {
        let beats = Beats {
            start: 0.0,
            beat: 24_000.0,
            per_bar: 4,
        };
        let mut clicker = Clicker::new(RATE);
        // Snapped a frame past beat 4; the stretcher's latency shows first.
        clicker.seek(72_001);
        let found = clicks(&play(&mut clicker, &params(beats), 72_001.0 - 3000.0, 1.0));
        assert_eq!(found[0], (3000 - 1, false));
    }

    #[test]
    fn loop_jumps_click_from_the_loop_start() {
        let beats = Beats {
            start: 0.0,
            beat: 24_000.0,
            per_bar: 4,
        };
        let params = params(beats);
        let mut clicker = Clicker::new(RATE);
        clicker.seek(30_000);
        let mut out = vec![0.0; 300 * BLOCK * 2];
        let looped = 24_000..60_000;
        let mut heard = 30_000.0;
        for block in out.chunks_mut(BLOCK * 2) {
            heard += BLOCK as f64;
            if heard >= looped.end as f64 {
                heard -= (looped.end - looped.start) as f64;
            }
            clicker.play(&params, heard, BLOCK as f64, Some(&looped), block);
        }
        // Beat 3 at 48 000, then the loop start (beat 2) and beat 3 at every pass,
        // never beat 4 after the loop end.
        let found: Vec<usize> = clicks(&out).iter().map(|c| c.0).collect();
        let expected = [18_000, 30_000, 54_000, 66_000];
        assert_eq!(found.len(), expected.len(), "{found:?}");
        for (f, e) in found.iter().zip(expected) {
            assert!(f.abs_diff(e) <= 1, "{found:?}");
        }
    }

    #[test]
    fn count_in_plays_one_bar() {
        // A bar is 200 blocks, so the count-in ends between two blocks.
        let beats = Beats {
            start: 0.0,
            beat: 12_800.0,
            per_bar: 4,
        };
        let params = params(beats);
        params.on.store(false, Ordering::Relaxed);
        let mut clicker = Clicker::new(RATE);
        let mut count_in = clicker.count_in(Some(beats), 102_400).unwrap();
        let mut out = vec![0.0; 300 * BLOCK * 2];
        for block in out.chunks_mut(BLOCK * 2) {
            if count_in.heard < 102_400.0 {
                clicker.play_count_in(&params, &mut count_in, 1.0, block);
            } else {
                clicker.play(&params, count_in.heard, BLOCK as f64, None, block);
                count_in.heard += BLOCK as f64;
            }
        }
        // Bar 2 of the grid, then nothing: the click is off.
        let found = clicks(&out);
        let expected: Vec<(usize, bool)> = (0..4).map(|k| (k * 12_800, k == 0)).collect();
        assert_eq!(found, expected);
    }

    #[test]
    fn sounds_are_short_and_normalized() {
        for sound in ClickSound::ALL {
            for accent in [false, true] {
                let samples = synthesize(sound, accent, RATE);
                assert!(samples.len() <= RATE as usize / 2);
                let peak = samples.iter().fold(0.0f32, |p, s| p.max(s.abs()));
                let level = if accent { 1.0 } else { 0.7 };
                assert!((peak - level).abs() < 1e-3, "{sound:?} {accent}: {peak}");
                assert_eq!(samples[0], 0.0);
                assert!(samples.last().unwrap().abs() < 0.01);
            }
        }
    }
}
