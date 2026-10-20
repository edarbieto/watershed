//! Separates one song with the catalog's default model into a new folder of FLAC
//! stems plus a session, ready to open in the app. The model is downloaded into
//! the models cache folder on first use, unless `--model` points to a local copy.
//!
//! ```sh
//! cargo run --release -p watershed-separation --example separate -- song.mp3 out-dir [--model file.gguf] [--cpu]
//! ```

use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Instant;

use anyhow::{Context, bail};
use watershed_separation::bsroformer::{Device, LoadOptions};
use watershed_separation::{GgufSeparator, Separator, Stage, catalog};

fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);
    let (mut cpu, mut model_file, mut paths) = (false, None, Vec::new());
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--cpu" => cpu = true,
            "--model" => {
                model_file = Some(PathBuf::from(args.next().context("--model needs a file")?))
            }
            _ => paths.push(arg),
        }
    }
    let [song_path, out_dir] = &paths[..] else {
        bail!("usage: separate <song> <out-dir> [--model file.gguf] [--cpu]");
    };

    let entry = catalog::default_model();
    let model_path = match model_file {
        Some(path) => path,
        None => {
            let dir = watershed_storage::models_dir().context("no cache folder on this system")?;
            if !entry.is_installed(&dir) {
                println!("downloading {} to {}", entry.name, dir.display());
            }
            let path = entry.install(
                &dir,
                &|done, total| {
                    eprint!("\r{:6.1} / {:.1} MB", done as f64 / 1e6, total as f64 / 1e6)
                },
                &|| false,
            )?;
            eprintln!();
            path
        }
    };

    let start = Instant::now();
    let options = LoadOptions {
        device: if cpu { Device::Cpu } else { Device::Auto },
        ..Default::default()
    };
    let mut separator = GgufSeparator::load(&model_path, entry.stems.clone(), &options)?;
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
