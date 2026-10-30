# Reference separation (development only)

Runs the original PyTorch code from
[ZFTurbo/Music-Source-Separation-Training](https://github.com/ZFTurbo/Music-Source-Separation-Training)
(MSST), pinned to a known commit and unmodified. It is never shipped with Watershed.

It serves two purposes:

1. Produce real stems to test the player while the native backend does not exist.
2. Produce the ground-truth outputs that the Rust implementation of BS-RoFormer
   is validated against (see the validation strategy in [docs/PLAN.md](../../docs/PLAN.md)).

## Requirements

- [uv](https://docs.astral.sh/uv/). It installs Python 3.12 and the dependencies by itself.
- `git`.
- The device is picked automatically: an NVIDIA GPU (PyTorch with CUDA 12.8), then
  Apple Silicon's GPU (MPS, default PyTorch wheels on macOS), then the CPU.

## Usage

```sh
cd tools/reference
uv run separate.py path/to/song.mp3 path/to/folder --out path/to/stems
```

The first run clones MSST into `.msst/` and downloads the model into `models/`
(~700 MB, verified by sha256). Each song becomes a folder with one 24-bit FLAC
per stem, which Watershed can open directly.

| Option | Meaning |
|---|---|
| `--model bs-roformer-sw` | Model to use (only one for now). |
| `--cpu` | Force CPU inference. |

Measured speed (BS-RoFormer SW):

| Machine | Device | Time |
|---|---|---|
| RTX 3050 6 GB | CUDA | a ~5 min song in 73–100 s (~0.3× real time) |
| Apple M5 Pro | MPS | a 4:39 song in 120 s (~0.43× real time) |
| Apple M5 Pro | CPU | 30 s of audio in 50 s (~1.7× real time; MPS took 19 s) |

## Notes

- Mixed precision (AMP) is only used on CUDA. MPS runs in float32 and matches the CPU
  output to within 1e-5. Operators that MPS lacks fall back to the CPU
  (`PYTORCH_ENABLE_MPS_FALLBACK`).
- BS-RoFormer SW outputs `bass, drums, other, vocals, guitar, piano`, 44.1 kHz stereo.
- Stems can peak above full scale, which FLAC cannot store. When that happens all
  stems of the song are scaled down by the same factor, so their balance is kept.
- The model has no declared license. It is downloaded from a public mirror and is
  not part of this repository.

## Tensors for validating the Rust model

```sh
uv run dump_tensors.py path/to/song.mp3 --out fixtures/chunk.safetensors
```

Runs one chunk of the song (from 60 s by default, `--start` changes it) through the
model on the CPU in f32 and saves the input, each layer's output and the final audio,
~2 GB in all. `*.safetensors` is ignored by git.
`crates/roformer/examples/chunk.rs` compares the Rust model against this file.
