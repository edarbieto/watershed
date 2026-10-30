//! Runs a chunk dumped by `tools/reference/dump_tensors.py` through the model: times
//! it, checks that quiet input stays finite and compares each layer with PyTorch.
//!
//! ```sh
//! cargo run --release -p watershed-roformer --example chunk -- [--full] CHECKPOINT DUMP
//! ```
//! `--full` computes in f32 instead of f16.

use std::time::Instant;

use anyhow::{Context, Result};
use burn::prelude::*;
use burn::tensor::{DType, TensorData};
use safetensors::SafeTensors;
use watershed_roformer::{BsRoformer, Config, Precision, Weights, default_device};

fn values(dump: &SafeTensors, name: &str) -> Result<(Vec<f32>, Vec<usize>)> {
    let view = dump.tensor(name).with_context(|| name.to_string())?;
    let values = view
        .data()
        .as_chunks::<4>()
        .0
        .iter()
        .map(|b| f32::from_le_bytes(*b))
        .collect();
    Ok((values, view.shape().to_vec()))
}

fn floats<const D: usize>(tensor: Tensor<D>) -> Vec<f32> {
    tensor
        .cast(DType::F32)
        .into_data()
        .try_to_vec()
        .expect("cast to f32")
}

/// RMS error relative to the reference's RMS.
fn error(got: &[f32], reference: &[f32]) -> f64 {
    let energy: f64 = reference.iter().map(|v| (*v as f64).powi(2)).sum();
    let error: f64 = got
        .iter()
        .zip(reference)
        .map(|(a, b)| ((a - b) as f64).powi(2))
        .sum();
    (error / energy).sqrt()
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
    let [checkpoint, dump] = args.as_slice() else {
        anyhow::bail!("usage: chunk [--full] CHECKPOINT DUMP");
    };
    let device = default_device();

    let start = Instant::now();
    let config = Config::bs_roformer_sw();
    let model = BsRoformer::load(&Weights::open(checkpoint)?, &config, precision, &device)?;
    device.sync()?;
    println!("model loaded in {:.1} s", start.elapsed().as_secs_f32());

    let bytes = std::fs::read(dump)?;
    let (stft, shape) = values(&SafeTensors::deserialize(&bytes)?, "stft")?;
    drop(bytes);
    let shape: [usize; 4] = shape.as_slice().try_into()?;
    let stft = || Tensor::<4>::from_data(TensorData::new(stft.clone(), shape), &device);

    for pass in 0..4 {
        let start = Instant::now();
        model.forward(stft()).into_data();
        println!("pass {pass}: {:.2} s", start.elapsed().as_secs_f32());
    }
    if let Some(usage) = device.memory_pool_usage() {
        println!(
            "GPU memory: {} MiB in use, {} MiB reserved",
            usage.bytes_in_use >> 20,
            usage.bytes_reserved >> 20
        );
    }

    for gain in [0.0, 1e-3, 1e-5] {
        let masks = floats(model.forward(stft() * gain));
        let bad = masks.iter().filter(|v| !v.is_finite()).count();
        println!("input × {gain}: {bad} values not finite");
    }

    let bytes = std::fs::read(dump)?;
    let dump = SafeTensors::deserialize(&bytes)?;
    let masks = model.forward_traced(stft(), &mut |name, tensor| {
        let (reference, _) = values(&dump, name).expect("in the dump");
        println!("{name:>12}: {:.2e}", error(&floats(tensor), &reference));
    });
    let masks = floats(masks);

    // PyTorch's masks are (time, frequency * 2), ours (frequency, time, 2).
    let (freqs, frames) = (shape[1], shape[2]);
    let mut reference = Vec::with_capacity(masks.len());
    for stem in 0..config.num_stems {
        let (mask, _) = values(&dump, &format!("mask_{stem}"))?;
        for f in 0..freqs {
            for t in 0..frames {
                reference.extend_from_slice(&mask[(t * freqs + f) * 2..][..2]);
            }
        }
    }
    println!("{:>12}: {:.2e}", "masks", error(&masks, &reference));
    Ok(())
}
