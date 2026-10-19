# Status

Last updated: 2026-10-19.

## Where we are

- **Phase 0** done: workspace, CI green on three platforms, reference tool.
- **Phase 1** done. Tested by the user on the MacBook: loads in ~1 s; mute, solo,
  faders, pan (value shown), double-click reset, space to play and click-to-seek all
  work with no noticeable latency, alongside Reaper. Output survives a Bluetooth
  headset switching profiles when Reaper arms a track (see below).
- **Phase 2.1, tempo and pitch: done.** The user listened in the app on the MacBook:
  sounds good. See "Tempo, pitch and limiter" below.
- **Spikes** done on macOS (Signalsmith Stretch and BSRoformer.cpp, see below). Their
  Windows measurements moved to the "Before Phase 3" list in `PLAN.md`.
- Reference separation works on CUDA (RTX 3050: a ~5 min song in 73–100 s) and on
  Apple Silicon through MPS (M5 Pro: a 4:39 song in 120 s, 2.6× faster than its CPU,
  same output as the CPU to within 1e-5).

## Next steps

1. **Phase 2**, in this order:
   1. Saved A-B loops, plus transport buttons for seeking (start, ±5 s).
   2. Sessions and library.
2. On the Windows machine when available (before Phase 3): Signalsmith benchmark;
   BSRoformer.cpp on the RTX 3050 with Vulkan vs CUDA.

## Open questions for the user

- **Windows practice session** (nice to have, not blocking): Reaper on ASIO and
  Watershed on shared WASAPI, both on the Yamaha AG03. Any clicks or dropouts?

## Tempo, pitch and limiter

Signal path: stems → mixer → Signalsmith Stretch → master gain and pause fade →
limiter → hard clip → device.

- `crates/audio/src/stretch.rs`: `Stretcher` wraps Signalsmith, configured like its
  default preset (block 120 ms, interval 30 ms). It reads the song at `tempo` song
  frames per output frame and keeps the song position as an `f64` so slow tempos do
  not drift. Created in `Engine::start` (UI thread); the engine is rebuilt per device
  anyway.
- **Displayed position** = song frame being heard = input fed − input latency −
  (output latency + limiter lookahead) × tempo. A test puts an onset in the song and
  checks the position reported when it becomes audible: off by 0–1.3 ms at tempos
  0.5, 1.0 and 1.25.
- **Seek and load:** `reset()`, then `seek()` with one block + one interval of
  pre-roll ending at `target + input latency`, so the first output is centered on the
  target. Sound starts within the output latency (~60 ms).
- **Pause** keeps the stretcher state, so resuming continues seamlessly. **End of
  song:** stops when the heard position reaches the end, so the latency tail plays.
- **Limiter** (`limiter.rs`): peak limiter at −1 dBFS with 3 ms lookahead and 100 ms
  release. Quiet material passes through unchanged (only delayed); tests check a
  +6 dB sine and isolated spikes stay under full scale. The hard clip stays as a last
  line of defense.
- UI: Tempo 50–125 % and Pitch ±12 semitones in the transport bar, double-click
  resets. Tempo, pitch and master gain now survive a device switch or restart (the
  master used to reset to 0 dB).
- The stretcher always runs, even at 100 % and 0 st. If the sound at 100 % is not
  transparent enough, a bypass with a crossfade is possible but costs complexity
  (the latency changes by ~120 ms when switching).

**Listening checklist** (passed; kept for regressions. On the interface or wired output; Bluetooth in headset mode
runs at 16 kHz mono):
1. At 100 % / 0 st: does it sound like before?
2. Slow down to 50–75 % and transpose ±1–2 st while playing: smooth, no clicks?
3. Click to seek, arrow keys, stop and play: starts quickly, cursor in sync?
4. Solo a hot stem (drums) and raise it +6 dB: no harsh clipping?
5. Play to the end: the last notes are not cut off?

## Spike: Signalsmith Stretch

`crates/audio/examples/stretch.rs` sums the stems into one stereo mix and runs it
through Signalsmith Stretch 1.3.1 (`signalsmith-stretch` crate, MIT) the way an audio
callback would: fixed output blocks, `block × tempo` input frames per call.

```sh
cargo run --release -p watershed-audio --example stretch -- [--block N] [--wav DIR] STEMS...
cargo run --release -p watershed-audio --example stretch -- --play STEMS...   # t 0.75, p -1, s, q
```

Measured on the MacBook (M5 Pro), a 4:39 song at 48 kHz, blocks of 512 frames
(10.7 ms budget per call):

| Preset | Latency in + out | CPU (one core) | p99 per call | Max per call |
|---|---|---|---|---|
| default | 60 + 60 ms | 0.43–0.58 % | 0.14–0.19 ms | 0.2–2.3 ms |
| cheaper | 50 + 90 ms | 0.31–0.40 % | 0.05–0.06 ms | 0.07–0.16 ms |

- Tempo 0.5–1.25 and ±2 semitones cost about the same; tempo 1.0 with no pitch change
  is the cheapest. With blocks of 64 frames (1.3 ms budget) `default` peaks at ~0.2 ms:
  it does a whole FFT step every 30 ms, while `cheaper` spreads it across calls.
  The rare ~2 ms maxima are probably OS preemption, not the algorithm.
- Live on the built-in output (44.1 kHz), changing tempo and pitch while playing:
  2 % of one core, slowest callback 0.76 ms, no glitches reported by cpal.
- The user listened to the renders and the live mode: sounds good.
- **Conclusion so far:** CPU is a non-issue, real-time works; no offline pre-render
  needed. Use `default` unless a slower machine says otherwise.
- **Latency** (~120 ms) only delays how fast a tempo/pitch change is heard and must be
  subtracted from the displayed position. On seek: `reset()` then `seek()` with one
  block + one interval of pre-roll so the output starts right away.
- `process` does not allocate after `configure` (its scratch `Vec`s are sized there),
  so it can run in the callback. Creating the instance allocates: do it on the UI
  thread and send it through the command queue.
- The crate generates its bindings with `bindgen`, which needs **libclang** at build
  time (Xcode tools on macOS; CI runners have it). If that becomes a problem, replace
  it with a small hand-written C wrapper built with `cc` only.

## Spike: BSRoformer.cpp + GGUF

[BSRoformer.cpp](https://github.com/chenmozhijin/BSRoformer.cpp) (MIT, built on ggml)
with the q8_0 GGUF of BS-RoFormer SW (188 MB, from the FractalSurfer mirror). Built
outside the repo with CMake against the latest ggml (the project pins no ggml version);
on macOS ggml enables Metal and Accelerate by default.

```sh
cmake -B build -DGGML_CUDA=OFF -DCMAKE_BUILD_TYPE=Release   # ggml cloned next to it
cmake --build build --config Release --parallel
bs_roformer-cli model.gguf song.wav out.wav                 # writes out_stem_0..5.wav
```

The CLI only reads 44.1 kHz WAV and names outputs `stem_N`; the order comes from the
`.yaml` (bass, drums, other, vocals, guitar, piano).

Measured on the MacBook (M5 Pro), the same 4:39 song as the PyTorch reference, model
defaults (chunk 352800, overlap 2, same as the reference `.yaml`):

| Backend | Time | Memory |
|---|---|---|
| BSRoformer.cpp, Metal | **79 s** (83 s with model load) | 1.9 GB RSS, 2.4 GB peak |
| BSRoformer.cpp, CPU only, 4 threads (its default) | 984 s | 3.1 GB RSS |
| BSRoformer.cpp, CPU only, 15 threads (patched) | ~450 s (estimated) | |
| PyTorch, MPS (reference) | 120 s | |
| PyTorch, CPU (reference) | ~310 s (MPS is 2.6× faster) | |

- **CPU threads:** BSRoformer.cpp never sets a thread count, so ggml uses its default of
  4 on a 15-core machine. A local patch (`ggml_backend_cpu_set_n_threads`) on a 60 s
  clip: 4 threads 248 s, 10 threads 124 s, 15 threads 114 s. Our wrapper must set it
  (e.g. to the number of performance cores). Even then the CPU path is ~1.5× slower
  than PyTorch on CPU: fine as a fallback, the GPU is the real path.
- **Quality:** compared with the PyTorch FLACs after fitting one gain per stem (the
  reference was scaled by a common −1.1 dB to avoid clipping): SDR between the two is
  36–43 dB for bass, drums, vocals, guitar and piano, i.e. the q8_0 quantization is
  inaudible. `other` is silent in this song (−99 dB RMS), so its 16 dB is meaningless.
- GGUF stems also exceed full scale (drums +1.1 dBFS, guitar +0.3 dBFS): the common
  gain factor is still needed when writing FLAC.
- **Integration looks simple:** the C++ API is `Inference(model_path)` and
  `Process(interleaved_stereo, chunk_size, num_overlap, progress_cb, cancel_cb)`, which
  returns one interleaved buffer per stem. Plan: a small `extern "C"` wrapper, built
  from `build.rs` with the `cmake` crate, ggml linked statically.
- Open points for Phase 3: pin a ggml commit; CI build cost (Metal is free, Vulkan on
  Windows/Linux needs the Vulkan SDK and `glslc` at build time); how backends are
  chosen (ggml picks the best available, `BSR_FORCE_CPU=1` forces CPU).

## Observations to investigate

### Output lost when another app opens a Bluetooth headset's mic

Reported by the user: arming a track in Reaper on the MacBook, with Bluetooth
headphones as output, made Watershed show "The audio device stopped working".

**Cause:** opening the headset's mic switches it from the music profile (A2DP,
44.1 kHz stereo) to the headset profile (HFP, 16 kHz mono). CoreAudio reports the rate
change and cpal turns it into `ErrorKind::StreamInvalidated`; the engine treated every
stream error, even an underrun, as fatal.

**Fixed:** the engine now sorts errors. `StreamInvalidated` makes the app restart the
engine on the same device and resample the stems if the rate changed, keeping position,
mixer settings and play state. Underruns are counted and logged, not fatal. Tested by
changing the device rate from a script (44.1 → 16 → 44.1 kHz).

**Limitation:** eframe does not run the UI while its window is covered by another
window on macOS, so the restart waits until Watershed is visible again. Fixing that
means restarting inside the engine (its own thread, real-time resampling when the rate
differs from the song). For practice, choosing the laptop mic (or the interface) as the
recording input avoids the profile switch entirely.

### Load spike when opening stems

Reported by the user: it only happened with `cargo run`, not with the release build.

**Cause, fixed:** `cargo run` builds in debug, and only dependencies were optimized
in the dev profile. Our own decoding and mixing code ran unoptimized: decoding the
7 stems took 6.4 s instead of 1.7 s, with 7 threads saturating the CPU meanwhile.
`Cargo.toml` now builds `watershed-dsp` and `watershed-audio` at `opt-level = 3`
in dev too; the debug app now loads in ~1 s like the release build.

The rest of this section still applies to the release build. Measured on Windows (16 logical CPUs) with 7 MVSEP stems of
3:35, resampled 44.1 → 48 kHz:

| | |
|---|---|
| Load time | ~1 s |
| CPU | ~3.7 CPU-seconds in ~0.75 s, about 5 cores at 100% |
| RAM | peaks at 1.07 GB, settles at 720 MB |
| Idle after load | 0% CPU |

Remaining costs, from the code (worth improving, not urgent):
- `loader.rs` spawns **one thread per file**, all decoding and resampling at the same
  time. With many stems this saturates the CPU and can starve the audio thread or the
  UI. Fix: a bounded pool (e.g. `available_parallelism() / 2`) or decode sequentially.
- Every stem is kept in memory as **f32 stereo at the device rate**:
  3:35 × 48 kHz × 2 ch × 4 bytes ≈ 82 MB per stem, ×7 ≈ 580 MB.
- The peak is higher because, per stem, the decoded 44.1 kHz copy and the resampled
  copy exist at the same time, and the decode `Vec` grows by doubling. Fixes: reserve
  the exact capacity from the codec's frame count; drop the decoded copy as soon as
  it is resampled; consider storing stems as `i16` (half the RAM) or streaming from disk.
- Loading a new song while another plays also pauses playback (`Engine::load`).

### Track colors

Not random. They are **fixed per stem type** (`StemKind::color` in
`crates/app/src/stems.rs`): vocals red, guitar orange, bass green, drums blue,
piano purple, other cyan. Instrumental and unrecognized names are gray.

There is **no limit on the number of tracks**, but:
- All unrecognized stems share the same gray, so a folder of arbitrary names
  (e.g. `click.wav`, `keys2.wav`) looks monochrome.
- Two stems of the same type (e.g. two guitar stems from different models) get the
  same color.

Possible fix: keep the fixed colors for known types and give unknown or duplicate
stems colors from an extra palette, cycling if needed; later let the user pick a color
per track (saved in the session).

### Other pending items

- Sessions (Phase 2) must store the common gain factor used to avoid clipping when
  writing FLAC.
- **Clipping, explained** (asked by the user): stems exceed 0 dBFS because the model
  works in float and stems partly cancel each other; together they add back up to about
  the original mix. MVSEP mp3s do it too (mp3 decodes to float) and nothing warns about
  it because it is not an error. It only clips in integer formats (fix: common factor,
  undone on load) and at the device output when a hot stem plays alone at full volume
  (fix: master limiter, planned with tempo/pitch).
- **Why PyTorch beats ggml on CPU** (likely, not verified): PyTorch runs the matrix
  maths through Accelerate, which uses Apple's matrix hardware; ggml's q8_0 CPU kernels
  target LLM inference and do not. Irrelevant for the decision: the GPU path is the
  real one, and Phase 4 replaces this backend.
- **Separation progress bar:** BSRoformer.cpp's `Process` takes progress (0–1) and
  cancel callbacks; the Phase 3 UI shows a bar with a Cancel button.
