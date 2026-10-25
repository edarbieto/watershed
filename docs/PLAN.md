# Watershed — Project plan

## 0. Decisions

| Topic | Decision |
|---|---|
| Language | Code, docs, commits and UI in English. i18n in v0.2. |
| License | `MIT OR Apache-2.0`. No GPL dependencies, so time-stretching uses Signalsmith Stretch (MIT). |
| Tempo / pitch | Applied to the **stereo mix**, after the mixer, so the cost does not grow with the number of stems. Real-time first; offline pre-render as an option if CPU is not enough. |
| Session stem format | 24-bit FLAC (`flac-codec` to write, `symphonia` to read; `flacenc` was rejected: it only has 16-bit Rice codes and bloats loud 24-bit audio). Lossy formats were first rejected (separation artifacts plus lossy compression plus time-stretching add up); Phase 3.5 adds a choice in the app settings: **Lossy** (the default, about 5× smaller) or **Lossless** (FLAC). The lossy codec is **Ogg Vorbis at ~256 kbps** (q8), chosen with the user over AAC and Opus for DAW compatibility (comparison in `STATUS.md`, "Lossy stem codec"). MP3 is out: its usable encoders are LGPL. Stems can exceed full scale, which integer FLAC would clip: scale all stems of a song by one common factor before writing, and store that factor in the session. |
| Separation | Python is a development tool only (reference outputs for validation). Temporary shippable backend: GGUF via BSRoformer.cpp, to be validated by a spike. Final backend: `burn` + `wgpu`, which loads community `.ckpt` + `.yaml` directly. |
| Platforms | Windows, Linux and macOS from day one, all built in CI. |
| Stem storage cost | 24-bit FLAC takes ~150–200 MB per 6-stem song; Ogg Vorbis q8 (the default) ~25 MB for the same 4 min song (measured: 24 MB vs 149 MB). |
| Reference speed | PyTorch + CUDA on the RTX 3050 separates a ~5 min song in 73–100 s. The native `wgpu` backend is measured against this. |
| Clipping | Stems can exceed full scale because the model works in float; they add back up to about the original mix. Not an error (MVSEP mp3s do it too). Only integer formats and the device output clip: FLAC uses the common factor above, and the master gets a light **limiter** (or default headroom) so a hot stem soloed at full volume never clips at the output. 32-bit float WAV was considered and rejected (~2× the size). |
| Temporary separation backend | BSRoformer.cpp (MIT, ggml) + q8_0 GGUF, validated by the spikes: inaudible quantization; on macOS (Metal) faster than PyTorch MPS, on Windows (Vulkan, RTX 3050) a 5 min song in ~2 min, 1.6× slower than CUDA but on any GPU and with nothing extra to ship. Same C++ source on all platforms, compiled by `build.rs`; GPU backend per platform: **Metal** on macOS, **Vulkan** on Windows/Linux. No CUDA in the distribution (could become an optional NVIDIA build later if speed matters). CPU only as a fallback (~4× slower than real time on the 5700G), with one thread per physical core. Our wrapper sets the CPU thread count (ggml defaults to 4). Its progress and cancel callbacks drive the progress bar. |
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
- [x] Import files (mp3, flac, wav, ogg, m4a) by drag & drop or a file dialog. One file is
  separated into a new folder in the library (`Music/Watershed` by default); a folder or
  several files open as stems.
- [x] Model catalog with automatic download and sha256 verification. Default model: **BS-RoFormer SW** (6 stems).
- [x] Progress bar and cancel (backend progress and cancel callbacks).
- [x] Chunked separation (overlapping chunks) to fit GPUs with ~4–6 GB of VRAM.
- [x] CPU fallback when no compatible GPU is found (not yet tried on a machine without one).
- [ ] (Later) Ensembles: run N models and average the results.
- [ ] (Later) Optional "cloud" engine through the **MVSEP API** (https://mvsep.com/full_api) with the user's account, for premium models.

### Player
- [x] One row per stem: waveform, **volume**, **pan L/R**, **mute**, **solo**.
- [x] Play / pause / stop. Click the waveform to seek.
- [x] **Master volume.**
- [x] **Choose the audio output device.**
- [x] **Tempo change without pitch change** (e.g. 50%–125%).
- [x] **Transposition** in semitones (for songs tuned a half step down, etc.).
- [x] **A-B loop**, with several saved loops per song.
- [x] **Markers and sections**: named markers placed on the waveform (button, a click,
  and Reaper's shortcut, `M`). A section runs from one marker to the next, or to the
  end of the song. Each marker has ▶ (play from here) and loop (loop its section).
- [x] **Waveform zoom** and horizontal scroll, to place markers and loop ends precisely.
- [ ] **Approximate BPM**, estimated when a song is opened (see section 3.1), plus tap
  tempo to correct it. Shown next to the tempo slider as the BPM at the current tempo.
- [ ] **Pitch lock per track**: transposition skips drum tracks by default (pitch-shifted
  drums sound wrong and gain nothing). See Phase 3.5.
- [ ] **Count-in** before playback, from the BPM and a time signature (4/4 by default).
  See Phase 3.5.

### Sessions
- [x] Each song folder keeps its **session** in a file next to the stems
  (`.watershed.toml`): mixer state, tempo and pitch, saved loops, markers, and the
  common gain factor of separated stems. Reopening the folder restores everything;
  a separated song is such a folder, so it never separates again.
- [x] Simple session library: recently opened folders, kept in the app config.
- [ ] Export individual stems or the current mix to WAV.

### 3.1 BPM estimation (idea, not scheduled)

Cost: **moderate, about one or two work sessions**, no new heavy dependencies.
1. Onset strength: STFT of the drums stem (or the sum of all stems when there is none)
   with `realfft`, and the spectral flux (how much energy rises from one frame to the
   next), which peaks on hits.
2. Tempo: autocorrelation of that curve over lags of 60–200 BPM; the strongest
   periodic lag is the beat period. A prior centered around 120 BPM picks between
   candidates.
3. Runs in the loader thread after decoding: well under a second for a song.

Known limits: half/double errors (70 vs 140 BPM), songs with tempo changes or rubato,
no beat grid (only a number). Mitigations: a ×2 / ÷2 button, tap tempo, store the
corrected value in the session. Having separated drums makes this much more reliable
than on a full mix. A beat grid (and snapping loops to beats) would be a larger,
separate step.

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

- **Lean on memory and disk.** Measure before optimizing, but keep it in mind in
  every phase. Current and planned measures:
  - RAM: stems are packed 24-bit stereo at the device rate (~62 MB per 3:35 stem,
    ~435 MB for seven). Further options: drop decoded copies as soon as they are
    resampled, reserve exact capacity, decode with a bounded thread pool, stream
    from disk.
  - Disk: models are the heaviest files. Download only on demand into the cache
    folder, prefer quantized weights (q8_0 GGUF is 188 MB vs ~700 MB for the fp32
    checkpoint, with inaudible difference), keep one default model, and show in
    settings what is installed and how much it takes, with a delete button.
  - Stems: Ogg Vorbis by default (~25 MB per 6-stem song), FLAC as an option
    (~150–200 MB); silent stems (e.g. `other` in many songs) could be skipped
    instead of written.
  - Binary: 14 MB release build today; check `opt-level`, LTO, `strip` and
    `codegen-units = 1` for distribution.

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

- The catalog is `crates/separation/models.toml`, embedded in the binary: id, name, file, size, sha256, URL(s) with mirrors (pinned to a commit), stem names/order, license. The `burn` backend will add the architecture and the associated `.yaml`.
- On-disk location: `models` in the OS cache folder (`directories` crate; `%LOCALAPPDATA%\Watershed\cache\models` on Windows).

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
| Downloads | `ureq` (blocking, rustls), `sha2` for verification |
| Time-stretch / pitch | Signalsmith Stretch bindings (MIT) |
| FLAC encoding | `flac-codec` (pure Rust, 24-bit) |
| WAV export | `hound` |
| Paths | `directories` |
| Thread communication | `crossbeam-channel`, `rtrb` (RT ring buffer), atomics |

## 9. Distribution

- One binary per platform, statically linking as much as possible. Per platform:
  - **Windows:** a single `.exe` is realistic. Rust links statically by default;
    add `+crt-static` so it does not need the Visual C++ runtime. GPU through
    DX12/Vulkan, whose libraries ship with the driver. Avoid anything that needs
    extra DLLs (e.g. CUDA).
  - **macOS:** a `.app` bundle (a folder that Finder shows as one file) with the
    binary, `Info.plist` and the icon. Possibly a universal binary (arm64 + x86_64).
    Without signing and notarization (Apple developer account, 99 USD/year),
    Gatekeeper blocks it on first open and the user must allow it in System
    Settings. Also needed during development (see "Cmd+Tab" in STATUS).
  - **Linux:** the hardest. A plain ELF depends on the glibc version it was built
    against (build on an old distro, in CI, so it runs on newer ones), and the
    window system (X11/Wayland), ALSA/PipeWire and Vulkan libraries are loaded from
    the system. A fully static (musl) binary does not work well with GPU drivers.
    The usual "one portable file" is an **AppImage**; Flatpak is the other option.
- **Portable mode** (all platforms): if a marker file sits next to the binary, keep
  the config, sessions library and models next to it instead of the OS folders, so
  the whole app can live on a USB stick.
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
- [x] Signalsmith benchmark on the Windows machine.
- [x] BSRoformer.cpp on the RTX 3050: Vulkan vs CUDA vs PyTorch CUDA. Decides whether
  Vulkan is good enough as the Windows backend.
- Moved to Phase 3, since it only makes sense once the wrapper is in the build:
  Vulkan SDK + `glslc` in CI, pin a ggml commit.

### Phase 2: Practice tools
In this order:
- [x] Tempo without pitch change, transposition: Signalsmith on the master, latency
  compensation in the displayed position, `reset` + `seek` pre-roll on seek, instance
  created off the audio thread. Master limiter.
- [x] Saved A-B loops (stored in the session file), transport buttons
  (start or loop start, ±5 s).
- [x] Session file per song folder (`.watershed.toml`): mixer, tempo/pitch, loops.
- [x] Markers and sections (stored in the session file).
- [x] Waveform zoom.
- [x] Library of recent folders.

### Phase 3: Separation with a temporary backend
- [x] BSRoformer.cpp + ggml (pinned commit) built by `build.rs` with the `cmake`
  crate behind a small `extern "C"` wrapper; Vulkan SDK + `glslc` in CI (Windows,
  Linux), Metal on macOS.
- [x] `trait Separator` + GGUF backend, `separate_song` (decode → separate → 24-bit
  FLAC stems with one common gain, stored in the session and undone on load).
- [x] Model catalog + download + sha256 (resumable, mirrors; no UI yet).
- [x] Full flow: drop an mp3 → separate → open the session.
- [x] ✅ Milestone: replaces MVSEP + Reaper for the basic case (confirmed in a practice
  session).

### Phase 3.5: Polish before Phase 4
Improvements with daily value, decided after the Phase 3 practice session. Priorities:
RAM first, then disk. In this order:
- [x] **Quick fixes.** Clicking the song that is already open in Recent does nothing
  (it used to reopen it and pause playback). A "Reload" button (F5) re-reads the open
  folder (new stems, renamed files), keeping the mixer settings of tracks that are
  still there.
- [x] **RAM.** The user saw ~2.5 GB while separating. Measured per stage, then cut
  (details in `STATUS.md`, "RAM"): ggml-metal's unused flash-attention buffers (GPU
  memory 6.8 → 1.5 GB on Apple Silicon, and faster), shorter pipeline queues and
  earlier frees in the C++ inference, stems stored as packed 24-bit in the player
  (−25 % per stem), and on macOS an allocator that returns large blocks at once.
  Still open: streaming stems from disk, if ever needed.
- [x] **Stem names and colors.** Recognize names in any position of the file name and
  in Spanish too (`bateria`, `voz`, `bajo`, `guitarra`, `teclado`, …). Unknown or
  duplicate stems get colors from an extra palette instead of all gray. Rename a
  track from the UI (saved in the session).
- [x] **Pitch lock per track.** Each track gets a pitch-lock icon; a locked track gets
  the tempo change but not the transposition. Locked by default for drum tracks,
  recognized by name like the stem colors (`drums`, `bateria`, …); the user can toggle
  it on any track (saved in the session). Needs two stereo buses after the mixer: the
  unlocked tracks go through a stretcher with the transposition, the locked ones
  through a second stretcher with the same tempo and no transposition, then both are
  summed before the master. Both stretchers share the same configuration, so their
  latency matches and they stay aligned; the second one costs another ~1 % of a core.
- [x] **Stem offsets.** Stems from other tools can start at different moments. Each
  track gets a lock icon (locked by default; a ↔ button, since 🔒 is pitch lock);
  unlocked, its waveform can be dragged in time. The offset is stored in seconds in the session (independent of the device
  rate). The mixer reads each track at `position - offset`; the song length is the
  latest track end.
- [x] **Settings window.** The app has no settings page yet: options live in the top
  bar (output device) or the start screen (library folder). A "Settings" button (and
  Cmd/Ctrl+,) opens a window that groups them, stored in `config.toml`. First options:
  the **stem output format** (next item), the library folder and the output device.
  Later ones (separation speed, model choice, portable mode) go there too.
- [x] **Stem output format: Lossy or Lossless.** Two choices in the settings window:
  **Lossy** (default: Ogg Vorbis q8, ~256 kbps, ~46 MB per 6-stem song instead of
  ~190 MB; same RAM, since stems are decoded on load) and **Lossless (FLAC)**. The aim
  is a quality the ear cannot tell from the WAV. Encoder: libvorbis (BSD, through
  `vorbis_rs`, which bundles it); symphonia already decodes it. Lossy
  formats decode to float, so they need no common gain against clipping. Only new
  separations use the setting; existing folders are not converted.
- [ ] **Approximate BPM** (section 3.1).
- [ ] **Count-in.** An optional click count before playback starts, so the user can
  come in on time. Its speed is the song's BPM at the current tempo; the bar is set by
  a time signature, **4/4 by default** (one bar of four clicks, the first accented).
  The time signature cannot be detected reliably (and songs like Opeth's change it
  often), so the user sets it: numerator and denominator (e.g. 3/4, 6/8, 7/8). Settings
  next to the transport: count-in on/off and the time signature, saved in the session;
  the BPM comes from the item above and its tap tempo. Plays when playback starts (play
  button, space, ▶ on a loop or marker), not when a loop wraps around. The clicks are
  synthesized (no sample files) and mixed after the stretcher, so they are not
  time-stretched or transposed. Without a BPM yet, the count-in is disabled.
- [ ] **Key detection** (e.g. "A major / F# minor"): chroma (energy per pitch class)
  from the pitched stems, without drums, compared against major and minor key
  profiles (Krumhansl–Schmuckler). Relative keys share the same notes, so the
  major/minor choice is the weak point: show the best guess with its relative, and
  let the user correct it (saved in the session). Shares the STFT with BPM. Pitch
  shifting updates the displayed key.

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
