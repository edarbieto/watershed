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
- An NVIDIA GPU is used if present (PyTorch with CUDA 12.8). Otherwise it runs on CPU,
  much slower. On macOS the default PyTorch wheels are used.

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

## Notes

- BS-RoFormer SW outputs `bass, drums, other, vocals, guitar, piano`, 44.1 kHz stereo.
- The model has no declared license. It is downloaded from a public mirror and is
  not part of this repository.
