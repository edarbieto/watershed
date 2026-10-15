# Watershed: notes for Claude

Open-source desktop app (Rust + egui) that separates songs into stems with RoFormer
models and plays them in a multitrack player for practice. Read these first:

- [docs/PLAN.md](docs/PLAN.md): vision, decisions (section 0), architecture, phases.
- [docs/STATUS.md](docs/STATUS.md): where we are, open questions, observations to
  investigate. **Update it at the end of every work session**, since work moves
  between machines (Windows desktop and a MacBook).

## Working with the user

- Chat with the user in **Spanish**. Everything in the repo is **English**: code,
  comments, docs, commit messages, UI text.
- The user's Rust/egui experience is medium-low and ML is new to them (checkpoints,
  GGUF, ONNX, PyTorch). Claude writes most of the code; explain ML and audio concepts
  plainly when they come up.
- Never put personal data in the repo: no usernames, home paths, emails or machine
  names in files.
- Commit in semantic groups (one feature / sub-phase per commit), Conventional
  Commits style (`feat(audio): ...`, `fix(tools): ...`, `docs: ...`). Push to `main`
  on `origin` when asked or when finishing a block of work the user approved.
- Before committing: `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`,
  `cargo test --workspace`. Do not chain `git commit` after a failing check.

## Toolchain

- Rust stable, MSRV 1.95 (eframe 0.36 requires it). **CI uses the latest stable**,
  which may have newer clippy lints than the local toolchain: run `rustup update`
  before working, or CI may fail on lints that pass locally.
- CI: `.github/workflows/ci.yml` (fmt, clippy, tests, release build on Windows, Linux,
  macOS; binaries uploaded as artifacts).

### macOS setup

```sh
xcode-select --install          # compiler and SDK
curl https://sh.rustup.rs -sSf | sh
brew install uv                 # only for tools/reference
cargo run --release -- path/to/stems-folder
```

- The config file lives in `~/Library/Application Support/Watershed/config.toml`
  (Windows: `%APPDATA%\Watershed\config\config.toml`). It stores the output device
  id, which differs per machine; an unknown id falls back to the default device.
- `tools/reference/separate.py` only picks CUDA or CPU. On a Mac it will run on CPU
  (slow). Adding Apple's MPS device is a pending task (see STATUS.md).

### Local files that are not in git

Recreated automatically, nothing to copy:
- `tools/reference/.venv`, `.msst/`, `models/`: created by `uv run separate.py`
  (downloads PyTorch, the MSST code and the ~700 MB model).

Not recreated: **test songs and stems** live outside the repo. Copy them to the new
machine or regenerate them with the reference tool from any mp3s.

## Code map

| Path | What |
|---|---|
| `crates/app` | `watershed` binary. `app.rs` UI and state, `loader.rs` background loading, `stems.rs` stem names/colors, `waveform.rs` widget |
| `crates/audio` | `Engine` (cpal stream), `Song`/`Track`/`TrackParams` (atomics), `mixer.rs` (pure, tested) |
| `crates/dsp` | `decode_file` (symphonia), `resample` (rubato FFT), `Peaks` |
| `crates/storage` | `Config` (TOML in the OS config dir) |
| `crates/separation` | empty, Phase 3 |
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
- To screenshot the app on Windows, capture only its window with `PrintWindow`
  (`PW_RENDERFULLCONTENT`); screen-region captures can grab whatever window is on top.
