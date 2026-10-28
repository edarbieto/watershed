# Status

Last updated: 2026-10-28.

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
- **Phase 3.1, BSRoformer.cpp in the build: done.** CI green on all three platforms (see
  "BSRoformer.cpp in the build" below). Separating from Rust gives output
  bit-identical to the spike's CLI, at the same speed.
- **Phase 3.2, `trait Separator` + GGUF backend: done** (see "Separator and stem
  files" below). A real 4:03 song on Windows: 145 s from mp3 to a folder of six
  24-bit FLAC stems plus its session. The user checked the result in the app: works.
- **Phase 3.3, model catalog + download: done** (see "Model catalog and downloads"
  below). The example downloads the default model on first use (188 MB in ~30 s,
  sha256 verified) and then separates with it.
- **Phase 3.4, separating from the app: done** (see "Separating in the app" below).
  Tried on Windows: dropping an mp3 separates it into `~/Music/Watershed/<title>`
  and opens it; dropping it again opens the existing folder. Cancel and separating
  while practicing another song also checked by the user. Peak RAM while separating:
  ~2.5 GB (to reduce in Phase 3.5).
- **Phase 3.5, quick fixes: done.** A Reload button (F5) re-reads the open folder:
  the session is written first, so tracks whose file is still there keep their mixer
  settings (a renamed file is a new track), and the position, loops, markers, zoom
  (if the length did not change) and playback carry over. Tested by the user: works.
  Clicking the open song in Recent now does nothing; before, it reopened the folder,
  which paused playback.
- **Phase 3.5, RAM: done on macOS** (see "RAM" below). Separating the 4:39 test
  song: GPU memory 6.8 → 1.5 GB, process peak 2.7 → 1.7 GB, 77–79 → 72 s, stems
  bit-identical. Playing: −20 % (stems stored as 24-bit). Tested by the user in the
  app: sounds the same, stays fluid, and the fans no longer spin up while separating.
  **Measured on Windows** (see "RAM"): the 4:03 test song in 109 s (was 145 s with the
  same example in Phase 3.2), process peak 1.4 GB, GPU memory 1.2 GB of the RTX
  3050's 6 GB with nothing spilled to shared memory.
- **GPU usage in the top bar: done** (see "CPU, GPU and RAM display" below). Tested
  by the user on macOS and Windows (GPU and RAM match Task Manager); Linux only
  compile-checked.
- **Phase 3.5, stem names and colors: done.** Tested by the user (see "Stem names and
  colors" below).
- **Phase 3.5, pitch lock per track and stem offsets: done.** Tested by the user:
  work well (see "Pitch lock" and "Stem offsets" below).
- **Phase 3.5, settings window and stem format: done.** Tested by the user on macOS
  (see "Settings and stem format" below), along with pitch lock, offsets with the
  trackpad and renaming. New separations write Ogg Vorbis by default: 24 MB instead
  of 149 MB for the 4:03 test song; on Metal it took about the usual time (not timed).
- **Phase 3.5, approximate BPM: done.** Tested by the user: passes the checklist
  (see "Approximate BPM" below).
- **Phase 3.5, time signature and beat grid: done.** Tested by the user: works.
  **Snap to the grid: done.** Tested by the user: works well (see "Beat grid" below).
- **Phase 3.5, click and count-in: done.** Tested by the user: timing, count-in and
  controls work. The three sounds were then remodelled on Ableton Live's; the user
  still has to listen to them (see "Click and count-in" below).
- **Phase 3.5, key detection: done.** Checked by the user against the songs they
  know (see "Key detection" below): meant as a rough hint, and that is enough.
- **Phase 3.5, minimum window size and maximizing on macOS: done.** Tested by the
  user: both work (macOS now renders with OpenGL). Open question kept: wgpu vs
  OpenGL on macOS (see "Window size and maximizing" below).
- Reference separation works on CUDA (RTX 3050: a ~5 min song in 73–100 s) and on
  Apple Silicon through MPS (M5 Pro: a 4:39 song in 120 s, 2.6× faster than its CPU,
  same output as the CPU to within 1e-5).

## Next steps

**Phase 3.5** (polish before Phase 4, `PLAN.md` section 10): quick fixes, RAM, stem
names, pitch lock, offsets, settings, stem format and BPM done.

1. The two new ideas in `PLAN.md`: **dynamic tempo**, then the **song info panel**.
   The feasibility experiment (Python, not kept) to redo in Rust: spectral flux of
   the drum stem at 100 frames/s (log-compressed magnitudes, a 0.5 s moving mean
   subtracted, clipped at 0) normalized by its standard deviation; global BPM from
   autocorrelation with a log-Gaussian prior around 120; then Ellis's (2007)
   dynamic-programming beat tracker: score(t) = env(t) + max over lags in [p/2, 2p]
   of score(t − lag) − 680 · ln(lag / p)², backtracked from the best end. Without
   normalizing the envelope (or with a weight of 100) it locked onto double tempo.
   Local tempo = 60 / beat intervals, averaged over 8 beats. Results: Change 133 →
   145 BPM (5.8 % between p5 and p95, beat jitter 2.6 %); the other test songs
   0.9–3.6 % and 1.4–2.4 %.
2. **Windows:** check that nothing changed there (it keeps wgpu) and the minimum
   window size.
3. **Pending, wgpu vs OpenGL on macOS** (see "Window size and maximizing"): retry
   the Rectangle test **on battery**, with both renderers.
3. **Gentler separation (idea, not decided):** the user would like separation not to
   push the machine to the limit. The GPU runs at ~97 % for the whole separation on
   both machines. Options discussed: a pause after each chunk (a "speed" setting:
   e.g. full / balanced / quiet), a lower priority for the separation thread, smaller
   chunks (less GPU memory, same load). Windows showed no memory pressure (no spill
   to shared GPU memory), so this is about heat, fans and a busy GPU, not RAM.
4. **RAM, optional leftovers** (not needed now, see "RAM" below): stream the
   separated stems to disk instead of holding all six (fits with the Ogg Vorbis
   work), fewer input copies in the C++ wrapper (~190 MiB), a computed overlap
   counter instead of a buffer (~95 MiB), separating in a child process.
5. **Windows, nice to have:** the practice session with the AG03 and Reaper on ASIO
   (open question below).
6. **macOS, later:** an `.app` wrapper (also fixes Cmd+Tab in development; the
   release `.app` comes with distribution).
7. Ideas recorded in `PLAN.md`: approximate BPM (section 3.1), disk optimization
   (design principles), portable builds (section 9).

Windows machine setup, for reference: LLVM with `LIBCLANG_PATH` (Signalsmith's
bindings), CMake 4.4, the Vulkan SDK 1.4.363 and the Visual Studio 2026 Build
Tools. The first release build takes ~5 min (MSVC linker, Defender scanning
`target\`), later ones ~40 s; excluding the project from Defender or using a Dev
Drive should help.

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
`name` when the user renamed it, linear `gain`, `pan`, `mute`, `solo`) and the saved `[[loops]]`.

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

## CPU, GPU and RAM display

Always visible in the top bar, left of the output device: `CPU 0.1 %   GPU  12 %
RAM 836 MiB (3.4 %)`, refreshed every second (`usage.rs`, `sysinfo` crate with only
its `system` feature; GPU in `usage/gpu.rs`).
- Measured **per process**, which is what the OS can measure: one process may own
  several windows and threads (UI, audio, loading), and they all count. Watershed has
  one window, so per process and per window are the same here.
- CPU is a share of the whole machine (all cores), like Windows Task Manager; macOS
  Activity Monitor instead shows a share of one core (up to 100 % per core).
- GPU is the busiest engine's share of time (GPUs have separate 3D, compute, copy
  and video engines). Windows: per process, from the "GPU Engine" performance
  counters (PDH, what Task Manager uses), any vendor. Linux: per process, from the
  DRM fdinfo files in `/proc` (what `nvtop` reads): amdgpu, i915, xe and others;
  NVIDIA's proprietary driver has none, shown as `GPU — %` (NVML could be added).
  macOS: **the whole GPU**, other apps included, from IOKit's `PerformanceStatistics`
  (no per-app number exists; no admin rights needed; ~0.1 ms per sample). The
  tooltip says so. Separating shows it near 100 %.
- RAM on macOS is the **memory footprint** (`proc_pid_rusage`), what Activity
  Monitor's "Memory" column shows; elsewhere the resident memory, like Task Manager.
  Both include memory the allocator holds for reuse (see "RAM" below).

**Test checklist** (passed):
1. Open two or three songs: they appear in Recent, latest first, and on the start
   screen after restarting the app.
2. Open a song from the menu and from the start screen.
3. Rename or move a song folder: it shows dimmed and cannot be opened; 🗑 removes it.
4. Clear list empties the menu.
5. The open song has the filled circle in the Recent menu.
6. CPU, GPU and RAM update every second, also while paused.

## RAM

Phase 3.5's first big item. Measured before changing anything, with
`cargo run --release --example separate -- --memory out.csv ...` (`examples/separate.rs`
samples RSS and the macOS footprint every 10 ms and prints peak and end per phase:
loading, decoding, separating, writing, freed). For in-use memory only, run it with
`MallocLargeCache=0` (undocumented macOS variable, for measuring only: see the
allocator note below).

**Separating** (MacBook M5 Pro, the 4:39 test song, `MallocLargeCache=0`):

| | Before | After |
|---|---|---|
| GPU compute buffer | 6682 MiB | 1282 MiB |
| System wired memory while separating | +6.8 GB | +1.5 GB |
| Process footprint, peak | 2698 MiB | ~1690 MiB |
| Time | 77–79 s | 72 s |

Stems bit-identical (md5) after every step. What changed:
- **GPU memory, the big one.** On Apple Silicon GPU buffers are system RAM. ggml-metal
  reserves extra buffers for flash attention "to avoid graph reallocations": ~2.4 GB
  for the vector kernel and an f16 copy of K/V, which our graph never uses (the vector
  kernel needs fewer than 20 query rows), and a padding buffer sized by the K/V row
  stride, which the frequency attention made huge by passing strided views. Fixes:
  `model.cpp` makes K and V contiguous (`ggml_cont`) before `ggml_flash_attn_ext`
  (smaller padding, and faster), and a patched `ggml-metal-ops.cpp` reserves the
  vector and f16 buffers only when they will be used. Likely why the fans no longer
  spin up: the Mac had to compress memory before.
- **Rust outputs not zeroed:** `bsroformer::Model::process` hands C++ uninitialized
  buffers with capacity, sets the length once it returns OK (`wrapper.h` documents
  that every sample is written).
- **Patched `inference.cpp`:** pipeline queues of 1 chunk instead of 3 (−217 MiB, same
  speed: the GPU is the bottleneck), each chunk's STFT input and model output freed
  as soon as they are used, and the final normalization done in place instead of into
  a second set of six stems.
- Same mechanism as before: CMake compiles patched copies, the submodules stay
  untouched, and a missing anchor stops the build.

**Playing** (6:55 song, 6 stems, footprint): 1061 → 850 MiB.
- Stems stored as **packed 24-bit** (`watershed_audio::Samples`): 3 bytes per sample
  instead of 4, each track scaled to its own peak, so the error stays 144 dB under the
  peak (the FLAC stems' own resolution) and stems over full scale are fine. Waveform
  peaks are computed from the f32 samples before packing; a shorter track now shows
  silence past its end instead of its last peak repeated.
- **macOS allocator** (`crates/app/src/alloc.rs`): macOS malloc keeps freed large
  blocks for reuse, and they keep counting in the footprint and RSS until the system
  is short of memory (or for about a minute). Packing the stems freed full f32 copies
  and made the number worse (1579 MiB) until blocks of 4 MiB or more went straight to
  `mmap`/`munmap`. `malloc_zone_pressure_relief` released nothing on macOS 27.

**In the app** (user, MacBook, "Karaoke"): 0.2 GB idle; 0.9 GB with the model loaded;
1.7 GB separating, 2.1 GB peak while writing the stems; then down to 0.9 and about
0.6 GB over a few minutes with the stems open; 0.5 GB when reopening them.

**Windows** (RTX 3050 6 GB, Vulkan, driver 610.62; the 4:03 test song "Illusion Is
Mine", `examples/separate.rs --memory`, GPU sampled with `nvidia-smi` every 250 ms and
the Windows "GPU Process Memory" / "GPU Engine" counters every ~1 s):

| | |
|---|---|
| Time | 108.9 s (separating 105.8 s, writing 2.7 s); Phase 3.2: 145 s |
| Process resident memory, peak | 1372 MiB while separating, 709 MiB writing, 201 MiB after freeing the model |
| GPU compute buffer (ggml log) | 855 MiB |
| GPU dedicated memory of the process | 1207 MiB peak |
| GPU shared memory of the process | 129 MiB peak (no spill: the card had ~2.9 GB free) |
| GPU use | 97 % on average, above 90 % for 95 % of the samples |
| GPU temperature / power | 62 °C / 69 W peak |

So the contiguous K/V patch does not slow Vulkan down; it likely explains part of the
gain (not isolated: the queue and free changes landed in the same build). GPU memory
before the patches was not measured on Windows.

**Left:** right after separating, ~900 MiB of C++ buffers (the wrapper's input copy,
the padded input, the overlap counter, the result) stay in malloc's cache: the C++
side does not use our allocator. macOS trims them after about a minute or under
memory pressure, and the next separation reuses them. Options if it matters: separate
in a child process (frees everything at once, isolates C++ crashes; needs IPC),
accumulate the C++ output straight into Rust buffers, fewer input copies (~190 MiB),
a computed overlap counter (~95 MiB), stream stems to disk.

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

Measured on Windows (Ryzen 7 5700G), 6 MVSEP stems of 3:35 at 48 kHz:

| Preset | Block | CPU (one core) | p99 per call | Max per call |
|---|---|---|---|---|
| default | 512 | 1.1–1.5 % | 0.37–0.56 ms | 0.6–1.6 ms |
| cheaper | 512 | 0.7–1.0 % | 0.12–0.16 ms | 0.2–0.8 ms |
| default | 64 | 1.1–1.4 % | 0.31–0.41 ms | 0.6–3.0 ms |
| cheaper | 64 | 0.8–1.0 % | 0.03 ms | 0.07–1.2 ms |

About 2.5× the CPU of the M5 Pro, still far below the budget (10.7 ms per call at
512 frames). Only `default` with tiny 64-frame blocks has a max (3 ms) above the
1.3 ms budget, once; device buffers on shared WASAPI are ~10 ms, so not a concern.
Same conclusion: keep `default`.

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
defaults (chunk and overlap from the GGUF, same as the reference `.yaml`: chunk
588800, overlap 2; this note used to say 352800, which is only the CLI's fallback):

| Backend | Time | Memory |
|---|---|---|
| BSRoformer.cpp, Metal | **79 s** (83 s with model load) | 1.9 GB RSS, 2.4 GB peak |
| BSRoformer.cpp, CPU only, 4 threads (its default) | 984 s | 3.1 GB RSS |
| BSRoformer.cpp, CPU only, 15 threads (patched) | ~450 s (estimated) | |
| PyTorch, MPS (reference) | 120 s | |
| PyTorch, CPU (reference) | ~310 s (MPS is 2.6× faster) | |

Measured on Windows (RTX 3050, 6 GB; Ryzen 7 5700G), a 5:06 song (44.1 kHz
WAV), same model and defaults, ggml commit `a05c992` (2026-10-15):

| Backend | Separation | Total (with model load and writing) |
|---|---|---|
| BSRoformer.cpp, CUDA 13.4 | **80 s** | 84 s |
| BSRoformer.cpp, Vulkan (first run) | 143 s | 162 s |
| BSRoformer.cpp, Vulkan (second run) | 128 s | 130 s |
| PyTorch, CUDA (reference) | 74 s | |

- **Vulkan is ~1.6× slower than CUDA** on this NVIDIA card, and the first run also
  compiles its shaders (~20 s more). CUDA is about as fast as PyTorch.
- **Quality:** SDR against the PyTorch stems (one gain fitted per stem): CUDA
  31–38 dB and Vulkan 32–39 dB for bass, drums, other, vocals and guitar; piano
  23–24 dB, but it is nearly silent in this song (−44 dB RMS). Both inaudible.
- **Trade-off for the Windows backend:** Vulkan works on any GPU (NVIDIA, AMD,
  Intel) with what the driver already has; CUDA only on NVIDIA and the app would
  have to ship the CUDA runtime DLLs (cuBLAS alone is hundreds of MB). A 5 min
  song in ~2 min (Vulkan) vs ~1.3 min (CUDA). Phase 4 (`burn` + `wgpu`) runs on
  DX12/Vulkan anyway.
- **CPU only** (`BSR_FORCE_CPU=1`, thread count set by a local patch through
  `ggml_backend_set_n_threads`), a 60 s clip of the same song. The Ryzen 7 5700G
  has 8 cores with 2 threads each:

  | Threads | Time for 60 s of audio | A 5 min song (estimated) |
  |---|---|---|
  | 16 | 248 s | ~21 min |
  | 12 | 267 s | ~22 min |
  | 8 | 288 s | ~24 min |
  | 6 | 332 s | ~28 min |
  | 4 (ggml's default) | 456 s | ~38 min |

  It scales with physical cores (4 → 8: 1.6× faster) but barely with the second
  thread of each core (8 → 16: 1.16×), likely limited by memory bandwidth. Per
  thread, the M5 Pro is ~1.8× faster (4 threads: 248 s on the same kind of clip),
  so single-core speed matters too. With 16 threads the CPU is ~16× slower than
  CUDA and ~9× slower than Vulkan. **For the wrapper:** default to the number of
  physical cores, which leaves the machine usable for ~15 % more time. CPU only
  makes sense as a fallback when there is no usable GPU.
- **Build notes (Windows):** Vulkan builds with the default Visual Studio
  generator once the Vulkan SDK is installed (`VULKAN_SDK` set). CUDA 13.4 does not
  integrate with Visual Studio 2026 Build Tools: use the Ninja generator from a
  `vcvars64.bat` shell with `-DCMAKE_CUDA_FLAGS=-allow-unsupported-compiler` and
  `-DCMAKE_CUDA_ARCHITECTURES=86`. Its DLLs are in `CUDA\v13.4\bin\x64`, which
  must be on `PATH` to run (otherwise exit code 0xC0000135, DLL not found).

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

## BSRoformer.cpp in the build

`crates/separation` builds BSRoformer.cpp and ggml from git submodules in `vendor/`,
pinned to the commits the spike measured: BSRoformer.cpp `a7b9625` (upstream has
nothing newer) and ggml `a05c992` (v0.27.0). `build.rs` runs CMake on `cpp/`, which
builds everything as static libraries (Release, even in debug builds) plus
`cpp/wrapper.cpp`, a C interface that catches every C++ exception. Rust side:
`bsroformer::Model` (`load`, `info`, `backend_name`, `process` with progress and
cancel callbacks). GPU backend: Vulkan on Windows and Linux, Metal on macOS.

- **Thread count without forking BSRoformer.cpp:** CMake compiles a copy of its
  `model.cpp` where both `ggml_backend_init_*` calls become
  `watershed_bsr_init_backend()`, defined in the wrapper. It picks the backend (auto
  or forced CPU) and sets the CPU threads, by default the number of physical cores.
- **Portable binaries:** `GGML_NATIVE=OFF` (the default tunes for the build
  machine's CPU); on x86-64, AVX2/FMA/F16C are enabled explicitly (CPUs from 2013 on),
  otherwise ggml falls back to SSE2.
- **Three build problems found and fixed:**
  - The `cmake` crate fills `CMAKE_CXX_FLAGS_RELEASE` with flags that have no `/O2`:
    all C++ was unoptimized and the song took 300 s instead of 129 s. `build.rs`
    now sets CMake's usual release flags.
  - Windows' 260-character path limit: ggml builds its Vulkan shader generator as a
    nested CMake project; under cargo's `target\release\build\<crate>-<hash>\out`
    the MSVC tools failed (`FTK1011`, `C1041`). Fixed with CMake's `EP_BASE`, which
    shortens the nested path by ~50 characters. A checkout in a much deeper folder
    could still hit it.
  - MSBuild left worker processes running that kept `target\` locked; disabled
    with `MSBUILDDISABLENODEREUSE=1`.
- **Results on Windows** (RTX 3050, the same 5:06 song as the spike):
  `examples/separate.rs` on Vulkan: model load 0.3 s, separation **129 s** (the
  spike's CLI: 128 s). The stems are **bit-identical** to the spike's Vulkan output
  and 24–39 dB SDR against PyTorch, as before. CPU only (`--cpu`, 8 threads by
  default), the 60 s clip: **315 s** (the spike: 288 s with 8 threads, 456 s with
  ggml's default of 4), so the thread count applies; the ~9 % gap is likely the
  portable build against the spike's `-march=native`.
- **Build cost:** the first build of the crate takes ~1 min 40 s on this machine
  (most of it the Vulkan shaders), then it is cached in `target/`. CI rebuilds it in
  every job because rust-cache does not keep workspace crates; if that gets slow,
  cache the CMake build with sccache.
- **Open points:** the app binary now needs `vulkan-1.dll` on Windows (installed by
  every GPU driver, but missing on machines with no GPU driver at all); the library
  prints its progress logs to stdout. Linux (apt Vulkan packages) and macOS
  (Metal) build and pass the tests in CI but have not separated a real song yet.
- **CI time** with the C++ build (first run): macOS 5.5 min, Linux 10 min, Windows
  14.5 min (it installs the 300 MB Vulkan SDK each run).

## Separator and stem files

- **`trait Separator`** (`crates/separation/src/lib.rs`): sample rate, stem names in
  output order, a device description, and `separate(input, progress, cancel)`.
  `GgufSeparator` implements it on top of `bsroformer::Model`. GGUF files store
  the stem count but not the names, so the caller passes them from the model
  catalog.
- **`separate_song(separator, song, folder, progress, cancel)`**: decode → resample
  to the model rate → separate → one common gain → `<stem>.flac` (24-bit, one
  thread per stem) + `.watershed.toml` with `stem_gain`. It writes into
  `<folder>.partial` and renames at the end, so a cancel or error leaves nothing
  behind, and it never overwrites an existing folder. Progress comes per stage
  (decoding, separating, writing) for the Phase 3.4 progress bar.
- **Stem gain:** the session's `stem_gain` (linear; 1.0 when nothing was scaled) is
  undone by the loader, so stems play at their original level and hot peaks are
  left to the master limiter. The app keeps it when it saves the session.
- **FLAC encoder: `flac-codec`, not `flacenc`.** `flacenc` (the crate the plan
  named) wrote a 172 MB drums stem for 64.5 MB of raw 24-bit audio: it only has
  4-bit Rice parameters (max 14, the 16-bit-era limit, no RICE2 or escape codes), so
  the residuals of loud 24-bit audio take dozens of bits per sample. It also wrote
  a minimum block size that symphonia refuses. `flac-codec` (pure Rust, MIT/Apache)
  on the same stems: 155 MB for all six (drums 44 MB), bit-exact when read back by
  symphonia, 2–3 s per stem. A test guards against the regression (loud noise must
  not exceed the raw size).
- **Timing** (RTX 3050, Vulkan, 4:03 mp3): load 1.3 s, decode + resample 4.5 s,
  separation 138 s, writing 3 s (sequential, with flacenc; parallel `flac-codec`
  measured separately). A second run was stopped: a game was using the same GPU, which
  slowed separation several times over.
- **Memory:** a 4 min song holds its input (~86 MB) during separation and six stems
  (~515 MB) while writing; the FLAC writer converts in 64k-sample chunks, so
  writing adds almost nothing.

## Model catalog and downloads

- **`crates/separation/models.toml`**, embedded in the binary: id, name, file name,
  size, sha256, mirror URLs, stem order and license per model; exactly one is the
  default. A test checks the file (one default, unique ids and files, hex hashes,
  https URLs, stem names usable as file names).
- **Default model:** BS-RoFormer SW q8_0 from `FractalSurfer/BS-RoFormer-SW-6stem` on
  Hugging Face, the same file as the spike (same sha256), with the URL pinned to
  commit `2b98254` so it cannot change underneath. Only one mirror is known; the
  catalog takes several.
- **Where:** `models_dir()` in `watershed-storage`: `models` in the OS cache folder
  (`%LOCALAPPDATA%\Watershed\cache\models` on Windows,
  `~/Library/Caches/Watershed/models` on macOS).
- **`ModelEntry::install`**: if the file is missing or has the wrong size, downloads
  it with `ureq` (blocking, rustls) into `<file>.part`, hashing while it writes, and
  renames only when size and sha256 match. A leftover `.part` is resumed with a
  range request (Hugging Face supports them); a server that ignores ranges or answers
  416 makes it start over. Mirrors are tried in order; a corrupt download is deleted.
  Cancel keeps the `.part` for next time. "Installed" means the right size: hashing
  188 MB at every start would cost ~0.5 s.
- **Tests** run against a small HTTP server on `127.0.0.1` inside the test (no
  network): full download, resume, server without ranges, mirror fallback, corrupt
  file, cancel.
- **New dependencies:** `ureq` 3 (MIT/Apache) with `rustls` + `ring`
  (Apache-2.0 AND ISC) + `webpki-roots` (CDLA-Permissive-2.0), and `sha2`. No GPL.
  The app does not link them yet (Phase 3.4).

## Separating in the app

- **What separates:** dropping or opening exactly one audio file. A folder, or several
  files, still open as stems. "Separate song…" in the top bar picks a file.
- **Where:** `<library>/<title>`, the library being `Music/Watershed` (home folder if
  there is no music folder), changeable from the start screen ("Change…", stored as
  `library_folder` in the config). The title is cleaned like folder names (MVSEP
  prefixes dropped) and made safe for every OS. If that folder holds a session, it is
  an earlier separation and is opened instead; if it is some other folder,
  `<title> (2)` and so on.
- **Job** (`crates/app/src/separate.rs`): one background thread per song, one song at a
  time. Downloads the model if needed, loads it (and drops it when done, freeing
  VRAM), then `separate_song`. The bar shows the step, percent and time left
  (estimated per step after 3 s); decoding and model loading animate since they
  have no percent. The device (e.g. `Vulkan0 (NVIDIA GeForce RTX 3050)`) is in the
  bar's tooltip.
- **When it finishes:** added to Recent; opened right away unless another song is
  playing, in which case "✔ <title> is separated" waits with an Open button.
- **Times in the app** (RTX 3050): a 4:03 song in 127 s and a 7:18 song in 192 s from
  drop to open.
- **Top bar:** with the new button there is no room for the song title at the default
  width (the right-hand controls take ~830 px), and long titles overlapped the CPU
  label. The title now goes to the window title ("Watershed — <title>") and appears in
  the bar only when there is room, cut short with "…".

## Stem names and colors

- **Recognized by any word** of the file name (`stems.rs`, `StemKind::from_path`):
  words are the runs of letters, lowercased and without accents, so `guitar2`,
  `Guitar 2 - take`, `Batería` and `voz principal` all match. When several words
  match the last one wins (song titles come first: `Bajo la lluvia - guitarra` is a
  guitar). Spanish words: `voz`/`voces`, `guitarra`, `bajo`, `bateria`/`percusion`,
  `teclado`/`teclas`, `otro`/`resto`, `acompañamiento`. `no_vocals` (Demucs) and
  `sin voz` are the instrumental.
- **Colors** (`track_colors`): the first stem of each kind keeps its color; unknown
  stems and repeated kinds take six extra colors (yellow, pink, indigo, mint, tan,
  olive) in track order, then repeat. The instrumental stays gray (it starts muted).
- **Default names** (`default_names`): the kind ("Guitar"), or the file name for
  unknown stems and for kinds that appear more than once, so two guitars from two
  models can be told apart.
- **Rename:** double-click a track name, Enter or click elsewhere to confirm, Escape
  to cancel; an empty name goes back to the default. Saved as `name` in the
  session's `[[tracks]]` (only when it differs from the default) and kept across a
  device switch and Reload. Names live in `LoadedSong`; `Track::name` in the audio
  crate stays the default name.
- Checked in the release build on Windows with a folder of `guitarra`,
  `Guitar 2 - take`, `Batería`, `voz principal`, `teclados` and `click`: Drums,
  Vocals and Piano recognized, the two guitars orange and yellow with their file
  names, `click` pink.

**Test checklist** (passed):
1. Open a folder with Spanish or unusual stem names: kinds and colors as above.
2. Double-click a name, type a new one, Enter: it stays after closing and reopening
   the folder, and after Reload (F5).
3. Escape while renaming keeps the old name; clearing the name restores the default.
4. While typing a name, keys like Space, M or A do not trigger shortcuts.

## Lossy stem codec

Open decision (Phase 3.5, "Stem output format"). The goal is a lossy format the ear
cannot tell from the WAV. At 256 kbps all three candidates are transparent in blind
tests; Opus already is at about 128–160 kbps. The differences are in tooling:

| | AAC 256 kbps | Opus 160 kbps | Vorbis q8 (~256 kbps) |
|---|---|---|---|
| Size, 6 stems of 4 min | ~46 MB | ~29 MB | ~46 MB |
| Encoder | No free one in Rust. fdk-aac (Fraunhofer license: not GPL, but not free per FSF/Debian, with a patent clause) or the OS encoder: macOS AudioToolbox (excellent), Windows Media Foundation (max 192 kbps), none on Linux | libopus, BSD, built with CMake like ggml, same on all OSes | libvorbis (aoTuV), BSD, `vorbis_rs` bundles it, same on all OSes |
| Decoding in the app | symphonia, already enabled (`.m4a`) | symphonia has no Opus decoder: libopus through an adapter (`symphonia-adapter-libopus`) or our own | symphonia, pure Rust |
| Opens in | everything: Reaper, Logic, Ableton, iPhone, Finder, Windows | Reaper, Audacity, VLC, browsers; fewer DAWs | Reaper, Audacity, VLC, most DAWs; not Apple apps |
| Sample rate | 44.1 kHz as separated | always 48 kHz: stems resampled when written (most interfaces run at 48 kHz anyway) | 44.1 kHz as separated |
| Start offset | encoder delay (~1024–2112 samples) removed only if the decoder reads the gapless info: to check with symphonia | 312-sample pre-skip, in the header, always removed | sample-exact |

**Decided: Ogg Vorbis, q8 (~256 kbps).** The user's priorities were DAW
compatibility first, then disk and quality. Opus is smaller (~29 vs ~46 MB per song)
but fewer DAWs import it; Vorbis opens in Reaper, Cubase, Ableton, FL Studio and
Bitwig, and at ~256 kbps it is transparent. AAC would open everywhere (Logic too) but
has no free encoder for all three OSes. Logic and Pro Tools users can pick Lossless.
q6 (~192 kbps, ~35 MB) is still transparent for most listeners, if disk matters more
later.

## Settings and stem format

- **Settings window** (`settings_window` in `app.rs`): the ⚙ button at the right of
  the top bar, or Ctrl+, (Cmd+, on macOS). Output device (the top bar keeps its own
  picker, used daily), the library folder with "Change…", and the stem format. Stored
  in `config.toml` (`stem_format = "lossy"` or `"lossless"`).
- **Stem format** (`StemFormat` in `watershed-storage`): Compact (Ogg Vorbis, the
  default) or Lossless (24-bit FLAC). Only new separations use it; existing folders
  stay as they are, and the player opens both.
- **Ogg Vorbis writer** (`watershed_dsp::write_vorbis`): `vorbis_rs` 0.5 (BSD-3,
  bundles libvorbis with the aoTuV tuning and libogg, both BSD-3, compiled with `cc`;
  no bindgen; default features off, so no `getrandom`), quality 0.8 ("q8", nominally
  ~256 kbps), blocks of 1024 frames, a fixed stream serial. Vorbis stores floating
  point, so no common gain: `stem_gain` is 1.0 and hot stems keep their peaks
  (test: a 1.5 peak decodes as 1.5, and the length is exact).
- `separate_song` takes the format; `examples/separate.rs` writes Ogg Vorbis unless
  `--flac`.
- **Measured** on Windows (RTX 3050), the 4:03 test song: 109.9 s in all (same as
  FLAC); **24 MB** for the six stems vs **149 MB** as FLAC. Average ~130 kbps: q8
  is VBR, and quiet or sparse stems (piano 1.4 MB, bass 2.5 MB) take much less than
  dense ones (drums 7.1 MB). Against the FLAC stems of the same song: same length to
  the sample, peaks above full scale kept (guitar 1.26, drums 1.34), SNR 23–38 dB,
  which is normal for a perceptual codec (the noise sits under the music; SNR does
  not measure what is heard). Opened in the app: loads and plays.
- Not tried by Claude: the settings window on screen (a synthetic click did not
  reach egui).

**Test checklist** (for the user):
1. ⚙ and Ctrl+, open and close the window; the X closes it.
2. Switch the output device from the window: playback moves to it.
3. Change the library folder from the window.
4. With Compact selected, separate a song: `.ogg` stems, about 25–45 MB. Listen
   against the FLAC version of a song (solo a stem in each) for any difference.
5. Choose Lossless, restart the app: still selected; a new separation writes FLAC.

## Approximate BPM

The song's BPM shows right of the tempo slider, at the current tempo (120 BPM at
75 % shows 90).

- **Estimate** (`watershed_dsp::OnsetEnvelope`, `tempo.rs`): per stem, the loader
  computes an onset envelope at ~100 frames per second (Hann-windowed FFT of 2048
  samples, log-compressed magnitudes, spectral flux), from the samples already at the
  device rate; the envelopes of all stems are summed (the instrumental is skipped
  when other stems are there, since it repeats them). The tempo is the period where
  the envelope best matches itself (autocorrelation, 40–240 BPM, weighted toward
  120 BPM so half or double the beat lose), refined with the sum over 16 beat
  periods for sub-frame precision. Under 10 s or silence: no estimate.
- **Cost:** ~110 ms per 4–5 min stem in release, in the stems' own loader threads;
  no extra copy of the audio.
- **Correcting it** (`bpm.rs`): drag or type the value, ½ and ×2 for half or double
  errors, Tap (or T) to tap the beat (from the 3rd tap; up to 8 averaged; a 2 s pause
  starts a new count; taps at 75 % speed are scaled back to the original tempo), ↺
  back to the estimate. The user's value is saved as `bpm` in the session (at the
  original speed); the estimate is not saved, it is computed on every load.
- **Measured** with `cargo run --release -p watershed-dsp --example bpm -- <files>`
  on the MacBook's test songs (BS-RoFormer SW stems, and MVSEP mp3s for the last):

  | Song | Sum of stems | Drums alone | Single stems off |
  |---|---|---|---|
  | Afuera | 125.9 | **63.0** | guitar 189, other 133.6 |
  | On Most Surfaces | 133.7 | 133.7 | vocals 128.9 |
  | Karaoke | 127.0 | 127.0 | other 63.5, vocals 103 |
  | Cada hogar | 150.0 | 150.0 | other, piano, vocals |
  | No Dejes Que… | 134.1 | 134.1 | piano 89.4 |
  | Change (In the House of Flies) | 137.0 | 137.1 | bass, guitar, other, piano |

  The sum agreed with most stems on every song, while the drums alone gave half the
  beat on one; so the plan's "drums stem first" was dropped. Not checked against a
  known tempo yet: the user's ear decides (Change is often listed as ~69 or ~138).
- Tests: clicks at 72, 97, 120 and 143.5 BPM at 44.1 and 48 kHz within 0.5 BPM;
  silence and short audio give none; taps, halving and the reset (`bpm.rs`).

**Test checklist** (passed):
1. Open a few songs you know: is the BPM right, or half/double? (½ and ×2 fix it.)
2. Move the tempo slider: the BPM follows (e.g. 75 % shows three quarters).
3. Tap (button or T) along with the music, also at 75 %: the BPM settles on the beat.
4. Close and reopen the song: your value is kept; ↺ goes back to the estimate.
5. Loading feels as fast as before.

## Beat grid

- **Bar 1** (`OnsetEnvelope::first_beat` in `tempo.rs`): for the song's BPM, the
  phase where a pulse train best lines up with the onsets (searched in quarter
  envelope frames, ~2.5 ms), then the first beat at or just before the first clear
  onset (30 % of the 95th percentile), so silence at the start is skipped. An attack
  shows in the envelope ~1.5 hops before the end of the window it enters: measured
  with clicks, now within ±2.3 ms (`finds_the_first_beat_after_silence`). An attack
  at 0:00 falls in the first window, which cannot see it, so a beat there counts too.
  The loader keeps the summed envelope (`LoadedSong::onsets`, ~100 values per
  second), so bar 1 is estimated again when the user changes the BPM.
- **Grid** (`grid.rs`): bars and beats from bar 1, nothing before it (a free intro
  stays clear). Lines are thinned out when zoomed out: every beat if they are 12
  points apart, else every 1, 2, 4… bars; bar numbers at least 32 points apart;
  beat labels (`12.3`) from 40 points. Drawn behind the waveforms (bars stronger)
  and in a **bar ruler** under the time ruler.
- **Controls**, beside the bar ruler: **Grid** on/off (off hides the grid and greys
  out the BPM and tap tempo), the time signature (beats per bar 1–32, beat unit 2, 4,
  8 or 16; the BPM counts the beat unit), and **1 here** (bar 1 at the playhead).
  Dragging the bar ruler moves the grid; a double-click goes back to the estimate.
- **Session:** `grid`, `beats_per_bar`, `beat_unit`, `grid_start` (only when set by
  the user).
- **Snap** (asked by the user after testing the grid, like Reaper): with the magnet
  on (the default, `snap_to_grid` in `config.toml`), a click on a waveform moves the
  playhead to the nearest grid line drawn, so the zoom sets the precision (bars when
  zoomed out, beats when zoomed in). The time ruler never snaps (click or drag), and
  neither do loop selections. egui's fonts have no magnet emoji, so the toggle is
  drawn (`magnet_button`).
- First beats found on the test songs: Afuera 1.125 s, On Most Surfaces 1.081 s,
  Karaoke 0.334 s, Cada hogar 0.551 s, No Dejes Que 0.341 s. On Afuera, zoomed in,
  the bar lines fall on the kick and the guitar enters right on bar 17.

**Snap test checklist** (passed):
1. Click a waveform with the magnet on: the playhead jumps to the nearest line; zoom
   in and it lands on beats.
2. Drag the playhead along the time ruler: it moves freely.
3. Magnet off: clicks land where you click; restart the app: it stays off.
4. Grid off: clicks never snap.

**Test checklist** (passed):
1. Open songs you know and zoom in: do the bar lines fall on the "1"? If the grid is
   a beat or two off, drag the bar ruler or use "1 here".
2. A song with a free intro: put bar 1 where the beat starts; nothing is drawn
   before it.
3. Change the time signature (3/4, 6/8): the bars regroup.
4. ½ / ×2 on the BPM: the grid follows.
5. Grid off: the grid and the bar ruler empty, BPM greyed out. Close and reopen the
   song: everything is kept.

## Click and count-in

- **Engine** (`crates/audio/src/click.rs`): the click is mixed **after the limiter**,
  so it is neither stretched, transposed nor ducked by the song. It follows the song
  frame being heard (`Stretcher::heard_exact`, fractional and without the seek floor,
  limiter delay included), so a beat clicks exactly when it is heard at any tempo:
  in the test it lands within 0.2 ms of a sine's attack at 0.5×, 1× and 1.25× (at
  slow tempos the stretcher spreads an attack over ~10 ms; the click falls in its
  middle). Beats come from the grid (bar 1 and beat length in song frames, atomics);
  nothing clicks before bar 1. After a seek, beats from the target on click (2 ms
  tolerance, for a playhead snapped onto a beat); after a loop jump, from the loop
  start. A pause lets a click already sounding ring out.
- **Count-in**: when playback starts from silence (play, space, ▶ on a loop or
  marker while paused), the engine seeks, then counts the bar before the playhead on
  the grid, extended backwards (the first click is the accent of that bar), with the
  stretcher frozen. The song starts when the count reaches the frame the stretcher
  will play first, so the count and the song form one continuous pulse (tested). A
  seek during the count-in starts the song at once. `Engine::counting_in()` keeps
  the playhead still meanwhile. Not on loop wraps, nor on seeks while playing.
- **Sounds**, synthesized when the engine starts (no files, any sample rate). The
  first version was designed blind and the user asked to adjust it; the second is
  modelled on **Ableton Live's own metronome samples** (Live 12 Lite installs them:
  `Misc/Metronome/Samples` is "Classic", `Core Library/Defaults/Metronomes/Samples`
  holds Click and Wood), measured for spectrum, envelope and peak level, not copied.
  Measured and matched (accent in brackets):
  - **Click**: a 0.3 ms pulse (0.6), a thump at 240 Hz (186) and a ring at 1050 Hz
    (1600), −20 dB at 3 ms, gone by 10 ms. Peak −6 dBFS.
  - **Metronome** (Live's "Classic"): 1100 Hz (1575) with the 3rd harmonic at −13 dB
    and the 5th at −32 dB, decaying ~1 dB/ms and cut off at 20 ms. Peak −13 dBFS
    (−9.4: the accent is also louder).
  - **Wood**: main mode 1080 Hz (1555; the accent adds a mode at 0.94× that beats
    against it), a 2 ms soft strike, inharmonic attack modes at 1.6×, 2.2× (−13 dB),
    2.7× and 3.4×, −20 dB at 15 ms, then a hollow ring of three close modes around
    −27 dB that fades by 0.3 s. Peak −1 dBFS.
  The peaks follow Live's, which balance a sustained beep against a dry pulse. All
  times the click volume. The user listened: they sound good. Copyright, as
  discussed: only physical measurements (frequencies, decay times, levels) were
  taken, not audio; the samples are not in the repo or the app.
- **UI**, at the right of the transport: **Click** (also the C key) and **Count-in**
  toggles, and a ⏷ menu with the sound, channels (Left / Both / Right) and volume
  (−40 to +12 dB, default −6; asked by the user). The click is mixed after the
  limiter, so above 0 dB its peaks go through a soft clipper (`soft_clip`: linear
  up to 0.5, then a tanh curve towards 1.0): turned far up it gets denser and louder
  instead of being squared off at the output. Disabled without a BPM, with the grid off or before bar
  1 is known. Click and count-in are saved in the session (`click`, `count_in`); the
  sound, channels and volume in `config.toml` (`click_sound`, `click_channels`,
  `click_volume_db`), the same for every song.
- **Not done** (ideas for after listening): the song only in the other ear when the
  click is on one side; showing the count (1 2 3 4) on screen.

**Test checklist:**
1. Turn on Click and play a song you know: is it on the beat, with the accent on the
   "1"? Try 50 % and 125 % tempo and a transposition: it stays on the beat and its
   pitch does not change.
2. Try the three sounds and the volume while playing; is each one pleasant and
   audible over the mix? Which one would you change?
3. Channels Left and Right with headphones.
4. Count-in on: press play with the playhead on a bar line (snap helps): four clicks,
   then the song comes in on the "1". Also from ▶ on a marker or loop, and with the
   Click off (only the count-in sounds).
5. A loop with Click on: no double or missing clicks where it wraps; no count-in on
   the wraps.
6. Grid off or no BPM: both buttons greyed out, silence. Close and reopen the song:
   Click and Count-in are kept; restart the app: sound, channels and volume are kept.

## Key detection

- **`watershed_dsp::Chroma`** (`key.rs`): STFT with a ~0.34 s window (16384 at 48
  kHz, hop half of it), 55 Hz–2 kHz, magnitudes summed into a **10-cent chroma**
  (120 steps); every frame normalized to count the same, frames below −50 dBFS
  skipped. The **tuning** is the circular mean of where energy falls within the
  semitone, taken out before folding to 12 pitch classes (a song at A = 448.6 Hz
  is read right, tested). The key: Pearson correlation with the 24 rotated
  Krumhansl–Kessler profiles; also the runner-up, the confidence and the tuning.
  ~30 ms per stem. `Key` prints and parses `"Eb"`, `"F#m"` (ASCII, any font).
  Example: `cargo run --release -p watershed-dsp --example key -- stems…`.
- **Loader:** one chroma per stem, summed, from the stems that also feed the tempo
  minus the drums.
- **UI:** a **Key** button after Pitch shows the key as heard (transposed by the
  pitch control); hover gives the confidence (clear ≥ 0.8, likely ≥ 0.6, else
  uncertain), the relative and the runner-up. Its menu: the estimate, the relative,
  and all 24 keys. Stored at the original pitch in the session (`key = "F#m"`).
- **On the test songs** (drums left out), with keys found on the web where any:
  Afuera **Bm** (0.86; tabs say Bm ✓), No Dejes Que **G** (0.78; the chords G–Am–Bm–C
  agree, a karaoke site says D), Change **F** (0.59; sites say Cm, uncertain),
  On Most Surfaces Gm (0.63), Karaoke Eb (0.71), Cada hogar Gm (0.47). Variants
  tried: Temperley and Sha'ath (KeyFinder) profiles, square-root magnitudes,
  frames weighted by loudness. All agree on Afuera and Karaoke and move around on
  the low-confidence ones; plain magnitudes with K–K were kept. **User's check:**
  Change roughly right (its chorus and bridge move the key center, so one key per
  song is a compromise there), Karaoke roughly right. With every known key matched,
  the parameters stay as they are: tuning them on four songs would only overfit.
  The user wants a rough hint, not precision. Possible later: a key per section
  (between markers), for songs like Change.

**Test checklist:**
1. For the songs you know: is the key right? If not, is it the relative (same
   notes), a fifth away, or something else? The menu shows what it guessed second.
2. Transpose ±2 semitones: the key follows (Bm → C#m).
3. Correct a key, close and reopen the song: it is kept; "Estimate" goes back.

## Window size and maximizing

**Minimum size** (`update_min_size` in `app.rs`): every frame the app measures what
the top bar and the transport bar need (their widgets laid out in a row, plus the
panel margins, measured before adding the contents: overflowing contents widen the
panel's rectangle) and what the tracks need (controls, a 200 px waveform, the side
panel, the ruler, one row and the scrollbar). When that changes, it sends
`ViewportCommand::MinInnerSize`; AppKit then keeps the user from making the window
smaller, and winit grows it if it already is. Widgets that come and go keep their
space so the minimum does not jump: the loop range (`A → B`) and the BPM's ↺ always
take their width. Result on macOS with a song open: **~1262 × 198** (one track row
shows); the transport bar is the widest part. The window now opens at 1320 × 760.
A narrower window would need the transport bar split into two rows.

**Maximizing** (reported by the user: laggy when maximizing, not in full screen).
Measured:
- Our drawing is cheap: 120 fps and 2–3 ms of CPU per frame at 1100 × 700 and at
  1800 × 1098, and every step of the native zoom animation is drawn.
- The user maximizes with **Rectangle**, which intercepts the title-bar double-click
  and moves, then resizes the window through the accessibility API (two requests).
  Repeating those calls from a script: most are answered in 1–15 ms, but about one
  in seven takes **~305–315 ms** (once 1 s) with the app idle, so the move shows
  before the resize. The same happens with a **bare eframe 0.36 app** with the
  default wgpu (Metal) renderer, and **never with its OpenGL (glow) renderer**. Not
  our code; likely how wgpu's Metal surface and the window interact (not dug
  further). `presentsWithTransaction` on the Metal layer did not change it.
- **Fix:** macOS uses `eframe::Renderer::Glow` (the `glow` feature only on the macOS
  target; it changes `App::on_exit`'s signature there). Watershed with glow: 100 calls,
  none over 26 ms. Playing a song costs **~26 % of a core with glow vs ~40 % with
  wgpu**; idle ~0.5 % for both. OpenGL is deprecated on macOS but still shipped;
  Windows and Linux keep wgpu.

**Follow-up (pending, not settled).** The user would rather use the native path
(wgpu on Metal), and it is odd that OpenGL costs less. What is known:
- Later the same day the stall **no longer reproduced with either renderer** (~500
  calls, none over 50 ms: the bare app, Watershed with wgpu and with glow). The
  user had plugged in the charger in between: on battery macOS may keep the GPU and
  the display in lower power states, which would fit a wait for a Metal drawable.
  **Next test:** on battery, the AX script on Watershed with each renderer (a
  temporary `options.renderer = eframe::Renderer::Wgpu` in `main.rs` switches back).
- The stall never happens while `sample` profiles the process (it wakes it every
  millisecond): the main thread is waiting, not working.
- The CPU difference is stable: playing, wgpu ~40 % of a core, glow ~26 % (four
  alternating runs, plugged in). A profile shows more time in timed waits
  (`semaphore_timedwait`) with wgpu; release builds have no symbols, so not deeper.
- Elsewhere: the same Rectangle symptom is open in winit
  ([#3644](https://github.com/rust-windowing/winit/issues/3644), 200–500 ms, wgpu
  and vulkano, no root cause); slow resizing in wgpu
  ([#7670](https://github.com/gfx-rs/wgpu/issues/7670)); `nextDrawable` taking ~1 s
  after a resize ([Apple forums 773229](https://developer.apple.com/forums/thread/773229),
  Apple suggests `CAMetalDisplayLink`); reports of Metal having a higher base CPU
  cost than OpenGL on Apple Silicon. egui-wgpu's `macos-window-resize-jitter-fix`
  (`presentsWithTransaction`) only covers mouse live resizes. OpenGL is deprecated
  since 2018 (4.1 only) but still works on macOS 26, with no removal date announced.
- Options if it comes back: keep glow (current); back to wgpu and accept the cost;
  try wgpu with a different frame latency or a `CAMetalDisplayLink`-driven loop
  (needs a reliable repro first). The separation backend (Phase 4, burn + wgpu)
  does not depend on the UI renderer.

## Pitch lock

Each track has a 🔒 button next to M and S. A locked track follows the tempo but
not the transposition. Drums (including `percussion`, `bateria`) are locked by
default; the user can toggle any track. Saved as `pitch_lock` in the session's
`[[tracks]]`; sessions from before have no value and use the default for the kind.

- **Two buses** (`mixer.rs`, `Bus`): the mixer renders the unlocked tracks into the
  pitched bus and the locked ones into the locked bus. Each track keeps a gain ramp
  per bus, so toggling the lock while playing fades it from one bus to the other
  within one block instead of clicking.
- **Two stretchers** (`stretch.rs`): two Signalsmith instances with the same
  configuration, so the same latency; only the pitched one gets the transposition.
  Both outputs are summed before the master gain and limiter. Seeking, loop jumps and
  the loop crossfade apply to both buses the same way; the position tracking is
  shared.
- **Cost:** twice the stretcher CPU (measured earlier at ~1–1.5 % of one core on the
  Ryzen, so ~2–3 %). A bus with no tracks still runs: ramps skip silent tracks, but
  Signalsmith processes the silence. Fine at this cost; could be skipped later.
- Tests: a locked 440 Hz sine stays at 440 Hz at +12 st while an unlocked one goes
  to 880 Hz (`pitch_locked_tracks_keep_their_pitch`), and toggling ramps a track out
  of a bus (`pitch_locked_tracks_go_to_the_locked_bus`).
- Checked in the release build on Windows: the button shows, on for Drums only.

**Test checklist** (passed):
1. Transpose ±2 st while playing: the drums keep their pitch, the rest moves; drums
   and the rest stay in time with each other, also at 50–75 % tempo.
2. Toggle 🔒 on a pitched track while playing at +2 st: it changes pitch without a
   click.
3. Loops and seeking behave as before (no click at the loop jump).
4. Close and reopen: the locks are kept; an older song opens with drums locked.

## Stem offsets

Stems from different tools can start at different moments; each track can now be
moved in time.

- **UI:** a ↔ button per track (not a lock icon: 🔒 is pitch lock). Off by default
  and again on every load, so a stray drag never moves a track. On, the waveform
  gets an outline and a hand cursor, and dragging it moves the track (a click still
  seeks); off, a drag selects a loop as before. A non-zero offset shows next to the
  name (`+0.250 s`); double-click it to reset.
- **Engine:** `TrackParams::offset` (atomic, frames at the song rate; positive starts
  the track later, negative cuts its beginning). The mixer reads each track at
  `position - offset`. `Song::frames()` is now the end of the track that ends last
  (offsets included), computed from the atomics, so the song grows when a track is
  moved past the end; `track_frames()` is the length every track holds.
- **Waveforms** are drawn shifted by the offset. The view follows the new length,
  but only once the mouse button is released, so the scale does not change under
  the pointer mid-drag. The drag is measured from where it started, so rounding
  never adds up.
- **Session:** `offset` in seconds in `[[tracks]]` (written only when non-zero), so it
  survives a device with another sample rate. The device-switch restore and the
  session now share one `TrackState` path (`track_states` / `apply_track_states` in
  `app.rs`), which also carries the names and pitch locks.
- Tests: mixing with positive and negative offsets and the song length
  (`offsets_move_tracks_in_time`), the view following a longer or shorter song.
- Checked in the release build on Windows with a session giving bass +20 s and
  vocals −10 s: both drawn shifted, labels shown, length 4:03.8 → 4:23.8. Dragging
  not tried by Claude (it would have moved the mouse on the user's desktop).

**Test checklist** (passed):
1. Press ↔ on a track and drag its waveform: it moves smoothly, and plays shifted
   against the others.
2. Drag a track past the end: the song gets longer once the button is released, and
   plays to the new end.
3. With ↔ off, dragging still selects a loop; with it on, a click still seeks.
4. Double-click the offset label: the track goes back to 0.
5. Close and reopen: the offset is kept and ↔ is off again.

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
- Every stem is kept in memory as stereo at the device rate, now packed 24-bit (see
  "RAM"): 3:35 × 48 kHz × 2 ch × 3 bytes ≈ 62 MB per stem, ×7 ≈ 435 MB (f32 was 580).
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

### Slow UI when the window is maximized (fixed)

Fixed in Phase 3.5: it was Rectangle's move-then-resize meeting the wgpu renderer;
see "Window size and maximizing". The notes below are from before. Reported by the
user: maximizing the window is slow, playing or paused, and (2026-10-28) also with
nothing playing or separating. Only
**during** the maximize animation on macOS, and not at all on Windows, so it is
likely how macOS resizes the window surface frame by frame (live resize), not our
drawing. Low priority; if it is ever worth looking at, start there. Candidates: the cost of a frame grows with the width (each waveform
draws one line per column per channel: ~2,500 columns × 2 × 7 tracks ≈ 35,000 line
segments, retessellated every frame while playing); or the resize itself (wgpu
surface reconfiguration). Next step: measure the frame time at both sizes, then
consider drawing each waveform as one mesh or caching it while the view is unchanged.

### Track colors (fixed)

All unknown stems used to share one gray and repeated kinds one color. Fixed in
Phase 3.5: see "Stem names and colors". Still possible later: a color picker per
track, saved in the session.

### Other pending items

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
