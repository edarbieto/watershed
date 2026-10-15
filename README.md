# Watershed

Separate a song into stems and practice over them.

Watershed is an open-source desktop app for musicians who learn songs by ear.
Drop in an mp3, flac or wav file. Watershed splits it locally into vocals, drums,
bass, guitar, piano and other with state-of-the-art RoFormer models, then opens
the stems in a simple multitrack player built for practice: per-track volume, pan,
mute and solo, tempo change without pitch change, transposition and A-B loops.

It is meant to run next to your DAW. Your live instrument stays in Reaper (or
whatever you use); Watershed only plays the backing stems.

> **Status:** early development. Nothing usable yet. See [the plan](docs/PLAN.md).

The name is a tribute to "Watershed" by Opeth.

## Goals

- One self-contained binary per platform: Windows, Linux and macOS.
- Models download on first use to your user cache folder.
- GPU acceleration through `wgpu` (Vulkan, DX12, Metal). No CUDA, no Python.
- Load community RoFormer checkpoints (`.ckpt` + `.yaml`) directly.

## Building

Requires Rust 1.95 or newer.

```sh
cargo run --release
```

On Linux you also need the development packages for X11/Wayland and ALSA:

```sh
sudo apt install libxkbcommon-dev libxcb-render0-dev libxcb-shape0-dev \
  libxcb-xfixes0-dev libwayland-dev libasound2-dev
```

## Workspace layout

| Crate | Purpose |
|---|---|
| `crates/app` | The `watershed` binary: egui user interface |
| `crates/audio` | Real-time playback engine |
| `crates/dsp` | Decoding, resampling, STFT/iSTFT, chunking |
| `crates/separation` | Source separation backends behind a `Separator` trait |
| `crates/storage` | Sessions, model cache and configuration |

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.

Separation models are not part of this repository and are downloaded from their
original sources. Each model has its own license (some have none declared).

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
