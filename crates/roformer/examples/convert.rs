//! Converts a checkpoint to the f16 safetensors file the app installs, and prints its
//! size and SHA-256 for `models.toml`.
//!
//! ```sh
//! cargo run --release -p watershed-roformer --example convert -- CHECKPOINT OUT.safetensors
//! ```

use std::io::Read;
use std::time::Instant;

use anyhow::Result;
use sha2::{Digest, Sha256};
use watershed_roformer::convert_to_half;

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [checkpoint, out] = args.as_slice() else {
        anyhow::bail!("usage: convert CHECKPOINT OUT.safetensors");
    };
    let start = Instant::now();
    convert_to_half(checkpoint.as_ref(), out.as_ref())?;
    println!("converted in {:.1} s", start.elapsed().as_secs_f32());

    let mut file = std::fs::File::open(out)?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0; 1 << 16];
    let mut size = 0;
    loop {
        let n = file.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
        size += n;
    }
    let hash: String = hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    println!("size = {size}\nsha256 = \"{hash}\"");
    Ok(())
}
