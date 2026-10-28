//! Tracks the beats of some audio files, as if they were the stems of one song, and
//! prints how the local tempo moves: `cargo run --release -p watershed-dsp --example
//! beats -- [--bpm 138] drums.flac bass.flac ...`. Files named `vocals` weigh less.

use std::time::Instant;

use watershed_dsp::{OnsetEnvelope, VOCALS_TEMPO_WEIGHT, decode_file};

/// Beats per printed line of the tempo curve.
const BEATS_PER_LINE: usize = 16;

fn main() {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    let mut bpm = None;
    if let Some(i) = args.iter().position(|a| a == "--bpm") {
        bpm = args.get(i + 1).and_then(|v| v.parse::<f64>().ok());
        args.drain(i..(i + 2).min(args.len()));
    }
    if args.is_empty() {
        eprintln!("usage: beats [--bpm <bpm>] <audio file>...");
        std::process::exit(2);
    }
    let mut envelope = OnsetEnvelope::default();
    for file in &args {
        match decode_file(file.as_ref()) {
            Ok(audio) => {
                // As the app does, by file name only.
                let weight = if file.to_lowercase().contains("vocals") {
                    VOCALS_TEMPO_WEIGHT
                } else {
                    1.0
                };
                envelope.add_weighted(
                    &OnsetEnvelope::compute(&audio.samples, audio.sample_rate),
                    weight,
                );
            }
            Err(e) => {
                eprintln!("{file}: {e}");
                std::process::exit(1);
            }
        }
    }
    let Some(bpm) = bpm.or_else(|| envelope.bpm()) else {
        println!("no steady beat found");
        return;
    };
    let start = Instant::now();
    let beats = envelope.track_beats(bpm);
    let elapsed = start.elapsed();
    println!(
        "{bpm:.1} BPM, {} beats tracked in {elapsed:.1?}",
        beats.len()
    );

    if let Some(end) = envelope.rhythm_end() {
        let length = envelope.values.len() as f64 / envelope.rate;
        println!(
            "rhythm ends at {}:{:04.1} of {}:{:04.1}",
            (end / 60.0) as u32,
            end % 60.0,
            (length / 60.0) as u32,
            length % 60.0
        );
    }
    let tempos: Vec<f64> = beats.windows(2).map(|w| 60.0 / (w[1] - w[0])).collect();
    if tempos.is_empty() {
        return;
    }
    let mut sorted = tempos.clone();
    sorted.sort_by(f64::total_cmp);
    let at = |q: f64| sorted[((sorted.len() - 1) as f64 * q).round() as usize];
    let (p5, median, p95) = (at(0.05), at(0.5), at(0.95));
    println!(
        "local tempo: p5 {p5:.1}, median {median:.1}, p95 {p95:.1} BPM; spread {:.1} %",
        (p95 - p5) / median * 100.0
    );
    // How far the tracked beats are from a steady grid at `bpm`, from its first beat.
    let first = envelope.first_beat(bpm);
    let beat = 60.0 / bpm;
    println!("  time     tempo   vs steady grid");
    for (line, chunk) in tempos.chunks(BEATS_PER_LINE).enumerate() {
        let mean = chunk.iter().sum::<f64>() / chunk.len() as f64;
        let from = line * BEATS_PER_LINE;
        let secs = beats[from];
        let offset = first.map_or(String::new(), |first| {
            let offsets: Vec<f64> = beats[from..(from + BEATS_PER_LINE).min(beats.len())]
                .iter()
                .filter(|&&t| t >= first - beat / 2.0)
                .map(|&t| (t - first - ((t - first) / beat).round() * beat) * 1000.0)
                .collect();
            if offsets.is_empty() {
                return String::new();
            }
            format!(
                "{:+6.0} ms",
                offsets.iter().sum::<f64>() / offsets.len() as f64
            )
        });
        println!(
            "  {:2}:{:04.1}  {mean:6.1}  {offset}",
            (secs / 60.0) as u32,
            secs % 60.0
        );
    }
}
