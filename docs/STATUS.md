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
- **Phase 2.2, A-B loops: done.** Tested by the user: works. Afterwards A was changed
  to always start a new loop (clears B) and a Clear button was added.
- **Phase 2.3, session file: done.** Tested by the user: works (see
  "Session file" below).
- **Stereo waveforms, time ruler, ▶ on saved loops: done.** Tested by the user (see
  "Waveforms and ruler" below).
- **Phase 2.4, markers and sections: done.** Tested by the user (see "Markers and
  sections" below).
- **Phase 2.5, waveform zoom: done.** Tested by the user (see "Waveform zoom"
  below).
- **Phase 2.6, library of recent folders: done.** Tested by the user (see "Recent
  songs" below). Also a CPU and RAM display in the top bar.
- **Windows check of Phase 2: mostly done.** The user tried all the Phase 2 features
  on Windows: they work well. Found and fixed one bug there (it also affected macOS):
  see "Resume after changing the mix while paused" below.
- **Spikes** done on macOS (Signalsmith Stretch and BSRoformer.cpp, see below). Their
  Windows measurements moved to the "Before Phase 3" list in `PLAN.md`.
- Reference separation works on CUDA (RTX 3050: a ~5 min song in 73–100 s) and on
  Apple Silicon through MPS (M5 Pro: a 4:39 song in 120 s, 2.6× faster than its CPU,
  same output as the CPU to within 1e-5).

## Next steps

**Phase 2 is feature-complete and works on macOS and Windows.** Then Phase 3.

1. **On the Windows machine:**
   1. ~~Setup~~ done. Signalsmith's bindings need **libclang** at build time:
      installed LLVM (`winget install LLVM.LLVM`) and set `LIBCLANG_PATH` to
      `C:\Program Files\LLVM\bin` as a user environment variable (only terminals
      opened afterwards see it). The first release build takes ~5 min on Windows
      (MSVC linker, Defender scanning `target\`); later ones ~40 s. Excluding the
      project from Defender or using a Dev Drive should help. The Windows CI job is
      also the slowest; it builds three times (clippy, tests, release).
   2. ~~Phase 2 checklists~~ done: the user tried everything new, works well.
      `.watershed.toml` is now hidden on Windows too (see "Session file").
   3. ~~Maximized window~~ not slow on Windows (see "Observations to investigate").
   4. ~~Practice session~~ mostly done: Reaper's metronome and Watershed played
      together on headphones, both on shared WASAPI, with no problems. Still to try
      with the AG03 and Reaper on ASIO (open question below).
   5. Before Phase 3: Signalsmith benchmark
      (`cargo run --release -p watershed-audio --example stretch -- STEMS...`), and
      BSRoformer.cpp on the RTX 3050 with Vulkan vs CUDA (needs CMake, the Visual
      Studio C++ tools and the Vulkan SDK).
2. **macOS, later:** an `.app` wrapper (also fixes Cmd+Tab in development; the
   release `.app` comes with distribution).
3. Ideas recorded in `PLAN.md`: approximate BPM (section 3.1), memory and disk
   optimization (design principles), portable builds (section 9).

## Open questions for the user

- **Windows practice session** (nice to have, not blocking): Reaper on ASIO and
  Watershed on shared WASAPI, both on the Yamaha AG03. Any clicks or dropouts?
  Both on shared WASAPI with headphones already works.

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
- **Pause** fades out over 10 ms. **Resume** seeks the stretcher to the position
  heard at the pause and fades in, so a mix changed while paused is heard at once
  (see "Resume after changing the mix while paused"). **End of
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

## A-B loops

- **Engine** (`stretch.rs`): when the input reaches B it jumps back to A without
  resetting Signalsmith, so the loop sounds continuous. The first 10 ms after the
  jump crossfade (equal power) with what follows B, so it does not click. The
  displayed position maps fed frames back to song frames for the current and the
  previous pass, so it follows the jump ~120 ms later, when it is heard. Only a pass
  that reaches B from before it jumps: playing after B goes on.
- The loop goes to the audio thread as one `AtomicU64` (`start << 32 | end`).
  Minimum length `MIN_LOOP_SECS` = 0.5 s, above the worst-case latency.
- **UI** (`loops.rs`, times in seconds so they survive resampling): A and B buttons
  (keys A, B; B starts looping), Loop toggle (L), drag on a waveform to select a loop
  (a click still seeks; **drag used to scrub**). Turning a loop on while outside it
  jumps to A once the mouse is released. A side panel saves, names, recalls and
  deletes loops. Opening another song clears them.
- A always starts a new loop: sets A at the playhead, clears B and stops looping
  (saved loops stay). Clear forgets A and B.
- Transport buttons: ⏮ back to the start, or to A when a loop is on (Home);
  ⏪ / ⏩ 5 s (arrow keys).

**Test checklist** (passed):
1. Drag over a phrase: it loops right away; the jump at B is smooth, no click.
2. Same at 50–75 % tempo and with pitch changed.
3. A and B while playing (keys and buttons); B starts the loop; L turns it off and
   playback goes on past B.
4. Save two loops, rename them, switch between them, delete one.
5. ⏮ goes to A with a loop on and to 0:00 without; ⏪ / ⏩ move 5 s.

## Session file

`.watershed.toml` in the song folder (next to the first stem), read and written by
`watershed_storage::Session`. It stores `version`, `position`, `tempo`, `semitones`,
the current loop (`loop_a`, `loop_b`, `looping`), one `[[tracks]]` per stem (`file`,
linear `gain`, `pan`, `mute`, `solo`) and the saved `[[loops]]`.

- Tracks are matched by **file name**, so adding or removing a stem keeps the others'
  settings; a stem missing from the file keeps its defaults (instrumental muted).
- **When it is written:** once the state has been unchanged for 1 s after a change
  (so dragging a slider does not write every frame), when another song replaces it,
  and on exit (`App::on_exit`). The position alone is not a change: it is written
  with the next change or on close. Opening a song without changing anything still
  writes the file on exit, because the position moved.
- **Hidden** on every platform: macOS and Linux hide it by its leading dot; on
  Windows the temporary file is created with the hidden attribute, which the rename
  keeps. Moving a folder between platforms breaks nothing: the attribute only
  affects file browsers, and the app finds the file by name.
- Written to a `.toml.tmp` file and renamed, so a crash never leaves half a file.
  An invalid file is logged, ignored and later overwritten. Write errors (read-only
  folder) are logged, not shown.
- On load the position is restored unless it is within 1 s of the end; tempo,
  pitch, gains and pan are clamped to the UI ranges. Default loop names continue
  after the highest "Loop N".
- Master gain is **not** in the session: it belongs to the listening setup, not the
  song. It still resets on restart (could go in `config.toml` later).
- Not yet stored: markers (next step) and the common gain factor of separated stems
  (Phase 3, add a field with a default of 1.0).

**Test checklist** (passed):
1. Open a song, change faders, pan, mute/solo, tempo and pitch, save two loops,
   leave a loop on, quit with Cmd+Q. Reopen the folder: everything is back, including
   the position.
2. Open another song while the first is loaded, then go back: the first one's
   changes are kept and the second starts with its own (or default) settings.
3. A song with no session starts at 100 %, 0 st, all faders at 0 dB.
4. Delete or rename a stem file: the others keep their settings.
5. Switch the output device to one with another rate while a song is loaded:
   nothing is lost.

## Waveforms and ruler

- Each track shows **left on top, right below** (`Peaks::stereo`, one `Peaks` per
  channel). Mono files are duplicated to both channels by the decoder, so they show
  two identical halves. Files with more than two channels (5.1) keep only the first
  two (front L and R) when decoded, so the display never has more than two; note that
  this drops the center channel, where a 5.1 mix usually puts the vocals. A proper
  downmix is only worth it if someone loads surround files.
- **Ruler** above the tracks: `m:ss` labels every 1 s to 10 min (at least 60 points
  apart) with a short tick in between, the playhead as a triangle and the loop region.
  A click moves the playhead (it plays if it was playing); a drag scrubs, seeking only
  when the pointer moves (each seek resets the stretcher). It takes its x range from
  the first waveform, so both always line up. Markers will go on it later.
- **▶ on a saved loop** recalls it, jumps to its start and plays. Before, it only set
  the loop, which did nothing audible while paused or when the playhead was already
  inside it.

**Test checklist** (passed):
1. A song with a stem panned to one side shows it in one half only.
2. Click on the ruler while paused, then Space: plays from there. Click while
   playing: jumps. Drag on the ruler: the playhead follows.
3. ▶ on a saved loop, paused or playing, anywhere in the song: plays from its start
   and loops.

## Markers and sections

Design agreed with the user beforehand: named points placed with a button, the ruler
and Reaper's shortcuts; a section runs from a marker to the next one or to the song
end; each has ▶ and loop, like playing a section in Ableton.

- `markers.rs`: `Markers` keeps them in time order. A marker within 0.1 s of another
  is not added (pressing M twice does not stack them). Default names "Marker N"
  continue after the highest N, like loops.
- **Adding:** `M` (Reaper's default) or the Add button at the playhead, or a
  double-click on the ruler.
- **Jumping:** keys 1–9 go to markers 1–9, numbered by time order (in Reaper they
  keep the number they were created with; by position is easier to read here).
- **Drawn** in a second row of the ruler (number and name, clipped at the next
  marker) and as faint lines across the waveforms.
- **Side panel**, above the loops, one row per section: number, ▶ (play from here;
  turns an active loop off when the marker is outside it, or playback would be caught
  by it), 🔁 (loop up to the next marker or the end, and play), name, time, delete. The
  section under the playhead is highlighted.
- No stored `[Start]` marker: when the first marker is after 0:00 the panel shows an
  implicit `[Start]` row, with ▶ and 🔁 but no name or delete.
- Stored in the session as `[[markers]]` (`name`, `time`).
- Not yet: moving a marker by dragging it (comes with waveform zoom, to place it
  precisely) and previous/next marker keys.

**Test checklist** (passed):
1. Press M while playing at a few points, rename them in the panel: they show in the
   ruler and the panel in time order, and survive closing and reopening.
2. Double-click the ruler: a marker appears there.
3. Keys 1–9 jump to the markers.
4. ▶ on a section plays from its start; 🔁 loops it up to the next marker (and the
   last one up to the end). `[Start]` works the same.
5. With a loop on, ▶ on a section outside it plays on without being pulled back.
6. Delete a marker: the sections before and after merge.

## Waveform zoom

- `view.rs`: `View` is the visible window in seconds (`start`, `span`), from the
  whole song down to 2 s (the peaks have one bucket per 256 frames, ~6 ms; closer
  would only show blocks). Reset to the whole song when a song is opened; not saved.
- **Zoom:** Cmd + scroll or a trackpad pinch over the ruler or the waveforms, around
  the pointer; `+` / `-` keys (Reaper's) and the − / + buttons, around the playhead
  (or the center if it is off screen), by 1.5×; Fit shows the whole song.
- **Scroll:** Shift + wheel or two fingers sideways, or the scrollbar under the
  ruler (drag the thumb, or click beside it to center there). Plain vertical scroll
  still scrolls the track list.
- **Following the playhead**, like Reaper's continuous scrolling: while playing,
  the view stays centered on the playhead, except near the start and end of the
  song, where the view stops and the playhead moves. While paused the view is free;
  a jump outside it (seek, marker key, ⏮) centers it there. (First version paged
  like Reaper's page mode; the user preferred continuous.)
- **Smooth playhead** (`playhead.rs`): the engine updates its position once per
  callback (~11 ms at 512 frames, larger and irregular on Bluetooth), which made the
  playhead and the centered view stutter at full zoom (2 s ≈ 5 points per callback).
  The UI now draws a clock that advances with real time × tempo and is pulled
  towards the engine's position with a 0.1 s time constant; jumps over 0.15 s (seek,
  loop) and pauses take the engine's position at once. Only drawing and following
  use it; actions (A, B, markers) still use the engine's position.
- Waveforms, ruler ticks (1 s to 10 min, at least 60 points apart), markers, loop
  region and clicks all use the visible range, so seeking, drag-to-loop and
  double-click-to-add-marker are as precise as the zoom.

**Test checklist** (passed):
1. Cmd + scroll and pinch zoom around the pointer; `+` / `-` and the buttons around
   the playhead; Fit goes back to the whole song.
2. Shift + scroll, two-finger swipe and the scrollbar move the view; vertical scroll
   still scrolls the tracks.
3. Zoomed in, scroll elsewhere and press play: the view centers on the playhead and
   moves with it; near the start and end it stops and the playhead moves.
4. Zoomed in, press a marker key or ⏮ for a point off screen: the view jumps there.
5. Zoomed in, drag a loop and double-click the ruler: they land where the pointer is.
6. At full zoom while playing (also at 50 % tempo and on Bluetooth): the playhead and
   the waveforms move smoothly.

## Recent songs

- `Config::recent` in `config.toml`: up to 20 song folders (`path`, `opened` in Unix
  seconds), most recent first, no duplicates. A folder is added when it finishes
  loading (not when the load fails).
- **Start screen** (no song open): the drop hint plus a "Recent songs" list with
  name, parent folder (home shown as `~`), how long ago, and 🗑 to remove it from the
  list (the files stay). Folders that no longer exist are shown dimmed and cannot be
  opened.
- **Recent** menu in the top bar, next to "Open folder…", to switch songs without
  going back to the start screen; the open song has a filled radio circle, as in
  macOS menus. It has "Clear list".

## CPU and RAM display

Always visible in the top bar, left of the output device: `CPU 0.1 %   RAM 836 MiB
(3.4 %)`, refreshed every second (`usage.rs`, `sysinfo` crate with only its `system`
feature).
- Measured **per process**, which is what the OS can measure: one process may own
  several windows and threads (UI, audio, loading), and they all count. Watershed has
  one window, so per process and per window are the same here.
- CPU is a share of the whole machine (all cores), like Windows Task Manager; macOS
  Activity Monitor instead shows a share of one core (up to 100 % per core).
- RAM is the resident memory (what `ps` reports as RSS). Activity Monitor's "Memory"
  column is a different measure (memory footprint) and can differ.
- With the 7 MVSEP stems loaded: ~840 MiB, consistent with the f32 stem storage
  noted under "Load spike when opening stems".

**Test checklist** (passed):
1. Open two or three songs: they appear in Recent, latest first, and on the start
   screen after restarting the app.
2. Open a song from the menu and from the start screen.
3. Rename or move a song folder: it shows dimmed and cannot be opened; 🗑 removes it.
4. Clear list empties the menu.
5. The open song has the filled circle in the Recent menu.
6. CPU and RAM update every second, also while paused.

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

### Resume after changing the mix while paused (fixed)

Reported by the user on Windows: all tracks on, play, pause, solo one track, play:
all tracks sounded for a split second before the solo applied.

**Cause:** the stretcher holds about 120 ms of already mixed audio (its latency).
Pausing kept that audio, mixed with the old mute/solo/fader settings, and resuming
played it first.

**Fixed:** resuming from silence seeks the stretcher to the position heard at the
pause, so it refills with the current mix (`CallbackState::process`). A seek also
jumps the track gains to their targets instead of ramping them across the pre-roll
(`Voice::snap_gains`). Test: `mix_changes_while_paused_are_heard_at_once`.

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

### Cmd+Tab loses the window (macOS, `cargo run`)

Reported by the user, with BetterCmdTab (open source) as the app switcher:
after Cmd+Tab the Watershed window goes to the end of the list, and choosing it there
opens Terminal and starts a second Watershed.

**Likely cause** (not verified): `cargo run` starts a bare Mach-O executable, not an
`.app` bundle. macOS gives it a window and a Dock icon, but it has no bundle
identifier, so app switchers that track apps by bundle cannot match the window to an
app; "activating" it then opens the executable file, which macOS does by launching it
in Terminal. **Fix to try:** a small script (or `cargo-bundle`) that wraps the release
binary in `target/release/Watershed.app` and starts it with `open`. The `.app` is
needed for distribution anyway (PLAN section 9). Check with the system Cmd+Tab too.

### Slow UI when the window is maximized

Reported by the user: maximizing the window is slow, playing or paused. Only
**during** the maximize animation on macOS, and not at all on Windows, so it is
likely how macOS resizes the window surface frame by frame (live resize), not our
drawing. Low priority; if it is ever worth looking at, start there. Candidates: the cost of a frame grows with the width (each waveform
draws one line per column per channel: ~2,500 columns × 2 × 7 tracks ≈ 35,000 line
segments, retessellated every frame while playing); or the resize itself (wgpu
surface reconfiguration). Next step: measure the frame time at both sizes, then
consider drawing each waveform as one mesh or caching it while the view is unchanged.

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
