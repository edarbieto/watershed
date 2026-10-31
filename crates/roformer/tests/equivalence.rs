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
use watershed_roformer::{BsRoformer, Config, Precision, Weights, default_device};

/// Largest relative error allowed per layer in f32.
const LIMIT: f64 = 1e-3;

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
