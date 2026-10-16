//! Plays a set of stems on the default output device.
//!
//! `cargo run --release -p watershed-audio --example play -- [--gain G] [--secs S] FILE...`

use std::sync::Arc;
use std::time::{Duration, Instant};

use watershed_audio::{Engine, Song};

fn main() {
    let mut args = std::env::args().skip(1).peekable();
    let mut gain = 0.5;
    let mut secs = 5.0;
    let mut files = Vec::new();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--gain" => gain = args.next().and_then(|v| v.parse().ok()).expect("--gain G"),
            "--secs" => secs = args.next().and_then(|v| v.parse().ok()).expect("--secs S"),
            _ => files.push(arg),
        }
    }

    let mut engine =
        Engine::start(None, std::sync::Arc::new(|| {})).expect("cannot open output device");
    let rate = engine.sample_rate();
    println!("device: {} at {rate} Hz", engine.device().name);

    let tracks = files
        .iter()
        .map(|f| {
            let audio = watershed_dsp::decode_file(f.as_ref()).expect("decode failed");
            let audio = watershed_dsp::resample(audio, rate).expect("resample failed");
            (f.clone(), audio.samples)
        })
        .collect();
    let song = Arc::new(Song::new(rate, tracks));
    println!(
        "loaded {} tracks, {:.1} s",
        song.tracks.len(),
        song.duration_secs()
    );

    engine.set_master_gain(gain);
    engine.load(song);
    engine.play();
    let start = Instant::now();
    std::thread::sleep(Duration::from_secs_f64(secs));
    let position = engine.position();
    println!(
        "played {position} frames in {:.2} s ({:.2} s of audio), failed: {}",
        start.elapsed().as_secs_f64(),
        position as f64 / f64::from(rate),
        engine.failed()
    );
}
