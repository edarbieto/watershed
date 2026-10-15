//! Decodes audio files and prints what was found.
//!
//! `cargo run --release -p watershed-dsp --example probe -- FILE [FILE...]`

use std::path::PathBuf;
use std::time::Instant;

fn main() {
    for path in std::env::args_os().skip(1).map(PathBuf::from) {
        let start = Instant::now();
        match watershed_dsp::decode_file(&path) {
            Ok(audio) => {
                let peak = audio.samples.iter().fold(0.0f32, |m, s| m.max(s.abs()));
                println!(
                    "{}: {} Hz, {} frames ({:.2} s), peak {peak:.3}, decoded in {:?}",
                    path.display(),
                    audio.sample_rate,
                    audio.frames(),
                    audio.duration_secs(),
                    start.elapsed(),
                );
            }
            Err(e) => println!("{}: {e}", path.display()),
        }
    }
}
