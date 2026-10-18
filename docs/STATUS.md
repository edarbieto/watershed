# Status

Last updated: 2026-10-17.

## Where we are

- **Phase 0** done: workspace, CI green on three platforms, reference tool.
- **Phase 1** done. Tested by the user on the MacBook: loads in ~1 s; mute, solo,
  faders, pan (value shown), double-click reset, space to play and click-to-seek all
  work with no noticeable latency, alongside Reaper. Output survives a Bluetooth
  headset switching profiles when Reaper arms a track (see below).
- Reference separation works on CUDA (RTX 3050: a ~5 min song in 73–100 s) and on
  Apple Silicon through MPS (M5 Pro: a 4:39 song in 120 s, 2.6× faster than its CPU,
  same output as the CPU to within 1e-5).

## Next steps

1. Spike: Signalsmith Stretch, **done on macOS** (see below). Pending: the same
   benchmark on Windows.
2. Spike: build BSRoformer.cpp with the GGUF model; measure on the MacBook (Metal) and
   on the RTX 3050 (Vulkan), compare with the PyTorch numbers above.
3. Phase 2: tempo/pitch, saved A-B loops, sessions.

## Open questions for the user

- **Windows practice session** (nice to have, not blocking): Reaper on ASIO and
  Watershed on shared WASAPI, both on the Yamaha AG03. Any clicks or dropouts?

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
