# Watershed

Separate a song into stems and practice over them.

Watershed is an open-source desktop app for musicians who learn songs by ear.
Drop in an mp3, flac, wav, ogg or m4a file. Watershed splits it locally, on your
GPU, into vocals, drums, bass, guitar, piano and other with a state-of-the-art
RoFormer model, then opens the stems in a multitrack player built for practice.

It is meant to run next to your DAW. Your live instrument stays in Reaper (or
whatever you use); Watershed only plays the backing stems.

> **Status:** early development, used daily by its author on macOS and Windows.
> No release builds yet: build it from source (below). CI builds binaries for
> Windows, Linux and macOS on every push. See [the plan](docs/PLAN.md) and
> [the status](docs/STATUS.md).

The name is a tribute to "Watershed" by Opeth.

## Features

- **Separation:** drop a song and it is split into six stems in a new folder of
  your library (`Music/Watershed` by default), with a progress bar and Cancel. The
  model (BS-RoFormer SW, 188 MB) downloads on first use and is checked by sha256.
  Runs on Metal on macOS and Vulkan on Windows and Linux, with a CPU fallback. A 4
  minute song takes about 1–2 minutes on a laptop or mid-range GPU. Stems are
  saved as Ogg Vorbis (~25 MB per song) or, in Settings, as 24-bit FLAC.
- **Player:** one row per stem with its waveform, volume, pan, mute and solo;
  master volume and output device. Folders of stems from other tools (for example
  [MVSEP](https://mvsep.com)) open too, and stems that start late can be dragged
  into place.
- **Practice:** tempo change without pitch change, transposition in semitones
  (drums are left untransposed by default), A-B loops with several saved per song,
  named markers and sections, waveform zoom.
- **Tempo and key:** an approximate BPM and key estimated when a song opens, with
  tap tempo and ½ / ×2 to correct the BPM; a beat grid with a bar ruler and snap;
  a click (left, right or both ears) and a count-in. **Dynamic** makes the grid and
  the click follow the song's own beats, for bands that were not playing to a
  click. Tempo and time signature changes can be marked, as in a DAW.
- **Sessions:** everything is saved next to the stems in `.watershed.toml`, so a
  song reopens as you left it and is never separated twice. Recently opened songs
  are listed on the start screen.

## Building

Requires Rust 1.95 or newer and CMake. The separation engine
([BSRoformer.cpp](https://github.com/chenmozhijin/BSRoformer.cpp) and
[ggml](https://github.com/ggml-org/ggml)) comes in as git submodules, so clone with
`--recursive` or run:

```sh
git submodule update --init
cargo run --release
```

The first build takes a few minutes (ggml compiles its GPU shaders). Per platform:

- **macOS:** `xcode-select --install` and `brew install cmake`.
- **Windows:** the Visual Studio Build Tools, CMake, LLVM (`winget install
  LLVM.LLVM`, with `LIBCLANG_PATH` set to its `bin` folder) and the
  [Vulkan SDK](https://vulkan.lunarg.com) (`VULKAN_SDK` set).
- **Linux** (Debian/Ubuntu names):

  ```sh
  sudo apt install cmake libxkbcommon-dev libxcb-render0-dev libxcb-shape0-dev \
    libxcb-xfixes0-dev libwayland-dev libasound2-dev libvulkan-dev glslc spirv-headers
  ```

  Also clang (`libclang-dev`), for the time-stretching library's bindings.

## Usage

Drop a song file on the window to separate it, or drop a folder of stems (or use
**Open folder…**, or pass it on the command line: `watershed path/to/stems`) to
play it. Stems are recognized by name, in English or Spanish (`vocals`/`voz`,
`drums`/`bateria`, `bass`/`bajo`, `guitar`/`guitarra`, `piano`/`teclado`, `other`,
`instrumental`).

Some shortcuts (press **?** in the app for the full list):

| Key | Action |
|---|---|
| Space | Play / pause |
| Home | Back to the loop start, or the song start |
| ← / → | Seek 5 s |
| A / B, L | Loop start / end at the playhead, loop on or off |
| M, 1–9 | Add a marker, go to marker 1–9 |
| T | Tap tempo |
| C | Click on or off |
| Shift+C | Change the tempo or time signature at the playhead |
| + / − | Zoom |
| F5 | Reload the song folder |
| Cmd/Ctrl+, | Settings |

Click or drag on any waveform to seek.

**With Reaper on Windows:** Watershed plays through shared-mode WASAPI, so it can
share an audio interface with Reaper on ASIO. Set both to the same sample rate.

## Workspace layout

| Crate | Purpose |
|---|---|
| `crates/app` | The `watershed` binary: egui user interface |
| `crates/audio` | Real-time playback engine: mixer, tempo and pitch, limiter, click |
| `crates/dsp` | Decoding, FLAC and Ogg Vorbis writing, resampling, waveforms, tempo and key estimation |
| `crates/separation` | Source separation behind a `Separator` trait (BSRoformer.cpp + ggml today), model catalog and downloads |
| `crates/storage` | Sessions, model cache and configuration |
| `tools/reference` | Development-only PyTorch reference separation ([README](tools/reference/README.md)) |

## Roadmap

The current separation backend is temporary. The plan is a native Rust
implementation (`burn` + `wgpu`) that loads community RoFormer checkpoints
(`.ckpt` + `.yaml`) directly, then ensembles, WAV export and portable builds
(single `.exe`, `.app`, AppImage). Details in [the plan](docs/PLAN.md).

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
