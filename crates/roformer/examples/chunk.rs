//! Runs a chunk dumped by `tools/reference/dump_tensors.py` through the model: times
//! it, checks that quiet input stays finite and compares each layer with PyTorch.
//!
//! ```sh
//! cargo run --release -p watershed-roformer --example chunk -- [--full] CHECKPOINT DUMP
//! ```
//! `--full` computes in f32 instead of f16.

use std::collections::BTreeMap;
use std::time::Instant;

use anyhow::Result;
use burn::prelude::*;
use burn::tensor::TensorData;
use safetensors::SafeTensors;
use watershed_roformer::{BsRoformer, Config, Precision, Weights, default_device};

#[path = "../tests/reference/mod.rs"]
mod reference;

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
    let (stft, shape) = reference::values(&SafeTensors::deserialize(&bytes)?, "stft")?;
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

    // Waiting for the GPU after each layer costs some time, so the sum is above a pass.
    let mut blocks = BTreeMap::<String, f32>::new();
    let mut last = Instant::now();
    let masks = model.forward_traced(stft(), &mut |name, _| {
        device.sync().expect("GPU");
        let block = name.split('_').next().expect("a name").to_string();
        *blocks.entry(block).or_default() += last.elapsed().as_secs_f32();
        last = Instant::now();
    });
    masks.into_data();
    blocks.insert("masks".to_string(), last.elapsed().as_secs_f32());
    println!("per block: {blocks:.2?}");

    for gain in [0.0, 1e-3, 1e-5] {
        let masks = reference::floats(model.forward(stft() * gain));
        let bad = masks.iter().filter(|v| !v.is_finite()).count();
        println!("input × {gain}: {bad} values not finite");
    }

    let bytes = std::fs::read(dump)?;
    let dump = SafeTensors::deserialize(&bytes)?;
    for (name, error) in reference::layer_errors(&model, &dump, config.num_stems, &device)? {
        println!("{name:>12}: {error:.2e}");
    }
    Ok(())
}
