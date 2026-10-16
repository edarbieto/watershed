//! Spike: Signalsmith Stretch on the stereo mix. Measures CPU cost and latency, writes
//! WAV renders to listen to, and can play the mix live with tempo and pitch control.
//!
//! ```text
//! cargo run --release -p watershed-audio --example stretch -- [OPTIONS] FILE...
//!
//!   --rate R          sample rate for the benchmark (default 48000)
//!   --block N         frames per process call, like an audio callback (default 512)
//!   --secs S          only use the first S seconds of the song (default: all)
//!   --wav DIR         write a WAV per benchmark case into DIR
//!   --play            play live instead of benchmarking; type commands on stdin:
//!                     `t 0.75` tempo, `p -1` pitch in semitones, `q` quit
//!   --cheaper         use the cheaper preset when playing
//! ```
//!
//! The stems are summed at unity gain, which roughly rebuilds the original song.

use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Instant;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use signalsmith_stretch::Stretch;
use watershed_audio::AtomicF32;

#[derive(Clone, Copy)]
enum Preset {
    Default,
    Cheaper,
}

impl Preset {
    fn build(self, sample_rate: u32) -> Stretch {
        match self {
            Preset::Default => Stretch::preset_default(2, sample_rate),
            Preset::Cheaper => Stretch::preset_cheaper(2, sample_rate),
        }
    }

    fn name(self) -> &'static str {
        match self {
            Preset::Default => "default",
            Preset::Cheaper => "cheaper",
        }
    }
}

fn main() {
    let mut args = std::env::args().skip(1);
    let mut rate = 48_000;
    let mut block = 512;
    let mut secs = None;
    let mut wav_dir = None;
    let mut play = false;
    let mut preset = Preset::Default;
    let mut files = Vec::new();
    while let Some(arg) = args.next() {
        let mut value = || args.next().unwrap_or_else(|| panic!("{arg} needs a value"));
        match arg.as_str() {
            "--rate" => rate = value().parse().expect("--rate N"),
            "--block" => block = value().parse().expect("--block N"),
            "--secs" => secs = Some(value().parse::<f64>().expect("--secs S")),
            "--wav" => wav_dir = Some(PathBuf::from(value())),
            "--play" => play = true,
            "--cheaper" => preset = Preset::Cheaper,
            _ => files.push(arg),
        }
    }
    assert!(!files.is_empty(), "give at least one audio file");

    if play {
        let device = cpal::default_host()
            .default_output_device()
            .expect("no output device");
        rate = device.default_output_config().unwrap().sample_rate();
    }
    let mut mix = load_mix(&files, rate);
    if let Some(secs) = secs {
        mix.truncate(((secs * f64::from(rate)) as usize * 2).min(mix.len()));
    }
    println!(
        "mix: {} stems, {:.1} s at {rate} Hz",
        files.len(),
        mix.len() as f64 / 2.0 / f64::from(rate)
    );

    if play {
        play_live(mix, rate, preset);
    } else {
        benchmark(&mix, rate, block, wav_dir.as_deref());
    }
}

fn load_mix(files: &[String], rate: u32) -> Vec<f32> {
    let mut mix: Vec<f32> = Vec::new();
    for file in files {
        let audio = watershed_dsp::decode_file(file.as_ref()).expect("decode failed");
        let audio = watershed_dsp::resample(audio, rate).expect("resample failed");
        if mix.len() < audio.samples.len() {
            mix.resize(audio.samples.len(), 0.0);
        }
        for (m, s) in mix.iter_mut().zip(&audio.samples) {
            *m += s;
        }
    }
    mix
}

/// Result of stretching a whole song in callback-sized blocks.
struct Run {
    output: Vec<f32>,
    /// Wall time of each `process` call, in seconds.
    times: Vec<f64>,
}

/// Feeds `input` through `stretch` like an audio callback would: every call produces
/// `block` output frames and consumes `block * tempo` input frames on average.
fn run(stretch: &mut Stretch, input: &[f32], block: usize, tempo: f64) -> Run {
    let in_frames = input.len() / 2;
    let out_frames = (in_frames as f64 / tempo) as usize;
    let mut output = vec![0.0; out_frames * 2];
    let mut times = Vec::with_capacity(out_frames / block + 1);
    let mut read = 0usize;
    let mut written = 0usize;
    while written + block <= out_frames {
        // Where the input should be after this block, rounded so it never drifts.
        let target = (((written + block) as f64 * tempo).round() as usize).min(in_frames);
        let start = Instant::now();
        stretch.process(
            &input[read * 2..target * 2],
            &mut output[written * 2..(written + block) * 2],
        );
        times.push(start.elapsed().as_secs_f64());
        read = target;
        written += block;
    }
    output.truncate(written * 2);
    Run { output, times }
}

fn benchmark(mix: &[f32], rate: u32, block: usize, wav_dir: Option<&Path>) {
    let budget = block as f64 / f64::from(rate);
    println!(
        "block {block} frames = {:.2} ms budget per call\n",
        budget * 1e3
    );

    for preset in [Preset::Default, Preset::Cheaper] {
        let stretch = preset.build(rate);
        println!(
            "preset {}: input latency {} frames ({:.1} ms), output latency {} frames ({:.1} ms)",
            preset.name(),
            stretch.input_latency(),
            stretch.input_latency() as f64 / f64::from(rate) * 1e3,
            stretch.output_latency(),
            stretch.output_latency() as f64 / f64::from(rate) * 1e3,
        );
    }
    println!();
    println!(
        "{:<8} {:>6} {:>6} {:>8} {:>9} {:>9} {:>9}",
        "preset", "tempo", "semi", "cpu %", "mean ms", "p99 ms", "max ms"
    );

    let cases = [
        (1.0, 0.0),
        (0.5, 0.0),
        (0.75, 0.0),
        (1.25, 0.0),
        (1.0, -1.0),
        (1.0, 2.0),
        (0.75, -1.0),
    ];
    for preset in [Preset::Default, Preset::Cheaper] {
        for (tempo, semitones) in cases {
            let mut stretch = preset.build(rate);
            stretch.set_transpose_factor_semitones(semitones, None);
            let run = run(&mut stretch, mix, block, tempo);

            let mut sorted = run.times.clone();
            sorted.sort_by(f64::total_cmp);
            let total: f64 = sorted.iter().sum();
            let mean = total / sorted.len() as f64;
            let p99 = sorted[sorted.len() * 99 / 100];
            let max = *sorted.last().unwrap();
            let audio_secs = run.output.len() as f64 / 2.0 / f64::from(rate);
            println!(
                "{:<8} {:>6.2} {:>+6.1} {:>8.2} {:>9.3} {:>9.3} {:>9.3}",
                preset.name(),
                tempo,
                semitones,
                total / audio_secs * 100.0,
                mean * 1e3,
                p99 * 1e3,
                max * 1e3,
            );

            if let Some(dir) = wav_dir {
                std::fs::create_dir_all(dir).expect("cannot create WAV dir");
                let name = format!("{}_t{tempo:.2}_p{semitones:+.0}.wav", preset.name());
                write_wav(&dir.join(name), rate, &run.output).expect("cannot write WAV");
            }
        }
    }
}

/// Writes interleaved stereo `f32` samples as a 32-bit float WAV.
fn write_wav(path: &Path, rate: u32, samples: &[f32]) -> std::io::Result<()> {
    let data_len = (samples.len() * 4) as u32;
    let mut w = std::io::BufWriter::new(std::fs::File::create(path)?);
    w.write_all(b"RIFF")?;
    w.write_all(&(36 + data_len).to_le_bytes())?;
    w.write_all(b"WAVEfmt ")?;
    w.write_all(&16u32.to_le_bytes())?;
    w.write_all(&3u16.to_le_bytes())?; // IEEE float
    w.write_all(&2u16.to_le_bytes())?;
    w.write_all(&rate.to_le_bytes())?;
    w.write_all(&(rate * 8).to_le_bytes())?;
    w.write_all(&8u16.to_le_bytes())?;
    w.write_all(&32u16.to_le_bytes())?;
    w.write_all(b"data")?;
    w.write_all(&data_len.to_le_bytes())?;
    for s in samples {
        w.write_all(&s.to_le_bytes())?;
    }
    w.flush()
}

/// Controls shared between stdin and the audio callback.
struct Live {
    tempo: AtomicF32,
    semitones: AtomicF32,
    done: AtomicBool,
    /// Callback statistics, in nanoseconds.
    max_ns: AtomicU64,
    total_ns: AtomicU64,
    frames: AtomicU64,
}

fn play_live(mix: Vec<f32>, rate: u32, preset: Preset) {
    let device = cpal::default_host().default_output_device().unwrap();
    let supported = device.default_output_config().unwrap();
    assert_eq!(
        supported.sample_format(),
        cpal::SampleFormat::F32,
        "the spike only handles f32 output"
    );
    let config = supported.config();
    let channels = usize::from(config.channels);

    let live = Arc::new(Live {
        tempo: AtomicF32::new(1.0),
        semitones: AtomicF32::new(0.0),
        done: AtomicBool::new(false),
        max_ns: AtomicU64::new(0),
        total_ns: AtomicU64::new(0),
        frames: AtomicU64::new(0),
    });

    // Everything the callback touches is allocated here.
    let mut stretch = preset.build(rate);
    let mut scratch = vec![0.0f32; 8192 * 2];
    let mut position = 0.0f64;
    let mut last_semitones = 0.0;
    let state = live.clone();

    let stream = device
        .build_output_stream(
            config,
            move |out: &mut [f32], _| {
                let start = Instant::now();
                let tempo = f64::from(state.tempo.load());
                let semitones = state.semitones.load();
                if semitones != last_semitones {
                    stretch.set_transpose_factor_semitones(semitones, None);
                    last_semitones = semitones;
                }

                let frames = out.len() / channels;
                let in_frames = mix.len() / 2;
                let from = (position as usize).min(in_frames);
                position += frames as f64 * tempo;
                let to = (position as usize).min(in_frames);
                if from == in_frames {
                    state.done.store(true, Ordering::Relaxed);
                }

                let stereo = &mut scratch[..frames * 2];
                stretch.process(&mix[from * 2..to * 2], &mut *stereo);
                for (o, s) in out
                    .chunks_exact_mut(channels)
                    .zip(stereo.as_chunks::<2>().0)
                {
                    o[0] = s[0] * 0.5;
                    if channels > 1 {
                        o[1] = s[1] * 0.5;
                    }
                }

                let ns = start.elapsed().as_nanos() as u64;
                state.max_ns.fetch_max(ns, Ordering::Relaxed);
                state.total_ns.fetch_add(ns, Ordering::Relaxed);
                state.frames.fetch_add(frames as u64, Ordering::Relaxed);
            },
            |e| eprintln!("stream error: {e}"),
            None,
        )
        .expect("cannot open stream");
    stream.play().unwrap();
    println!("playing at {rate} Hz, preset {}", preset.name());
    println!("commands: `t 0.75` tempo, `p -1` pitch in semitones, `s` stats, `q` quit");

    let print_stats = |live: &Live| {
        let frames = live.frames.load(Ordering::Relaxed);
        let secs = frames as f64 / f64::from(rate);
        let busy = live.total_ns.load(Ordering::Relaxed) as f64 / 1e9;
        println!(
            "{secs:.1} s played, cpu {:.2} % of one core, slowest callback {:.3} ms",
            busy / secs * 100.0,
            live.max_ns.load(Ordering::Relaxed) as f64 / 1e6
        );
    };

    for line in std::io::stdin().lock().lines() {
        let line = line.unwrap();
        let mut parts = line.split_whitespace();
        match (
            parts.next(),
            parts.next().and_then(|v| v.parse::<f32>().ok()),
        ) {
            (Some("t"), Some(v)) => live.tempo.store(v.clamp(0.25, 2.0)),
            (Some("p"), Some(v)) => live.semitones.store(v.clamp(-12.0, 12.0)),
            (Some("s"), _) => print_stats(&live),
            (Some("q"), _) => break,
            _ => println!("?"),
        }
        if live.done.load(Ordering::Relaxed) {
            break;
        }
    }
    print_stats(&live);
}
