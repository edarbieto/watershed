# Watershed: notes for Claude

Open-source desktop app (Rust + egui) that separates songs into stems with RoFormer
models and plays them in a multitrack player for practice. Read these first:

- [docs/PLAN.md](docs/PLAN.md): vision, decisions (section 0), architecture, phases.
- [docs/STATUS.md](docs/STATUS.md): where we are, open questions, observations to
  investigate. **Update it at the end of every work session**, since work moves
  between machines (Windows desktop and a MacBook).

## Working with the user

- Reply in the language the user writes in. Everything in the repo is **English**:
  code, comments, docs, commit messages, UI text.
- The user's Rust/egui experience is medium-low and ML is new to them (checkpoints,
  GGUF, ONNX, PyTorch). Claude writes most of the code; explain ML and audio concepts
  plainly when they come up.
- Never put personal data in the repo: no usernames, home paths, emails or machine
  names in files.
- Commit in semantic groups (one feature / sub-phase per commit), Conventional
  Commits style (`feat(audio): ...`, `fix(tools): ...`, `docs: ...`). Push to `master`
  on `origin` when asked or when finishing a block of work the user approved.
- Before committing: `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`,
  `cargo test --workspace`. Do not chain `git commit` after a failing check.

## Toolchain

- Rust stable, MSRV 1.95 (eframe 0.36 requires it). **CI uses the latest stable**,
  which may have newer clippy lints than the local toolchain: run `rustup update`
  before working, or CI may fail on lints that pass locally.
- CI: `.github/workflows/ci.yml` (fmt, clippy, tests, release build on Windows, Linux,
  macOS; binaries uploaded as artifacts).
- BSRoformer.cpp and ggml are **git submodules** in `crates/separation/vendor/`
  (pinned commits). Clone with `--recursive` or run `git submodule update --init`.
  Building them needs **CMake**, plus the **Vulkan SDK** on Windows (`VULKAN_SDK`
  set) or `libvulkan-dev glslc spirv-headers` on Linux. The first build of the
  crate takes ~2 min (ggml compiles its Vulkan shaders).

### Windows setup

LLVM (`winget install LLVM.LLVM`, `LIBCLANG_PATH=C:\Program Files\LLVM\bin`) for
Signalsmith's bindings, CMake, the Vulkan SDK from vulkan.lunarg.com, and the Visual
Studio Build Tools. Open a new terminal after installing so it sees the new variables.

### macOS setup

```sh
xcode-select --install          # compiler and SDK
curl https://sh.rustup.rs -sSf | sh
brew install cmake
brew install uv                 # only for tools/reference
git submodule update --init
cargo run --release -- path/to/stems-folder
```

- The config file lives in `~/Library/Application Support/Watershed/config.toml`
  (Windows: `%APPDATA%\Watershed\config\config.toml`). It stores the output device
  id, which differs per machine; an unknown id falls back to the default device.
- `tools/reference/separate.py` uses the GPU through MPS on Apple Silicon (about 2.6×
  faster than the CPU).

### Local files that are not in git

Recreated automatically, nothing to copy:
- `tools/reference/.venv`, `.msst/`, `models/`: created by `uv run separate.py`
  (downloads PyTorch, the MSST code and the ~700 MB model).

Not recreated: **test songs and stems** live outside the repo. Copy them to the new
machine or regenerate them with the reference tool from any mp3s.

## Code map

| Path | What |
|---|---|
| `crates/app` | `watershed` binary. `app.rs` UI and state, `loader.rs` background loading, `separate.rs` background separation job, `bpm.rs` song BPM (estimate, tap, user value), `grid.rs` beat grid (bar 1, tempo and time signature changes, dynamic tempo), `key.rs` song key, `metronome.rs` click and count-in settings and controls, `loops.rs`, `markers.rs`, `view.rs` zoom and scroll, `playhead.rs` smooth playhead, `shortcuts.rs` shortcuts window (one table to update when a shortcut is added), `stems.rs` stem names/colors, `waveform.rs` widget, `usage.rs` CPU/GPU/RAM display, `alloc.rs` macOS allocator |
| `crates/audio` | `Engine` (cpal stream), `Song`/`Track`/`TrackParams` (atomics), `Samples` (packed 24-bit), `Beats` (beat times for the grid and click), `click.rs`, `mixer.rs` (pure, tested), `stretch.rs` (Signalsmith, two buses for pitch lock), `limiter.rs`; `examples/play.rs`, `examples/stretch.rs` |
| `crates/dsp` | `decode_file` (symphonia), `write_flac` + `gain_to_fit` (flac-codec, 24-bit), `write_vorbis` (vorbis_rs), `resample` (rubato FFT), `Peaks`, `OnsetEnvelope` (tempo estimation, beat tracking with anchors), `Chroma`/`Key` (key estimation); `examples/bpm.rs`, `examples/beats.rs`, `examples/key.rs`, `examples/probe.rs` |
| `crates/storage` | `Config` (TOML in the OS config dir), `Session` (`.watershed.toml`), `models_dir` |
| `crates/separation` | `trait Separator`, `GgufSeparator`, `separate_song` (song file → folder of FLAC stems + session); `catalog` (`models.toml`: downloadable models, `install` with sha256 check), `download` (resumable, mirrors); `bsroformer::Model` (safe wrapper over BSRoformer.cpp + ggml); `cpp/` C wrapper and CMake project; `build.rs`; `vendor/` submodules; `examples/separate.rs` |
| `tools/reference` | dev-only PyTorch reference separation (uv project) |

Real-time rule: the cpal callback never allocates, frees, locks or does I/O. Songs go
to the audio thread through an `rtrb` queue and come back on another queue to be
dropped on the UI thread. Mixer parameters are atomics.

## Gotchas learned so far

- Several dependencies are newer than Claude's training data: **eframe/egui 0.36**
  (`App::ui(&mut self, ui, frame)` instead of `update`, `egui::Panel::top(..).show(ui, ..)`,
  `CentralPanel::default_margins()`, `DroppedFile` is a trait with `.path()`),
  **cpal 0.18** (`device.id()` / `description()`, `DeviceId` parses from string,
  `sample_rate` is a plain `u32`), **symphonia 0.6**, **rubato 5** (`Fft` +
  `process_all` with `audioadapter` buffers). Read the sources in
  `~/.cargo/registry/src/` instead of guessing APIs.
- Separated stems often exceed full scale (+0.8 to +4.5 dB measured). Integer formats
  like FLAC clip them: scale all stems of a song by one common factor.
- The BS-RoFormer SW `.yaml` has `audio.hop_length: 441` marked "don't work"; the model
  really uses `model.stft_hop_length: 512`. Always read parameters from the config.
- **C++ build (`crates/separation`)**: the `cmake` crate fills
  `CMAKE_<LANG>_FLAGS_RELEASE` with flags that carry no optimization level, so
  `build.rs` sets them itself (without that, separation ran 2.3× slower). MSVC tools
  still enforce the 260-character path limit, and ggml's nested Vulkan shader
  generator build under cargo's deep `OUT_DIR` hit it; `cpp/CMakeLists.txt` shortens
  it with `EP_BASE`. BSRoformer.cpp is not modified: CMake compiles a patched copy of
  its `model.cpp` that calls `watershed_bsr_init_backend()` in `cpp/wrapper.cpp` (backend
  choice and CPU thread count, plus contiguous K/V before flash attention), of its
  `inference.cpp` (smaller pipeline queues, earlier frees) and of ggml's
  `ggml-metal-ops.cpp` (no unused flash-attention buffers). If a submodule bump breaks
  the patch anchors, CMake stops with a clear error.
- **macOS memory numbers:** malloc keeps freed large blocks, and they count in RSS and
  the footprint. The app's global allocator (`alloc.rs`) maps blocks ≥ 4 MiB directly;
  C++ allocations still go through malloc. To measure in-use memory only, run with
  `MallocLargeCache=0` (undocumented, for measuring only).
- **macOS renders with OpenGL** (`eframe::Renderer::Glow`, eframe's `glow` feature
  enabled only for the macOS target), not wgpu: with wgpu the app sometimes took
  ~300 ms to answer window moves from window managers like Rectangle. With `glow`
  on, `App::on_exit` takes the GL context, so it is defined per platform.
- To screenshot the app on macOS: `screencapture -x -o -l <window id>` (the id from
  `CGWindowListCopyWindowInfo`); it captures only that window.
- To screenshot the app on Windows, capture only its window with `PrintWindow`
  (`PW_RENDERFULLCONTENT`); screen-region captures can grab whatever window is on top.
