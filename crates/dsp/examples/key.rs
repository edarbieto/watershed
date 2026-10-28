//! Prints the estimated key of some audio files, as if they were the pitched stems of
//! one song: `cargo run --release -p watershed-dsp --example key -- bass.flac other.flac`.

use std::time::Instant;

use watershed_dsp::{Chroma, decode_file};

fn main() {
    let files: Vec<String> = std::env::args().skip(1).collect();
    if files.is_empty() {
        eprintln!("usage: key <audio file>...");
        std::process::exit(2);
    }
    let mut chroma = Chroma::default();
    for file in &files {
        let audio = match decode_file(file.as_ref()) {
            Ok(audio) => audio,
            Err(e) => {
                eprintln!("{file}: {e}");
                std::process::exit(1);
            }
        };
        let start = Instant::now();
        let own = Chroma::compute(&audio.samples, audio.sample_rate);
        let elapsed = start.elapsed();
        match own.key() {
            Some(k) => println!(
                "{:>4} ({:.2}, then {:>3})  {file} ({elapsed:.0?})",
                k.key.to_string(),
                k.confidence,
                k.second.to_string()
            ),
            None => println!("   -  {file} ({elapsed:.0?})"),
        }
        chroma.add(&own);
    }
    if let Some(k) = chroma.key() {
        println!(
            "{:>4} ({:.2}, then {:>3}, tuning {:+.0} cents)  all",
            k.key.to_string(),
            k.confidence,
            k.second.to_string(),
            k.tuning_cents
        );
        let names = [
            "C", "C#", "D", "Eb", "E", "F", "F#", "G", "G#", "A", "Bb", "B",
        ];
        let classes = chroma.pitch_classes();
        let max = classes.iter().copied().fold(0.0, f32::max);
        for (name, value) in names.iter().zip(classes) {
            println!("  {name:>2} {}", "#".repeat((value / max * 40.0) as usize));
        }
    }
}
