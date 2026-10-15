# Status

Last updated: 2026-10-15.

## Where we are

- **Phase 0** done: workspace, CI on three platforms, reference tool.
- **Phase 1** implemented: open a folder of stems, mixer (volume, pan, mute, solo,
  master), seek on waveforms, output device picker with resampling, config persistence.
  The milestone ("replaces Reaper for stem playback") still needs a real practice
  session to confirm.
- Reference separation works: PyTorch + CUDA on an RTX 3050 6 GB separates a ~5 min
  song in 73–100 s.

## Next steps

1. User test of Phase 1 in a real practice session (see questions below).
2. Spike: Signalsmith Stretch on the stereo mix in real time; measure CPU and latency.
3. Spike: build BSRoformer.cpp with the GGUF model; measure on the RTX 3050 (Vulkan)
   and on the MacBook (Metal), compare with the PyTorch numbers above.
4. Phase 2: tempo/pitch, saved A-B loops, sessions.

## Open questions for the user

- **Practice session test:** Reaper on ASIO and Watershed on shared WASAPI, both on the
  Yamaha AG03. Does it sound right? Any clicks or dropouts while playing, seeking,
  pausing or changing faders? How does seeking feel? What is missing or annoying in
  the UI?
- Is it OK that the pan slider has no label (it shows its value on hover)?

## Observations to investigate

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

- `tools/reference/separate.py`: support Apple MPS (`torch.backends.mps`) so the
  reference tool is usable on the MacBook; disable AMP when not on CUDA.
- Sessions (Phase 2) must store the common gain factor used to avoid clipping when
  writing FLAC.
