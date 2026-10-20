//! Separates one song with a GGUF model and writes `stem_N.wav` (32-bit float).
//!
//! ```sh
//! cargo run --release -p watershed-separation --example separate -- model.gguf song.mp3 out-dir [--cpu]
//! ```

use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::time::Instant;

use anyhow::{Context, bail};
use watershed_separation::bsroformer::{Device, LoadOptions, Model};

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let cpu = args.iter().any(|a| a == "--cpu");
    let paths: Vec<&String> = args.iter().filter(|a| !a.starts_with("--")).collect();
    let [model_path, song_path, out_dir] = paths[..] else {
        bail!("usage: separate <model.gguf> <song> <out-dir> [--cpu]");
    };
    let out_dir = PathBuf::from(out_dir);
    std::fs::create_dir_all(&out_dir)?;

    let start = Instant::now();
    let options = LoadOptions {
        device: if cpu { Device::Cpu } else { Device::Auto },
        ..Default::default()
    };
    let mut model = Model::load(Path::new(model_path), &options)?;
    let info = model.info();
    println!(
        "loaded in {:.1} s on {} ({}): {} stems at {} Hz",
        start.elapsed().as_secs_f64(),
        model.backend_name(),
        model.device_description(),
        info.num_stems,
        info.sample_rate
    );

    let mut audio = watershed_dsp::decode_file(Path::new(song_path))
        .with_context(|| format!("decoding {song_path}"))?;
    if audio.sample_rate != info.sample_rate {
        audio = watershed_dsp::resample(audio, info.sample_rate)?;
    }
    println!("song: {:.1} s", audio.duration_secs());

    let start = Instant::now();
    let stems = model.process(
        &audio.samples,
        &|f| eprint!("\r{:5.1} %", f * 100.0),
        &|| false,
    )?;
    eprintln!();
    println!("separated in {:.1} s", start.elapsed().as_secs_f64());

    for (i, stem) in stems.iter().enumerate() {
        let path = out_dir.join(format!("stem_{i}.wav"));
        write_wav_f32(&path, info.sample_rate, stem)?;
    }
    println!("wrote {} stems to {}", stems.len(), out_dir.display());
    Ok(())
}

/// Minimal WAVE_FORMAT_IEEE_FLOAT writer for interleaved stereo.
fn write_wav_f32(path: &Path, sample_rate: u32, samples: &[f32]) -> std::io::Result<()> {
    let data_len = u32::try_from(samples.len() * 4).expect("WAV over 4 GB");
    let mut w = BufWriter::new(File::create(path)?);
    w.write_all(b"RIFF")?;
    w.write_all(&(36 + data_len).to_le_bytes())?;
    w.write_all(b"WAVEfmt ")?;
    w.write_all(&16u32.to_le_bytes())?;
    w.write_all(&3u16.to_le_bytes())?; // IEEE float
    w.write_all(&2u16.to_le_bytes())?; // channels
    w.write_all(&sample_rate.to_le_bytes())?;
    w.write_all(&(sample_rate * 8).to_le_bytes())?; // bytes per second
    w.write_all(&8u16.to_le_bytes())?; // block align
    w.write_all(&32u16.to_le_bytes())?; // bits per sample
    w.write_all(b"data")?;
    w.write_all(&data_len.to_le_bytes())?;
    for s in samples {
        w.write_all(&s.to_le_bytes())?;
    }
    w.flush()
}
