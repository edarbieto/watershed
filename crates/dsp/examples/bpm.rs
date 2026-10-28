//! Prints the estimated tempo of some audio files, as if they were the stems of one
//! song: `cargo run --release -p watershed-dsp --example bpm -- drums.flac`.

use std::time::Instant;

use watershed_dsp::{OnsetEnvelope, decode_file};

fn main() {
    let files: Vec<String> = std::env::args().skip(1).collect();
    if files.is_empty() {
        eprintln!("usage: bpm <audio file>...");
        std::process::exit(2);
    }
    let mut envelope = OnsetEnvelope::default();
    for file in &files {
        let audio = match decode_file(file.as_ref()) {
            Ok(audio) => audio,
            Err(e) => {
                eprintln!("{file}: {e}");
                std::process::exit(1);
            }
        };
        let start = Instant::now();
        let own = OnsetEnvelope::compute(&audio.samples, audio.sample_rate);
        let elapsed = start.elapsed();
        match own.bpm() {
            Some(bpm) => println!("{bpm:6.1} BPM  {file} ({elapsed:.0?})"),
            None => println!("     -      {file} ({elapsed:.0?})"),
        }
        envelope.add(&own);
    }
    if files.len() > 1 {
        match envelope.bpm() {
            Some(bpm) => println!("{bpm:6.1} BPM  all"),
            None => println!("     -      all"),
        }
    }
}
