//! The model against PyTorch's, layer by layer.
//!
//! The tiny model runs everywhere, on the CPU. The real one needs a GPU and the local
//! files of `tools/reference`:
//!
//! ```sh
//! cargo test --release -p watershed-roformer --test equivalence -- --ignored --nocapture
//! ```

mod reference;

use std::path::Path;

use anyhow::Result;
use burn::prelude::*;
use safetensors::SafeTensors;
use watershed_dsp::{StereoBuffer, Stft};
use watershed_roformer::{BsRoformer, Config, Precision, Weights, default_device};

/// Largest relative error allowed per layer in f32.
const LIMIT: f64 = 1e-3;

/// Largest relative error allowed on audio, in f32: 80 dB under the signal.
const AUDIO_LIMIT: f64 = 1e-4;

/// As in `tools/reference/make_tiny_fixture.py`.
fn tiny_config() -> Config {
    Config {
        dim: 16,
        depth: 2,
        num_stems: 2,
        heads: 2,
        dim_head: 8,
        time_transformer_depth: 1,
        freq_transformer_depth: 1,
        freqs_per_bands: vec![2, 2, 4, 8, 17],
        mask_estimator_depth: 2,
        mlp_expansion_factor: 4,
        stft_n_fft: 64,
        stft_hop_length: 16,
        chunk_size: 496,
        num_overlap: 2,
    }
}

fn check(checkpoint: &Path, dump: &Path, config: &Config, device: &Device) -> Result<()> {
    let weights = Weights::open(checkpoint)?;
    let model = BsRoformer::load(&weights, config, Precision::Full, device)?;
    let bytes = std::fs::read(dump)?;
    let dump = SafeTensors::deserialize(&bytes)?;

    let errors = reference::layer_errors(&model, &dump, config.num_stems, device)?;
    for (name, error) in &errors {
        println!("{name:>12}: {error:.2e}");
    }
    // The band split, a time and a frequency transformer per block, the norm, the masks.
    assert_eq!(errors.len(), 3 + 2 * config.depth);
    let over: Vec<_> = errors
        .iter()
        .filter(|(_, e)| e.is_nan() || *e > LIMIT)
        .collect();
    assert!(over.is_empty(), "over {LIMIT:e}: {over:?}");
    Ok(())
}

fn fixture(name: &str) -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

#[test]
fn tiny_model_matches_pytorch() -> Result<()> {
    check(
        &fixture("tiny.ckpt"),
        &fixture("tiny.safetensors"),
        &tiny_config(),
        &Device::flex(),
    )
}

fn tiny_model(config: &Config) -> Result<BsRoformer> {
    let weights = Weights::open(fixture("tiny.ckpt"))?;
    Ok(BsRoformer::load(
        &weights,
        config,
        Precision::Full,
        &Device::flex(),
    )?)
}

/// (channels, time) stored planar, as interleaved stereo.
fn interleave(planar: &[f32]) -> Vec<f32> {
    let (left, right) = planar.split_at(planar.len() / 2);
    left.iter().zip(right).flat_map(|(l, r)| [*l, *r]).collect()
}

#[test]
fn stft_matches_torch() -> Result<()> {
    let bytes = std::fs::read(fixture("tiny.safetensors"))?;
    let dump = SafeTensors::deserialize(&bytes)?;
    let (audio, _) = reference::values(&dump, "audio")?;
    let (expected, shape) = reference::values(&dump, "stft")?;
    let [_, freqs, frames, _] = shape.as_slice().try_into()?;

    let stft = Stft::new(64, 16);
    let (left, right) = audio.split_at(audio.len() / 2);
    let spectra = [stft.forward(left), stft.forward(right)];
    assert_eq!(stft.freqs() * 2, freqs);
    assert_eq!(stft.frames(left.len()), frames);

    let mut got = Vec::with_capacity(expected.len());
    for freq in 0..stft.freqs() {
        for spectrum in &spectra {
            for frame in 0..frames {
                let bin = spectrum[frame * stft.freqs() + freq];
                got.extend([bin.re, bin.im]);
            }
        }
    }
    let error = reference::error(&got, &expected);
    assert!(error < AUDIO_LIMIT, "{error:e}");
    Ok(())
}

/// The whole chunk: STFT, masks, zeroed DC and inverse STFT, against the model's output.
#[test]
fn chunk_matches_pytorch() -> Result<()> {
    let config = tiny_config();
    let model = tiny_model(&config)?;
    let bytes = std::fs::read(fixture("tiny.safetensors"))?;
    let dump = SafeTensors::deserialize(&bytes)?;
    let (audio, _) = reference::values(&dump, "audio")?;
    let (output, _) = reference::values(&dump, "output")?;

    let stems = model.separate_chunk(&interleave(&audio));
    let per_stem = output.len() / config.num_stems;
    for (stem, expected) in stems.iter().zip(output.chunks_exact(per_stem)) {
        let error = reference::error(stem, &interleave(expected));
        assert!(error < AUDIO_LIMIT, "{error:e}");
    }
    Ok(())
}

/// Whole songs against MSST's `demix`. The cases are explained in
/// `tools/reference/make_tiny_fixture.py`.
#[test]
fn songs_match_demix() -> Result<()> {
    let bytes = std::fs::read(fixture("tiny.safetensors"))?;
    let dump = SafeTensors::deserialize(&bytes)?;

    for (length, num_overlap) in [(200, 2), (400, 2), (1000, 2), (1000, 4), (992, 1)] {
        let config = Config {
            num_overlap,
            ..tiny_config()
        };
        let model = tiny_model(&config)?;
        let (song, _) = reference::values(&dump, &format!("song_{length}_{num_overlap}"))?;
        let (output, _) = reference::values(&dump, &format!("stems_{length}_{num_overlap}"))?;

        let stems = model.separate(&StereoBuffer::new(44100, interleave(&song)));
        let per_stem = output.len() / config.num_stems;
        for (stem, expected) in stems.iter().zip(output.chunks_exact(per_stem)) {
            assert_eq!(stem.frames(), length);
            let error = reference::error(&stem.samples, &interleave(expected));
            assert!(error < AUDIO_LIMIT, "{length}, {num_overlap}: {error:e}");
        }
    }
    Ok(())
}

/// Fails if the band split goes to f16 with PyTorch's epsilon: silence then gives NaN.
#[test]
fn quiet_input_stays_finite_in_f16() -> Result<()> {
    let device = Device::flex();
    let config = tiny_config();
    let weights = Weights::open(fixture("tiny.ckpt"))?;
    let model = BsRoformer::load(&weights, &config, Precision::Half, &device)?;
    let bytes = std::fs::read(fixture("tiny.safetensors"))?;
    let dump = SafeTensors::deserialize(&bytes)?;

    for gain in [0.0, 1e-3, 1e-5] {
        let masks = reference::floats(model.forward(reference::stft(&dump, &device)? * gain));
        assert!(masks.iter().all(|v| v.is_finite()), "input × {gain}");
    }
    Ok(())
}

#[test]
#[ignore = "needs a GPU and the local files of tools/reference"]
fn real_model_matches_pytorch() -> Result<()> {
    let tools = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tools/reference");
    check(
        &tools.join("models/BS-Rofo-SW-Fixed.ckpt"),
        &tools.join("fixtures/chunk.safetensors"),
        &Config::bs_roformer_sw(),
        &default_device(),
    )
}
