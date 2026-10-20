//! Separates one song with the BS-RoFormer SW GGUF model into a new folder of
//! FLAC stems plus a session, ready to open in the app.
//!
//! ```sh
//! cargo run --release -p watershed-separation --example separate -- model.gguf song.mp3 out-dir [--cpu]
//! ```

use std::path::Path;
use std::sync::Mutex;
use std::time::Instant;

use anyhow::bail;
use watershed_separation::bsroformer::{Device, LoadOptions};
use watershed_separation::{BS_ROFORMER_SW_STEMS, GgufSeparator, Separator, Stage};

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let cpu = args.iter().any(|a| a == "--cpu");
    let paths: Vec<&String> = args.iter().filter(|a| !a.starts_with("--")).collect();
    let [model_path, song_path, out_dir] = paths[..] else {
        bail!("usage: separate <model.gguf> <song> <out-dir> [--cpu]");
    };

    let start = Instant::now();
    let options = LoadOptions {
        device: if cpu { Device::Cpu } else { Device::Auto },
        ..Default::default()
    };
    let names = BS_ROFORMER_SW_STEMS.map(String::from).to_vec();
    let mut separator = GgufSeparator::load(Path::new(model_path), names, &options)?;
    println!(
        "loaded in {:.1} s on {}: {} stems at {} Hz",
        start.elapsed().as_secs_f64(),
        separator.device(),
        separator.stem_names().len(),
        separator.sample_rate()
    );

    let start = Instant::now();
    let stage_start = Mutex::new((Stage::Decoding, Instant::now()));
    let separated = watershed_separation::separate_song(
        &mut separator,
        Path::new(song_path),
        Path::new(out_dir),
        &|stage, f| {
            let mut current = stage_start.lock().unwrap();
            if current.0 != stage {
                eprintln!(
                    "\r{:?}: {:.1} s",
                    current.0,
                    current.1.elapsed().as_secs_f64()
                );
                *current = (stage, Instant::now());
            }
            eprint!("\r{stage:?} {:5.1} %", f * 100.0);
        },
        &|| false,
    )?;
    let (stage, since) = *stage_start.lock().unwrap();
    eprintln!("\r{stage:?}: {:.1} s      ", since.elapsed().as_secs_f64());

    println!(
        "done in {:.1} s, stem gain {:.1} dB: {} files in {}",
        start.elapsed().as_secs_f64(),
        20.0 * separated.stem_gain.log10(),
        separated.files.len(),
        separated.folder.display()
    );
    Ok(())
}
