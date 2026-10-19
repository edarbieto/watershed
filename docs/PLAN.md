# Watershed — Project plan

## 0. Decisions

| Topic | Decision |
|---|---|
| Language | Code, docs, commits and UI in English. i18n in v0.2. |
| License | `MIT OR Apache-2.0`. No GPL dependencies, so time-stretching uses Signalsmith Stretch (MIT). |
| Tempo / pitch | Applied to the **stereo mix**, after the mixer, so the cost does not grow with the number of stems. Real-time first; offline pre-render as an option if CPU is not enough. |
| Session stem format | FLAC (`flacenc` to write, `symphonia` to read). No lossy formats: separation artifacts plus lossy compression plus time-stretching add up. Stems can exceed full scale, which integer FLAC would clip: scale all stems of a song by one common factor before writing, and store that factor in the session. |
| Separation | Python is a development tool only (reference outputs for validation). Temporary shippable backend: GGUF via BSRoformer.cpp, to be validated by a spike. Final backend: `burn` + `wgpu`, which loads community `.ckpt` + `.yaml` directly. |
| Platforms | Windows, Linux and macOS from day one, all built in CI. |
| Stem storage cost | Measured with the reference tool: 24-bit FLAC takes ~170–200 MB per 6-stem song. |
| Reference speed | PyTorch + CUDA on the RTX 3050 separates a ~5 min song in 73–100 s. The native `wgpu` backend is measured against this. |
| Clipping | Stems can exceed full scale because the model works in float; they add back up to about the original mix. Not an error (MVSEP mp3s do it too). Only integer formats and the device output clip: FLAC uses the common factor above, and the master gets a light **limiter** (or default headroom) so a hot stem soloed at full volume never clips at the output. 32-bit float WAV was considered and rejected (~2× the size). |
| Temporary separation backend | BSRoformer.cpp (MIT, ggml) + q8_0 GGUF, validated by the spike: faster than PyTorch on the GPU, inaudible quantization. Same C++ source on all platforms, compiled by `build.rs`; GPU backend per platform: **Metal** on macOS, **Vulkan** on Windows/Linux. No CUDA in the distribution. Our wrapper sets the CPU thread count (ggml defaults to 4). Its progress and cancel callbacks drive the progress bar. |
| Reference dev machine | RTX 3050 6 GB, Ryzen 7 5000, 64 GB RAM, Yamaha AG03 (ASIO for Reaper and shared WASAPI for Watershed at the same time). |

## 1. Vision

An **open-source** desktop app written in **Rust + egui** for guitarists who learn songs by ear:

1. Drag in an mp3/flac/wav.
2. The app splits it into stems (vocals, drums, bass, guitar, piano, other) with state-of-the-art neural models (RoFormer family), **locally**.
3. The stems open in a **basic DAW-like multitrack player** to practice over.

It replaces this manual workflow: MVSEP website → download stems → build a folder → import into Reaper.

**Usage context:** live guitar (audio interface + amp VSTs) **stays in Reaper**. Watershed runs alongside it and only plays the separated stems.

## 2. Goals and non-goals

### Goals
- Distribution **as portable as possible**: a single `.exe` on Windows, a `.app` on macOS and an ELF on Linux.
- Models and heavy assets **download automatically** on first use, into the user's cache folder.
- GPU acceleration **without CUDA**: Vulkan / DX12 / Metal through `wgpu`.
- Use **any community RoFormer checkpoint** (`.ckpt` + `.yaml`), not just pre-converted models.
- A functional, sober interface without decoration.

### Non-goals
- Live audio input, monitoring, or hosting VST/CLAP plugins. Reaper does that.
- Audio editing, recording or effects.
- Training models.

## 3. Features

### Separation
- [ ] Import files (mp3, flac, wav, ogg, m4a) by drag & drop or a file dialog.
- [ ] Model catalog with automatic download and sha256 verification. Default model: **BS-RoFormer SW** (6 stems).
- [ ] Progress bar and cancel (backend progress and cancel callbacks).
- [ ] Chunked separation (overlapping chunks) to fit GPUs with ~4–6 GB of VRAM.
- [ ] CPU fallback when no compatible GPU is found.
- [ ] (Later) Ensembles: run N models and average the results.
- [ ] (Later) Optional "cloud" engine through the **MVSEP API** (https://mvsep.com/full_api) with the user's account, for premium models.

### Player
- [ ] One row per stem: waveform, **volume**, **pan L/R**, **mute**, **solo**.
- [ ] Play / pause / stop. Click the waveform to seek.
- [ ] **Master volume.**
- [ ] **Choose the audio output device.**
- [ ] **Tempo change without pitch change** (e.g. 50%–125%).
- [ ] **Transposition** in semitones (for songs tuned a half step down, etc.).
- [x] **A-B loop**, with several saved loops per song.
- [ ] **Markers and sections**: named markers placed on the waveform (button, a click,
  and Reaper's shortcut, `M`). A section runs from one marker to the next, or to the
  end of the song. Each marker has ▶ (play from here) and loop (loop its section).
- [ ] **Waveform zoom** and horizontal scroll, to place markers and loop ends precisely.

### Sessions
- [ ] Each song folder keeps its **session** in a file next to the stems
  (`.watershed.toml`): mixer state, tempo and pitch, saved loops, markers, and the
  common gain factor of separated stems. Reopening the folder restores everything;
  a separated song is such a folder, so it never separates again.
- [ ] Simple session library: recently opened folders, kept in the app config.
- [ ] Export individual stems or the current mix to WAV.

## 4. Architecture

```
┌──────────────────────────── egui (eframe) ────────────────────────────┐
│  Library │ Session view (tracks, transport, loops) │ Settings         │
└──────┬──────────────────────────┬─────────────────────────────────────┘
       │ commands / state         │ commands (lock-free)
┌──────▼───────────┐      ┌───────▼──────────────────────────────────────┐
│ Separation       │      │ Audio engine (cpal audio thread)             │
│ - trait Separator│      │ - mixes N stems in a single callback         │
│ - backends:      │      │ - gain/pan/mute/solo per stem                │
│   * burn-wgpu    │      │ - time-stretch + pitch-shift on the mix      │
│   * (temporary)  │      │ - A-B loop, global position in samples       │
│     gguf / ort   │      └──────────────────────────────────────────────┘
│   * mvsep API    │
└──────┬───────────┘      ┌──────────────────────────────────────────────┐
       │                  │ Storage: sessions, model cache, config       │
┌──────▼───────────┐      └──────────────────────────────────────────────┘
│ DSP: decode,     │
│ resample, STFT/  │
│ iSTFT, chunking  │
└──────────────────┘
```

### Design principles
- **Separation is a swappable component** (`trait Separator`). We can start with a temporary backend and switch to the final one without touching the UI or the audio engine.
- **Sync:** all stems come from the same audio and have the same length. They are mixed in **one callback** that advances **a single sample index**, so drift is impossible.
- **Real-time audio thread:** no allocation, no blocking locks and no I/O inside the callback. UI → audio communication goes through lock-free queues or atomics.
- **STFT outside the model:** the app computes STFT and iSTFT in Rust; the model takes and returns spectrograms. Parameters (`n_fft`, `hop_length`, `chunk_size`, bands, etc.) are read from each checkpoint's `.yaml`, never hardcoded.

## 5. AI backend

### Final target: `burn` + `wgpu`
- Reimplement the **BS-RoFormer** architecture in Rust, then **MelBand-RoFormer**. They share almost everything; they differ in how they split frequency bands.
- Load weights from community checkpoints: `burn-import` can read PyTorch `.ckpt`/`.pt` files directly (reading data only, never executing pickle code). Fall back to a `safetensors` conversion script if needed.
- Read the model config from its `.yaml`, in the same format used by ZFTurbo/Music-Source-Separation-Training.
- GPU through Vulkan, DX12 or Metal, without CUDA, which keeps a single binary.

### Validation strategy (mandatory)
1. Use the ZFTurbo repo (PyTorch) as the reference.
2. Run the same audio clip through PyTorch and the Rust implementation.
3. Compare intermediate tensors layer by layer (band split, transformers, mask estimator) and the final output. Define a numeric tolerance; report SDR between both outputs.
4. Keep these tests as regression tests.

### Temporary backends (to have something working in the meantime)
- **GGUF + BSRoformer.cpp** through FFI: there is a ~188 MB q8_0 GGUF of the SW model that runs on CPU, CUDA, Vulkan or Metal. Preferred: lightest, and can be linked into the binary.
- **ONNX Runtime (`ort`)** with an export that keeps the STFT outside the graph (see the Silverdaw references).
- **Python as a separate process** (`bs-roformer-infer` or the ZFTurbo repo). Development only, never shipped.

## 6. Models

| Model | Stems | Notes |
|---|---|---|
| **BS-RoFormer SW** (jarredou, mirror `enerjazzer/BS-ROFO-SW-Fixed`) | bass, drums, other, vocals, guitar, piano (**in that order**, per the `.yaml`) | Default model. `.ckpt` ~700 MB. **License: not declared** → do not bundle it; download on demand. |
| BS-RoFormer viperx / MelBand Kim / unwa / becruily / gabox | vocals / instrumental | For a future vocal ensemble. |
| Community guitar MelBand / BS RoFormer | guitar / other | Research which ones are public. The "MVSep Guitar" models are private. |
| MVSep Mega 53-stem (ZFTurbo) | 53 | **Out of scope**: needs ≥16 GB of VRAM. |

- The catalog lives in a config file (JSON or TOML) inside the repo: name, URL(s) with mirrors, sha256, architecture, associated `.yaml`, and stem names/order.
- On-disk location: the OS cache folder (`directories` crate).

## 7. Audio and coexisting with Reaper

- Output through `cpal`. On Windows, **shared-mode WASAPI**: Reaper will hold the interface through ASIO, and most ASIO drivers only accept one app at a time.
- **The sample rate must match Reaper/the interface** (typically 48 kHz). The app resamples stems (`rubato`) to the device rate.
- Output device picker in Settings; remember the last choice.
- Linux: ALSA/PipeWire through `cpal`. macOS: CoreAudio.

## 8. Candidate crates (to evaluate)

| Area | Crate |
|---|---|
| UI | `eframe` / `egui` |
| File dialogs | `rfd` |
| Decoding | `symphonia` |
| Audio output | `cpal` |
| Resampling | `rubato` |
| FFT / STFT | `realfft` / `rustfft` |
| ML | `burn` (`wgpu` backend) |
| Weights | `safetensors`, `burn-import` |
| Model config | `serde`, `serde_yaml`, `serde_json` / `toml` |
| Downloads | `reqwest` (or `hf-hub`), `sha2` for verification |
| Time-stretch / pitch | Signalsmith Stretch bindings (MIT) |
| FLAC encoding | `flacenc` |
| WAV export | `hound` |
| Paths | `directories` |
| Thread communication | `crossbeam-channel`, `rtrb` (RT ring buffer), atomics |

## 9. Distribution

- One binary per platform, statically linking as much as possible.
- No CUDA, Python or Electron in the distribution.
- First run: download the default model with visible progress.
- Public repo with CI (GitHub Actions) building for Windows, Linux and macOS.
- Watch dependency licenses (no GPL) and model licenses.
- macOS: signing and notarization will be needed before sharing builds widely.

## 10. Phases

### Phase 0: Skeleton
- [x] Cargo workspace with separate crates: `app` (UI), `audio`, `dsp`, `separation`, `storage`.
- [x] Empty egui window.
- [x] CI building on the 3 platforms.
- [x] Python reference environment (`tools/reference`, uv + ZFTurbo/MSST). 3 songs separated with BS-RoFormer SW as test stems and future golden outputs.

### Phase 1: Player with pre-separated stems
- [x] Open a folder of stems in any supported format (e.g. MVSEP mp3s, reference FLACs).
- [x] Audio engine: mixing, volume, pan, mute, solo, master, seek.
- [x] Output device picker + resampling.
- [x] Waveforms.
- [x] ✅ Milestone: replaces Reaper for stem playback (confirmed in a practice session on macOS).

### Spikes (before Phase 2)
- [x] Signalsmith Stretch on the stereo mix in real time: under 1% of one core, ~120 ms
  latency, sounds good. Real-time is enough; no offline pre-render needed.
- [x] Build BSRoformer.cpp/GGUF and separate a song: 79 s on Metal (M5 Pro) vs 120 s for
  PyTorch MPS, SDR 36–43 dB against the PyTorch output. Details in `STATUS.md`.

### Before Phase 3 (Windows, not blocking Phase 2)
- [ ] Signalsmith benchmark on the Windows machine.
- [ ] BSRoformer.cpp on the RTX 3050: Vulkan vs CUDA vs PyTorch CUDA. Decides whether
  Vulkan is good enough as the Windows backend.
- [ ] Vulkan SDK + `glslc` in CI (Windows, Linux); pin a ggml commit.

### Phase 2: Practice tools
In this order:
- [x] Tempo without pitch change, transposition: Signalsmith on the master, latency
  compensation in the displayed position, `reset` + `seek` pre-roll on seek, instance
  created off the audio thread. Master limiter.
- [x] Saved A-B loops (stored in the session file), transport buttons
  (start or loop start, ±5 s).
- [x] Session file per song folder (`.watershed.toml`): mixer, tempo/pitch, loops.
- [x] Markers and sections (stored in the session file).
- [ ] Waveform zoom.
- [ ] Library of recent folders.

### Phase 3: Separation with a temporary backend
- [ ] `trait Separator` + GGUF backend.
- [ ] Model catalog + download + sha256.
- [ ] Full flow: drop an mp3 → separate → open the session.
- ✅ Milestone: replaces MVSEP + Reaper for the basic case.

### Phase 4: Native `burn` + `wgpu` backend
- [ ] STFT/iSTFT + overlapping chunking in Rust.
- [ ] BS-RoFormer in burn + weight loading + equivalence tests against PyTorch.
- [ ] MelBand-RoFormer.
- [ ] Remove the temporary backend from the distribution.

### Phase 5: Extras
- [ ] Ensembles.
- [ ] Cloud backend (MVSEP API).
- [ ] Export WAV / mix.

## 11. Risks

| Risk | Mitigation |
|---|---|
| Reimplementing RoFormer with numeric accuracy | Layer-by-layer equivalence tests against PyTorch from day one. |
| `wgpu` performance vs CUDA | Measure early; acceptable if around 2× slower. Tune chunk size. |
| Limited VRAM (target ≥4–6 GB) | Configurable chunking; CPU fallback. |
| Undeclared model licenses | Do not bundle them; download from the source or mirrors. |
| Audio dropouts | Real-time discipline in the callback; configurable buffers. |
| Device conflict with Reaper/ASIO | Shared WASAPI + matching sample rate; document it. |

## 12. References

- MVSEP algorithms and metrics: https://mvsep.com/algorithms
- ZFTurbo / Music-Source-Separation-Training (PyTorch reference, `.yaml` configs): https://github.com/ZFTurbo/Music-Source-Separation-Training
- BS-RoFormer architecture (lucidrains): https://github.com/lucidrains/BS-RoFormer
- BS-RoFormer SW mirror (ckpt + yaml + GGUF): https://huggingface.co/FractalSurfer/BS-RoFormer-SW-6stem
- bs-roformer-infer (checkpoint registry): https://github.com/openmirlab/bs-roformer-infer
- Silverdaw, ONNX with STFT outside the graph: https://huggingface.co/silverdaw/bs-roformer-rhythm-onnx
- `stem-splitter-core` crate (htdemucs + ort, chunking reference): https://docs.rs/crate/stem-splitter-core/latest
- Similar apps to study:
  - StemDeck (open source, Demucs, web mixer): https://alternativeto.net/software/stemdeck/about/
  - PracticeSession (commercial, desktop): https://alternativeto.net/software/practice-session/about
