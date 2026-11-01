//! Separates a whole song and compares the stems with a folder of reference stems from
//! `tools/reference/separate.py`, which must have separated the very same audio.
//!
//! ```sh
//! cargo run --release -p watershed-roformer --example song -- [--full] [--out DIR] \
//!     CHECKPOINT SONG REFERENCE_DIR
//! ```
//! `--full` computes in f32 instead of f16. `--out` also writes the stems as FLAC.

use std::path::{Path, PathBuf};
use std::time::Instant;

use anyhow::{Context, Result};
use watershed_dsp::{decode_file, gain_to_fit, resample, write_flac};
use watershed_roformer::{BsRoformer, Config, Precision, Weights, default_device};

/// In the order of the model's outputs.
const STEMS: [&str; 6] = ["bass", "drums", "other", "vocals", "guitar", "piano"];
const SAMPLE_RATE: u32 = 44100;

/// Signal to distortion ratio in dB, after fitting one gain: the reference stems were
/// scaled to fit in FLAC.
fn sdr(got: &[f32], reference: &[f32]) -> f64 {
    let dot: f64 = got
        .iter()
        .zip(reference)
        .map(|(a, b)| f64::from(*a) * f64::from(*b))
        .sum();
    let energy: f64 = reference.iter().map(|v| f64::from(*v).powi(2)).sum();
    let gain = dot / energy;
    let error: f64 = got
        .iter()
        .zip(reference)
        .map(|(a, b)| (f64::from(*a) - gain * f64::from(*b)).powi(2))
        .sum();
    10.0 * (gain * gain * energy / error).log10()
}

fn rms_db(samples: &[f32]) -> f64 {
    let energy: f64 = samples.iter().map(|v| f64::from(*v).powi(2)).sum();
    10.0 * (energy / samples.len() as f64).log10()
}

fn main() -> Result<()> {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    let precision = match args.iter().position(|a| a == "--full") {
        Some(i) => {
            args.remove(i);
            Precision::Full
        }
        None => Precision::Half,
    };
    let out = args.iter().position(|a| a == "--out").map(|i| {
        args.remove(i);
        PathBuf::from(args.remove(i))
    });
    let [checkpoint, song, reference] = args.as_slice() else {
        anyhow::bail!("usage: song [--full] [--out DIR] CHECKPOINT SONG REFERENCE_DIR");
    };

    let device = default_device();
    let config = Config::bs_roformer_sw();
    let model = BsRoformer::load(&Weights::open(checkpoint)?, &config, precision, &device)?;
    let input = resample(decode_file(Path::new(song))?, SAMPLE_RATE)?;
    println!("{:.1} s of audio", input.duration_secs());

    let start = Instant::now();
    let stems = model.separate(&input);
    println!("separated in {:.1} s", start.elapsed().as_secs_f32());

    for (name, stem) in STEMS.iter().zip(&stems) {
        let path = Path::new(reference).join(format!("{name}.flac"));
        let expected = decode_file(&path).with_context(|| path.display().to_string())?;
        anyhow::ensure!(expected.frames() == stem.frames(), "{name}: other length");
        println!(
            "{name:>7}: SDR {:6.2} dB   (stem at {:6.1} dB RMS)",
            sdr(&stem.samples, &expected.samples),
            rms_db(&stem.samples),
        );
    }

    if let Some(out) = out {
        std::fs::create_dir_all(&out)?;
        let gain = gain_to_fit(&stems);
        for (name, stem) in STEMS.iter().zip(&stems) {
            write_flac(&out.join(format!("{name}.flac")), stem, gain)?;
        }
    }
    Ok(())
}
