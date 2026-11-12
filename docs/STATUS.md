# Status

Last updated: 2026-11-12.

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
  listened to them: they are fine (see "Click and count-in" below).
- **Phase 3.5, key detection: done.** Checked by the user against the songs they
  know (see "Key detection" below): meant as a rough hint, and that is enough.
- **Phase 3.5, dynamic tempo: closed** (see "Dynamic tempo" below). Change passes
  (checked by the user), "46 & 2" lines up with its time signature marks. The plan
  to close it (songs of different genres with tempo and time signature changes)
  passed with the user's marks: Brubeck's "Blue Rondo à la Turk" (jazz),
  Radiohead's "Paranoid Android" (prog rock, with tempo marks, Fixed and Fit),
  Queen and Opeth. The user: "a few things don't quite fit, but that is asking the
  algorithm for magic"; Opeth's ending is a bit odd. Accepted as is.
- **Phase 3.5, time signature changes: done.** Tested by the user: "excellent". On
  "46 & 2" they marked the 7/8 parts (10 changes) and, with Dynamic on, everything
  lined up (see "Time signature changes" below).
- **Phase 3.5, tempo changes: done.** Tested by the user on "Paranoid Android":
  works, with Fixed and Fit (see "Tempo changes" below). Found with the second
  song of the dynamic tempo plan. Fixed later: ½ and ×2 now scale the tempo marks
  too (on Opeth's "Deliverance" they did nothing after its first tempo mark).
- **Windows practice session: done.** The user tried Reaper on ASIO and Watershed
  on shared WASAPI, both on the Yamaha AG03: works.
- **Keyboard shortcuts window** (asked by the user): `?` (Shift+/ on a US keyboard)
  or the **?** button beside ⚙ lists every shortcut and mouse gesture, by section
  (`shortcuts.rs`, one table to keep up to date when a shortcut is added).
- **Phase 3.5, minimum window size and maximizing on macOS: done.** Tested by the
  user: both work (macOS now renders with OpenGL). Open question kept: wgpu vs
  OpenGL on macOS (see "Window size and maximizing" below).
- **Phase 4, native backend:** 4.0 to 4.4 done; 4.5 measured on both machines. **4.8 on
  Windows: burn is now faster than ggml there** (108.8 s against 122.2 s for a whole song,
  no autotune at start, ~1.6 GB of GPU memory instead of 3.4 GB), see "Own kernels on
  Vulkan (4.8)". **On macOS they help too:** 79 s for the 4:39 song (was 111.5 s), level
  with ggml's 77 s, see "On macOS (4.8)". The time-left estimate fix (4.8): tested by the
  user on Windows, works. **f16 weights converted on install: done** (see "Weights
  converted to f16"), tested by the user in the app on macOS: works. **4.7: the GGUF
  backend is removed** (see "GGUF backend removed (4.7)"). Both tested by the user on
  Windows: works.
- **Phase 4.5, more adjustments:** system default output, Reaper's Space / Enter and
  arrow keys on the grid, solo over mute, the start screen and the unsteady tempo icon:
  done and tested by the user on macOS (see "Output device and transport keys" and "Solo
  over mute, start screen"). Search in the recent songs: done, tested. Transposed vocals
  (formants): done, tested; it helps going down, less going up. The RAM peak when changing
  device: done, tested (see "Changing device without a RAM peak").
- **Logo: chosen** (distribution, step 1). Five rounded bars of a waveform, one per stem color
  (vocals, guitar, bass, drums, piano), on the app's dark tile. `assets/icon.svg` full-bleed
  for Windows and Linux, `assets/icon-macos.svg` on Apple's icon grid (824 px tile in 1024 with
  transparent margins); PNGs rendered with `resvg`. Set as the window icon (`main.rs`, `icon()`);
  the user saw it in the macOS Dock: looks good. Not done yet: `.icns`, `.ico` and the
  AppImage icon, with the release packaging.
- Reference separation works on CUDA (RTX 3050: a ~5 min song in 73–100 s) and on
  Apple Silicon through MPS (M5 Pro: a 4:39 song in 120 s, 2.6× faster than its CPU,
  same output as the CPU to within 1e-5).

## Next steps

**v0.2 (next version):** the user's feedback on v0.1.1 (2026-11-11) is planned in
`PLAN.md`, "v0.2: feedback on v0.1.1": cents and reference tuning, key with an online
prior, chords, the waveform shift bug, track selection and regrouped controls, undo / redo,
remembered window, click on another output, changelog. The user answered the open questions the same day.
The two bugs (waveform shift, tempo set at bar 1 with Dynamic on) are fixed and tried by
the user, and so is a follow-up fix (a fixed tempo steering the tracker past its stretch) (see "Waveform shift and tempo at bar 1").
Cents and reference tuning: done, tried by the user: works (see "Cents and
reference tuning"). Count-in on the bar line: done, to try (see "Count-in on the bar
line"). Remember the window: done, tried by the user on macOS (maximized included): works; still
to try on Windows (see "Remember the window").

Phases 0 to 3.5 and Phase 4.5 are done; the song info panel of 3.5 stays in the plan for
later. **Phase 4** (the native `burn` + `wgpu` backend): 4.0 to 4.5, 4.7 and 4.8 done; 4.6
(MelBand-RoFormer) is left, for the model browser.

0. **Goal: v0.1 with the native backend only,** no backend choice in Settings (decided
   2026-11-04, see "Decisions and open questions" below). Left:
   1. ~~Measure on macOS the build with the own kernels (4.8).~~ Done, see "On macOS (4.8)".
   2. **Machines without a dedicated GPU:** the app requires a GPU (decided, see below).
      Integrated GPUs with matrix instructions may still be fast enough; the test
      executable (`examples/gpu_test.rs`) on more machines would tell. ggml no longer
      needs to stay as a CPU fallback.
   3. ~~f16 weights converted on the user's machine.~~ Done, see "Weights converted to
      f16"; tested in the app on macOS and Windows.
   4. ~~Try the time-left fix in the app on Windows (4.8).~~ Tested by the user: works.
   5. ~~4.7, removing the GGUF backend.~~ Done, see "GGUF backend removed (4.7)"; tested by
      the user on Windows: works.
   6. **Later, not now:** for machines without a usable GPU, suggest separating with MVSEP,
      or integrate MVSEP's API into Watershed (see "Decisions and open questions").
1. **Model browser** (idea from the user, not designed): a window to search, download and
   pick among more models (see "Choosing models" below). Needs each architecture in
   `crates/roformer` first: MelBand-RoFormer is step 4.6.
2. **Windows:** check that nothing changed there (it keeps wgpu) and the minimum window
   size.
3. **Pending, wgpu vs OpenGL on macOS** (see "Window size and maximizing"): retry the
   Rectangle test **on battery**, with both renderers.
4. **Gentler separation (idea, not decided):** the user would like separation not to push
   the machine to the limit. The GPU runs at ~97 % for the whole separation on both
   machines. Options discussed: a pause after each chunk (a "speed" setting: e.g. full /
   balanced / quiet), a lower priority for the separation thread, smaller chunks (less GPU
   memory, same load). Windows showed no memory pressure, so this is about heat, fans and
   a busy GPU, not RAM.
5. **RAM, optional leftovers** (see "RAM" below): stream the separated stems to disk
   instead of holding all six.
6. ~~**macOS:** an `.app` wrapper.~~ `tools/package/macos.sh --no-dmg` builds
   `dist/Watershed.app` (see "Release workflow").
7. Ideas recorded in `PLAN.md`, not scheduled yet: the **song info panel** (it can reuse
   the tracked beats), disk optimization (design principles), portable builds (section 9),
   cutting pieces of a stem (in "46 & 2" the bass intro came out in the guitar stem). If
   dynamic tempo needs more later: Opeth's ending, and the Bar-mark idea in "Tempo
   changes". New feature ideas, with **auto update**, are in `PLAN.md`, "Feature ideas (not
   scheduled)": speed trainer, pitch in cents, MIDI pedal, chords on the grid, a gap
   between loop passes.
8. **v0.1:** features are enough for it; what is missing is item 0 and distribution
   (`PLAN.md` section 9). The logo and the release workflow are done (see "Release
   workflow"). Released `v0.1.0` on 2026-11-05. `v0.1.1` (2026-11-05): non-blocking file dialogs (resizing one crashed the macOS app), separate start folders for the song and stems dialogs, and auto update; released by Claude with `gh release edit --draft=false` after the workflow built it. The README has download instructions and two screenshots
   (`docs/screenshots`, taken by the user on macOS, halved to 1x).

## Auto update

Done 2026-11-05, ships in v0.1.1 (v0.1.0 users install v0.1.1 by hand once).
`crates/app/src/update.rs`, in Settings → Updates (version, "Check for updates", a
"Check for a new version at start" setting, on by default; when the start check finds a
release, an "Update to x.y.z" button shows in the top bar).

- **Check:** `GET api.github.com/repos/<repo>/releases/latest` (drafts and
  pre-releases are not listed, so a draft release is invisible until published).
  Versions compare as `x.y.z`.
- **Verified download:** GitHub lists a `sha256:` digest for every release asset; the
  download reuses `watershed_separation::download` (resumable, size and sha256 checked).
  An asset without a digest is not installed: the page link shows instead.
- **Install, per system:** macOS mounts the `.dmg` (`hdiutil`), copies the `.app` next
  to the installed one with `ditto` and swaps them (`.Watershed-old.app` is deleted;
  leftovers at the next start). Our own download carries no quarantine attribute, so
  Gatekeeper does not ask again. Windows unpacks the zip with the system's
  `tar.exe`, renames the running `watershed.exe` to `watershed.exe.old` (deleted at the
  next start) and copies the new one. Linux replaces the file in `$APPIMAGE`.
- **Restart:** "Restart now" closes the window; `on_exit` saves the session and then
  starts the new version (`open -n` on macOS). Disabled while a separation runs.
- **Not updatable:** builds inside a Cargo project (`target/`), macOS binaries outside
  an `.app`, Linux without `APPIMAGE`: they show the reason and the release page link.
- **Tested on macOS:** a 0.0.9 `.app` in a temp folder found v0.1.0 at start.
  Windows and Linux installers: compiled on every system, not run yet.

## Release workflow

Done 2026-11-05. `.github/workflows/release.yml`: pushing a `v*` tag builds the three
packages and creates a **draft** GitHub Release with them and `.github/release-notes.md`
(downloads, what each system needs, how to get past the unsigned-app warnings) plus
GitHub's generated notes; publishing it is by hand. The tag must match the version in
`Cargo.toml`, or the release job fails. Running it from the Actions tab only builds the
packages, as workflow artifacts: the first such run passed on all three, ~17 min.

| Package | Built on | Size | How |
|---|---|---|---|
| `Watershed-<v>-windows-x86_64.zip` | windows-latest | 21 MB | `.exe` with `+crt-static` (no Visual C++ runtime needed), README and licenses |
| `Watershed-<v>-macos-arm64.dmg` | macos-latest | 18 MB | `tools/package/macos.sh`: `.app`, ad-hoc signed, Applications link |
| `Watershed-<v>-linux-x86_64.AppImage` | ubuntu-22.04 | 21 MB | `tools/package/linux.sh`: AppDir by hand, appimagetool 1.9.1 and runtime 20251108 pinned by sha256 |

- **Icons:** `assets/make_icons.py` renders the PNGs, `icon.ico` (16–256 px) and
  `icon.icns` from the two SVGs with `resvg` and `iconutil`; the outputs are in git, so
  CI needs neither. The `.exe` gets its icon and version info from `crates/app/build.rs`
  (`winresource`, Windows only).
- **Decided here:** macOS arm64 only (no universal binary: Intel Macs would separate
  slowly anyway); Linux built on Ubuntu 22.04 for glibc 2.35; bundle id
  `app.watershed.Watershed` (only needs to be unique; no domain behind it). Nothing is
  signed: Windows SmartScreen and macOS Gatekeeper warn on first start (the release notes
  say how to continue). Signing and notarization stay for later (`PLAN.md` section 9).
- **Seen on macOS:** `macos.sh` built and opened the `.app` locally. The Windows zip's
  `.exe` from CI has the icon and version 0.1.0 in its resources; ProductName and
  FileDescription were "watershed" there; now "Watershed", checked in a second run.
- **`[skip ci]` skips tag pushes too:** GitHub skips every workflow of a push whose head
  commit says `[skip ci]`, so never tag such a commit (or the release does not build).
- **macOS package: tested by the user** (the .dmg from the Actions run): works.
- **Windows and Linux packages: not tried,** by the user's decision: they ship as built and
  users will report issues.

**Test checklist** (packages from the Actions run, or the draft release):
1. Windows: unzip, run `watershed.exe` on the desktop: SmartScreen's "Run anyway", the
   icon in Explorer and the taskbar, "Watershed" in Task Manager, separating a song.
2. macOS: open the `.dmg`, drag to Applications, open: "Open Anyway" in Privacy &
   Security, the icon in the Dock and Cmd+Tab, separating a song.
3. Linux (if a machine is around): `chmod +x`, run, play a folder of stems.

Windows machine setup, for reference: LLVM with `LIBCLANG_PATH` (Signalsmith's
bindings), CMake 4.4, the Vulkan SDK 1.4.363 and the Visual Studio 2026 Build
Tools. The first release build takes ~5 min (MSVC linker, Defender scanning
`target\`), later ones ~40 s; excluding the project from Defender or using a Dev
Drive should help.

## Decisions and open questions

Decided by the user on 2026-11-04:

1. **Download size: convert on the user's machine.** The app downloads the 699 MB `.ckpt`,
   converts the weights to f16 and deletes it: ~350 MB on disk, no loss beyond what f16
   inference already has, and no converted file to host (the checkpoint declares no
   license). The download stays 699 MB. To keep in mind when building it: the `.ckpt`'s
   sha256 is checked before converting; the converted file needs its own check (if the
   conversion is deterministic, the catalog can hold its expected sha256); an interrupted
   conversion must not leave a file that looks complete; `Weights` reads f32 only today.
2. **Backend: native only,** no choice in Settings.
3. **A GPU is required.** The user: an app that takes longer than the song itself to
   separate it makes no sense, so no CPU fallback (ggml's or burn's `flex`, ~42 min per
   song on the Ryzen 7 5700G). For machines without a usable GPU, the app can later
   suggest separating with MVSEP, or integrate MVSEP's API (already in `PLAN.md` Phase 5 as
   "Cloud backend"). Left for later.

MVSEP's API, found on mvsep.com on 2026-11-04, from its API page and credits page, nothing
tried:

- Free accounts can use the API with a token (register, log in), create a job and poll
  it (or a webhook). Non-premium users get 1 job at a time and a lower queue priority.
- Credits are charged per minute of audio; how many a free account gets per day, and
  which models are free, is not stated there. The terms for third-party apps were not
  read.
- Against it: the song is uploaded, it needs internet and an MVSEP account per user (the
  app cannot ship a shared token), and the stems come from MVSEP's models, not
  necessarily the same six.

**Choosing models** (asked 2026-11-04): a `.ckpt` is not a version of one model but a whole
model: each one is trained for its own stems and comes with its `.yaml`. The current one is
**BS-RoFormer SW** (6 stems: bass, drums, other, vocals, guitar, piano), the only public
6-stem one we know of. Others in `PLAN.md` section 6 are mostly vocals / instrumental
(viperx, Kim, unwa, becruily, gabox), many of them MelBand-RoFormer, which needs step 4.6
first. A model browser can only list models whose architecture `crates/roformer` runs.

## Native backend: burn + wgpu

Phase 4. `crates/roformer` (no C++): BS-RoFormer on burn 0.22.1, inference only, the
layers are plain tensors rather than burn `Module`s. It reads the `.ckpt` directly
(`pytorch-reader` parses the pickle without running it). `BsRoformer::forward` takes a
spectrogram and returns complex masks; `separate` goes from a song to its stems, with
the STFT and masking on the CPU. `Config` reads the parameters from the checkpoint's
`.yaml`, and refuses options the model here does not implement.

`tools/reference/dump_tensors.py` runs one real chunk through PyTorch (CPU, f32) and
saves the input and each layer's output (~2 GB, not in git);
`examples/chunk.rs` times the model and compares it layer by layer.

Measured on the M5 Pro, Metal, one chunk (13.35 s of audio, 1151 frames). Today's
ggml backend takes ~1.6 s per chunk here and PyTorch MPS ~2.7 s:

| Precision | Time per chunk | Masks vs PyTorch (relative RMS) | GPU memory reserved |
|---|---|---|---|
| f32 | 3.75 s | 2.8e-6 | 2362 MiB |
| f16 after the band split | **2.4 s** | 3.3e-3 | **1211 MiB** |

These are with the kernel trims made on Windows (below), which are shared code: before
them the same runs gave 4.1 s and 2.6 s, masks at 2.6e-6 and 3.1e-3. Rebuilding that
code today reproduced both errors to the digit, and the machine was on battery with
no low power mode, so the change comes from the trims and not from the setup.

The first pass of a run takes ~0.5 s more (compiling kernels). A 4:39 song would take
~106 s of model time in f16 (ggml: 72 s in all). Time per block in f32: 0.22 s for
the time transformer, 0.08 s for the frequency one, 0.09 s for the masks.

- **The model was right at the first run** in f32: every layer within 1e-5 of PyTorch.
- **f16 only after the band split.** With everything in f16, quiet signals square to
  zero inside the norms: silence gave NaN in every value and the error reached 9e-3.
  The band split stays in f32 and the norm's epsilon is 1e-6 in f16 (PyTorch's 1e-12
  is zero there). Silence and input at −60 and −100 dB now stay finite. Masks in f32
  gave no better error (3.06e-3 vs 3.09e-3), and they hold most of the weights, so
  they run in f16 too. PyTorch on CUDA also runs in f16 (`use_amp`).
- **Error of f16:** 3e-3 on the masks misses the 1e-3 set for layers, and peaks are
  much higher on isolated values. In the audio it gives 52–59 dB of SDR, see "Whole
  songs (4.3)" below; the listening test decides.
- **Attention memory:** on Metal burn has no streaming kernel (`cubecl-wgpu` only
  reports cooperative matrices on Vulkan), so it builds the whole score matrix:
  2.63 GB per time layer in f32, 5.9 GB reserved in all. `MAX_SCORES` in `layers.rs`
  splits long batches: same time, 1/5 of the memory. Its limit of 67 M scores only
  splits the time layers. Burn's own `attention` and `attention_fallback` took the
  same time.
- **Autotune on Metal: tried, not kept.** An earlier test here that "changed nothing"
  did not test it, since the workspace's `default-features = false` left the feature
  out of the build. With it really on, one change against the table above, two f16 runs
  and one f32 run:

  | Precision | Without | With `autotune` |
  |---|---|---|
  | f16, s per chunk | 2.38 | 2.32 |
  | f32, s per chunk | 3.75 | 3.51 |
  | First pass | 2.9 s | ~25 s, at every start |
  | f16 memory reserved | 1211 MiB | 1282 MiB |
  | Masks, f16 / f32 | 3.3e-3 / 2.8e-6 | 2.95e-3 / 1.9e-6 |

  A 5 min song (47 chunks) in f16 saves ~3 s and pays ~23 s of tuning, so the total
  gets worse; it would pay off only after ~40 min of audio in one run. Cubecl's
  `persistence` feature could remove the tuning, not tried: it would win 2.5 % here.
- **The trims are not exact in floating point.** They are exact as algebra, but they
  round differently: on Metal the masks went from 3.09e-3 to 3.32e-3 in f16 and from
  2.60e-6 to 2.78e-6 in f32. In f32 the layers match up to the last ones; in f16 the
  gap grows from the fifth block on. On Vulkan the same trims moved the error in both
  directions, which also points to rounding. None of the three was isolated. Harmless
  for the 1e-3 limit in f32; f16 is judged by the SDR (4.3).
- **Vulkan is better here:** ggml's speed on Metal comes from the GPU's matrix
  instructions, which wgpu exposes on Metal but cubecl does not use there. On Vulkan
  cubecl does use them when the driver has f16 cooperative matrices: confirmed on the
  RTX 3050, see below. The small gain above fits with that. **Corrected in 4.8:** cubecl
  does report matrix instructions on Metal (4 `cmma` configurations on the M5 Pro), and our
  fixed matmuls use them; what did not use them was burn's own matmul without autotune.
- `Device::metal` and `Device::wgpu` gave the same speed here.

Measured on Windows (RTX 3050, Vulkan, driver 610.62), same chunk. Today's ggml backend
takes ~2.9 s per chunk there and PyTorch CUDA ~2.1 s. **The 4.2 s gate passes**, three
runs in a row at 3.98–4.06 s:

| Build | f16, s per chunk | Masks vs PyTorch, f16 | GPU memory reserved |
|---|---|---|---|
| As first measured | 6.5 | 2.65e-3 | 1217 MiB |
| + `autotune` | 4.45 | 2.84e-3 | 1282 MiB |
| + rotary without split and concat | 4.27 | | 1282 MiB |
| + norm gain in the weights | 4.23 | 2.74e-3 | 1282 MiB |
| + GELU in three kernels | **4.0** | 2.78e-3 | **1282 MiB** |

f32 is now 8.0 s per chunk (was 12.8 s), masks at 1.7e-6 from PyTorch, 2536 MiB reserved.
Silence and quiet input stay finite in f16.

- **Which GPU ran:** the RTX 3050, so the first gate miss was real. `nvidia-smi` during
  the example: 94 % use on average, memory from 0.9 to 2.3 GB, 65 W. `DefaultDevice`
  does pick it.
- **Which kernel ran:** `autotune` and `fusion` were both off, because the workspace
  sets `default-features = false` on burn. No flash attention kernel existed in the
  build: every attention went through the materialized fallback. With `autotune`
  (Vulkan only, in `crates/roformer/Cargo.toml`) the driver's f16 cooperative matrices
  are used and the attention runs `blackbox_accelerated`, the streaming kernel.
- **Where the time goes,** per pass, before the kernel trims: time transformer 2.97 s,
  frequency 1.37 s, masks 0.11 s. Inside the two transformers: attention 2.16 s, feed
  forward 0.88 s, rotary 0.62 s, q/k/v 0.34 s, the rest under 0.3 s. Numbers from timers
  placed in the layers, which wait for the GPU, so they sum a little over a pass.
- **Kernel trims,** all exact rewrites: rotary is `(a, b) cos + (b, a) sin` with a flip,
  the norm's scale is folded into its gain at load, and GELU's two divisions are folded
  into the linear weights around it.
- **Tried, no gain, not kept:** `MAX_SCORES` from 16 M to unlimited does not change the
  time and unlimited reserves 3781 MiB, so it stays. `fusion` gave 4.39 s against 4.45 s
  but doubled the first pass to 76 s. `CUBECL_AUTOTUNE_LEVEL=extensive` gave 4.40 s.
  DX12 not tried: 4.5 compares it.
- **Cost of autotune, still open:** the first pass of every process takes ~33 s instead
  of ~4 s, because it benchmarks kernels again each time (cubecl's `persistence` feature
  may fix it, not tried). A 5 min song (47 chunks) would take ~3.6 min of model time with
  that included, against ~2.3 min with ggml, so burn passes the gate but is not faster
  than today's backend here.
- **Measuring:** `CUBECL_DEBUG_LOG=<file>` logs the autotune decisions but costs up to
  3 s per pass, mostly shown as masks time, so never time with it on. `chunk` now prints
  the time per block. Its numbers with `fusion` on are not reliable: syncing between
  blocks breaks up the fused kernels.

### Equivalence tests (4.2)

`crates/roformer/tests/equivalence.rs` fails if any layer is over 1e-3 from PyTorch in
f32:

- **Tiny model, in CI:** `tools/reference/make_tiny_fixture.py` builds the real
  PyTorch class with the real options, but small (dim 16, 2 blocks, 5 bands, 2 stems)
  and with seeded random weights, and runs it on seeded audio. The two files in
  `crates/roformer/tests/fixtures` take ~400 KB; the `.ckpt` goes through the same
  reader as the real one. It runs on burn's `flex`, the pure Rust CPU backend, in a
  dev-dependency, and takes 0.01 s. Worst layer 3.1e-7.
- **Real model, by hand:** the `--ignored` test on the GPU and the 2 GB dump. On Metal
  every layer passes: worst 1.41e-5 at `freq_9`, masks 2.78e-6.
- **The tests were checked by breaking the model:** a wrong GELU and a flipped rotation
  each put 6 layers at 2–5e-2. A second test checks that quiet input stays finite in
  f16 on `flex`, which does compute in f16 and fails like Metal: silence gives NaN with
  the band split in f16 and PyTorch's epsilon. Either guard alone already prevents it
  on the tiny model, so the test does not cover each guard by itself.

### Whole songs (4.3)

`Stft` and `overlap_add` in `crates/dsp`, `BsRoformer::separate` on top. `examples/song.rs`
separates a song and gives the SDR of each stem against a folder of `separate.py` stems.
Both sides must read the same audio: here, a 24-bit FLAC made from the mp3 with
`separate.py`'s own decoder, so that two mp3 decoders are not part of the comparison.

The 4:39 test song on the M5 Pro, Metal, against PyTorch on MPS:

| Stem | RMS | SDR, f32 | SDR, f16 |
|---|---|---|---|
| bass | −22.6 dB | 99.1 dB | 54.3 dB |
| drums | −20.3 dB | 105.1 dB | 58.5 dB |
| other | −97.6 dB | 50.6 dB | 31.0 dB |
| vocals | −19.2 dB | 108.1 dB | 59.0 dB |
| guitar | −23.2 dB | 96.8 dB | 52.2 dB |
| piano | −24.2 dB | 100.0 dB | 52.5 dB |
| **Time** | | 177 s | **116 s** |

- **f32 passes the 60 dB gate by far** on the five stems that sound. `other` is silent
  in this song, and its 50.6 dB fits the 24-bit reference: its rounding noise, ~−149 dB,
  sits ~51 dB under a stem at −97.6 dB. Not measured against a 32-bit float reference.
- **f16 is above the q8 GGUF**, so its limit is 45 dB on audible stems. Measured the
  same way on the same song, the GGUF gets 36.4–43.1 dB on the five stems that sound.
  **The user listened to f16 against PyTorch's stems: "almost the same".**
- **Time:** ggml takes 72 s on this song. 44 chunks at ~2.4 s are ~106 s, so about 10 s
  (~0.2 s per chunk) go to the STFT and iSTFT, one after the other on one thread. Those
  could run in parallel, for 4.5.
- **Tests,** all on the tiny model in CI, on `flex`: the STFT, a chunk from audio to
  audio and five whole runs of MSST's own `demix` (`make_tiny_fixture.py` explains the
  cases), at most 1e-4 of relative error. Checked by breaking the code in eight ways:
  no zero DC, conjugated masks, Nyquist imaginary part kept, a symmetric Hann window, no
  song padding, zeros instead of reflection on the last chunk, and fades on the first and
  last chunk. Each one fails a test; the last fade needed the fifth case.
- **Same quirks as `demix`, on purpose:** when a song is at most twice the border long
  (13.35 s at overlap 2), there is no padding at the edges; a frame no chunk reached with
  any gain becomes 0.

### In the app (4.4)

`BurnSeparator` in `crates/separation`, always in f16 on the default GPU. The catalog's
second model has the `.ckpt` as its file and a `[model.native]` table with the
architecture and the `.yaml`, downloaded with it. The choice is in Settings ("Separation
model", `Config::separation_model`); the default stays the GGUF. Musicians see **Compact**
(GGUF) and **Full** (native): what differs for them is download size and speed. "Native"
stays a word for the code. **Those two names are provisional:** the user decides naming at
the end of Phase 4, since the C++ backend may be gone by then and the choice with it.

From the example (`separate --native --flac`), same 24-bit input, on the M5 Pro:

| | Native (f16) | GGUF |
|---|---|---|
| Separating | 111 s | 82 s |
| Process footprint, peak | 2489 MiB | not measured here |
| SDR against PyTorch, the five stems that sound | 52.2–59.0 dB | 36.4–43.1 dB |

- **The example's stems match `examples/song.rs`** to the hundredth of a dB (`other`
  aside: 30.9 against 31.0), so catalog, download and sha256, `.yaml` and writing add no
  error.
- **The GGUF took 82 s here, not the usual 72 s.** It ran right after the native run;
  heat is a guess, not checked. For 4.5.
- **Freeing the model:** dropping `BurnSeparator` runs `memory_cleanup`. With
  `MallocLargeCache=0`, a 60 s clip ends at 136 MiB of footprint 2 s after that. The
  full song's 1212 MiB, sampled 50 ms after, was too early and had malloc's cache on, so
  the example now waits 2 s.
- **GPU errors are panics in `burn`.** `BurnSeparator` catches them and returns an error,
  or the app would wait forever for a dead thread. Only tested with a fake panic: a
  machine with no GPU has not been tried. There is no CPU fallback yet (4.5).
- **Cancel** is polled before each chunk, so it takes up to ~2.4 s here, ~4 s on the RTX
  3050 and, in the first chunk there, ~33 s of autotune.
- **Same song, other model:** dropping a song separated before opens its folder, whatever
  the model. To compare, move that folder out of the library first.

**RAM in the app, seen by the user on macOS, with the native model** (to look into in 4.5):

| Moment | RAM |
|---|---|
| App started | ~200 MB |
| Separating "The Motherload" | 2.6 GB |
| Right after it finished, stems open | ~1 GB, and it stays there |
| The same stems, in a fresh start of the app | ~800 MB |
| Another song cancelled, from a clean start | ~400 MB (was 200–300 MB) |

So something like 100–200 MB stayed after any run, finished or cancelled: see below.

### RAM left after a run (4.5)

`examples/separate.rs --repeat N` separates a song N times in one process, each on a new
thread that loads and frees the model, as the app does, and after each run prints the
memory left and burn's pools, and saves `vmmap --summary`. The 4:39 test song on the M5
Pro, `MallocLargeCache=0`, footprint after each run, starting from 4 MiB:

| Build | Run 1 | Run 2 | Run 3 | Run 4 | Peak |
|---|---|---|---|---|---|
| One thread for all runs | 196 | 190 | 190 | 192 | 2407 |
| One new thread per run, as the app | 196 | 299 | 350 | 463 | 2728 |
| Fixed | 107 | 77 | 77 | | 2224 |

- **It was a leak of 108 MiB per separation.** Each thread gets its own cubecl stream, and
  each stream a staging buffer, CPU-visible memory that GPU reads go through. It grows to
  the largest read, which was the masks of a chunk: 1025 × 2 × 1151 × 2 × 6 × 4 bytes,
  exactly the 113,258,496 bytes burn reported. `memory_cleanup` does not free staging
  memory, and the stream outlives its thread. A first test on one thread looked flat,
  because that thread kept reusing its buffer.
- **Fixes:** `default_device` sets `StreamPolicy::Single`, so all threads share one stream
  (needs `cubecl-environment` as a direct dependency), and `separate_chunk` reads the masks
  stem by stem, so the buffer is 18 MiB and the masks in host memory are 1/6 as well. Stems
  bit-identical on a 30 s clip, same time (109–113 s per song).
- **What the 77 MiB are:** ~32 MiB of malloc in use, ~25 MiB of Metal driver objects, among
  them the 18 MiB staging buffer, and ~28 MiB of "owned unmapped" graphics memory. The
  Metal objects (compiled kernels and the like) live as long as the process.
- **`sync` before `memory_cleanup` changes nothing,** so timing was not the cause.
- **In the app:** tested by the user, two songs split in a row with the window open: "works
  quite well", the leak is gone. The app's numbers were not noted down.
- **Windows (RTX 3050, Vulkan, driver 610.62): the fix holds.** The 4:03 test song, `--native
  --repeat 3 --memory`, one process, with the usual desktop apps open:

  | | Run 1 | Run 2 | Run 3 |
  |---|---|---|---|
  | Total | 242.1 s | 168.1 s | 166.7 s |
  | Separating | 238.7 s | 164.5 s | 164.0 s |
  | burn's pools, reserved | 18.0 MiB | 18.0 MiB | 18.0 MiB |
  | Resident after the run | 333 MiB | 352 MiB | 357 MiB |

  Pools identical to the byte (18,876,416) after every run, stems the same size in all three.
  Resident memory rose by 19 and then 5 MiB, not the 108 MiB per run of the leak; three runs
  cannot tell whether it settles. Resident peak 1296 MiB. GPU: 3461 MiB dedicated, 79 MiB
  shared at the peak, so nothing spilled, but the card was at 5118 of 6144 MiB, 1674 of them
  before starting.
- **Run 1 here is not the usual first run:** it was the first ever with that binary, and
  separate processes afterwards take ~198 s (see "Time per song (4.5)"). The extra ~44 s were
  not explained; compiled shaders landing in the driver's cache is a guess.

### Time per song (4.5)

The 4:39 test song on the M5 Pro, plugged in, alternating the two backends in one sitting:

| Order | GGUF | Native (f16) |
|---|---|---|
| First pair | 72.5 s | 107.2 s |
| Second pair | 78.1 s | 111.5 s |

- **The GGUF's 72 vs 82 s is not a regression:** the same binary gave 72.5 s first and
  78.1 s after a native run, so the later runs slow down. `pmset -g therm` recorded nothing,
  so heat stays a guess, but comparing a backend across a sitting has about 5 s of noise.
- **STFT and iSTFT in parallel: 2 %.** `separate_chunk` now does them on scoped threads.
  Same binary either way apart from that, 30 s clip, three rounds of four runs alternating:
  17.25 s sequential, 16.9 s parallel, every time. Stems bit-identical. That is ~0.05 s per
  chunk, 2–3 s per song, not the ~10 s estimated in 4.3: it subtracted two separate
  measurements, which does not hold at this precision.
- **So the gap to ggml, ~35 s per song, is all GPU time in the model.** "Vulkan is better
  here" above is why: cubecl does not use the matrix instructions on Metal.

**Windows** (RTX 3050 6 GB, Vulkan, driver 610.62): the 4:03 test song, one process per run,
alternating, sampled as in "RAM", with the usual desktop apps open:

| | GGUF, 1st | Native, 1st | GGUF, 2nd | Native, 2nd |
|---|---|---|---|---|
| Total | 106.1 s | 198.1 s | 106.8 s | 197.1 s |
| Separating | 103.4 s | 195.4 s | 104.1 s | 194.1 s |
| Writing | 2.4 s | 2.5 s | 2.5 s | 2.8 s |
| Resident peak | 1364 MiB | 1226 MiB | 1359 MiB | 1237 MiB |
| GPU dedicated, peak | 1207 MiB | 3462 MiB | 1207 MiB | 3462 MiB |
| GPU shared, peak | 129 MiB | 75 MiB | 129 MiB | 75 MiB |
| Card in use, peak (of 6144 MiB) | 2759 MiB | 4985 MiB | 2817 MiB | 5030 MiB |

- **Native is 1.86× the GGUF** per run, against ~1.45× on the M5 Pro. Repeatable: 1 s between
  pairs, and no slowdown across the sitting as on macOS.
- **It fits, with nothing spilled,** but 2.9× the GGUF's GPU memory, and ~1.1 GiB free with
  ~1.5 GiB already taken by the desktop apps. A 4 GB card would not have held it this way.
- **Autotune at every start: ~30 s.** A new process takes ~198 s, a later run in the same
  process ~168 s, so the ~33 s stands and the app pays it only on its first separation.
  That the 30 s are all autotune is inferred: the same difference would show if it were
  kernels compiling once per process. Not isolated.
- **The first run of a new binary costs ~40 s more,** seen twice: 242.1 s and 239.2 s, then
  ~197 s. Only once per build, so users would pay it after an update. The cause is a guess:
  kernels landing in the driver's shader cache.
- **cubecl's `persistence`: not kept.** Same binary otherwise, new process each run: 239.2 s
  (the new binary's first run), then 197.8 s and 196.5 s, so no gain against ~198 s. It also
  brings in a database (turso) and took the build from 6 to 10.5 min. The test does not
  settle it: `target/environment/default.db` was created and stayed at 0 bytes, so no tuning
  result reached the disk, and why was not found. The default location would also not suit
  a release build. Left for 4.8.
- **Vulkan vs DX12: Vulkan stays.** Only `default_device` changed, pinned to DX12 with
  `Device::wgpu_options()` so it could not fall back to another API (the example still prints
  "Vulkan": `burn.rs` hardcodes that label). Two runs, 729.4 s and 717.6 s, so 3.6× Vulkan and
  not a first-run cost. GPU memory 3194 MiB dedicated, **1104 MiB shared** against 75 MiB,
  resident peak ~2030 MiB against ~1230 MiB, GPU busy (96 % on the 3D engine). Why it is so
  slow was not looked into; `dxcompiler.dll` was on the PATH through the Vulkan SDK, which
  users would not have, and which compiler it really used was not checked.

### CPU fallback (4.5)

burn's `flex` (pure Rust, already a dev-dependency for the tests), `examples/chunk.rs`, one
chunk on the M5 Pro (15 threads). This was a temporary build: nothing here is in the code.

| Build | f16, s per chunk | f32, s per chunk | Memory, f16 / f32 |
|---|---|---|---|
| As in the workspace | 76.5 | 90.5 | 1.8 / 2.6 GiB |
| + `burn-flex` default features | 26.7 | 25.5 | 1.8 / 2.7 GiB |

- **Same trap as `autotune`:** the workspace's `default-features = false` on burn leaves
  `simd` and `rayon` out of `burn-flex`, so it ran on one core. A direct `burn-flex`
  dependency with its defaults turns them on. `ps` showed 3.2–5.5 cores in use, not 15.
- **f32 is no slower than f16 on the CPU,** so a fallback should use f32: same time, and
  97–108 dB of SDR instead of 52–59.
- **Time:** 26 s × 44 chunks is ~19 min for the 4:39 song, against ~7.5 min estimated for
  ggml's CPU backend (patched to 15 threads) and ~5 min for PyTorch's. Only the model; no
  whole song was run on the CPU.
- **Not tried:** `Device::cpu` (cubecl's CPU backend, a much heavier build), `apple-amx`.

**Windows, Ryzen 7 5700G (16 threads):** same temporary build, `examples/chunk.rs`, 4 passes
each, with the usual desktop apps open:

| Build | s per chunk | Cores in use, average | Worst layer error |
|---|---|---|---|
| f16 | 146.3 | 2.7 | 8.6e-3 |
| f32 | 65.7 | 5.2 | 5.3e-6 |

- **On x86 f32 is 2.2× faster than f16,** not level as on the M5 Pro: another reason for a
  fallback to use f32. Why was not looked into; this CPU not having f16 in hardware is a guess.
- **Time:** 66 s × ~38 chunks is ~42 min for the 4:03 song, about 2.5× the M5 Pro per chunk.
  Only the model, no whole song was run on the CPU.
- **Same thread limit as on macOS:** ~5 of 16.
- **Memory not measured:** the sampling failed (it read 153 and 0 MiB).

### Own kernels on Vulkan (4.8)

Windows, RTX 3050, driver 610.62, `examples/chunk.rs`, one chunk in f16:

| Build | s per chunk | First pass | Masks vs PyTorch |
|---|---|---|---|
| Before (autotune, burn's flash attention) | 4.06 | 75 s (new binary), ~30 s after | 2.85e-3 |
| + matmul, own softmax, burn's ops around it | 2.90 | 29 s | 2.85e-3 |
| + linear layers as one 2D matmul | 2.82 | | |
| + fixed matmul kernels, `autotune` off | 2.73 | 5.0 s | 2.94e-3 |
| + own kernels to split and merge the heads | **2.27** | **4.0 s** | 2.90e-3 |

f32: 6.1 s per chunk, masks 1.75e-6, no layer above 1e-4; the real-model equivalence test
passes. GPU memory reserved: 1422 MiB in f16.

- **Where the time went:** a time layer took ~210 ms, ~170 ms of them attention. burn's flash
  kernel ran at ~1 TFLOPS. As matmul + softmax + matmul, the softmax took ~10 ms per slice of
  scores against ~5 ms for both matmuls: burn's is five kernels over 127 MB each. Our softmax
  (`kernels.rs`, one plane per row, sums in f32) takes ~1.2 ms.
- **Then copies:** cubecl's profiler (`CUBECL_DEBUG_OPTION=profile` plus `CUBECL_DEBUG_LOG`)
  showed ~150 ms per pass in rotary's `flip` (~3 ms per call) and ~200 ms in copies from the
  head reshapes. `split_heads` goes from `to_qkv`'s output to q, k and v in one pass, with the
  rotation and the scale; `merge_heads` writes each slice's output times its sigmoid gate
  straight into place, so there is no `cat` either.
- **No autotune:** it made 29 decisions per process, each compiling and timing ~25 kernels. The
  winners were all tensor-core (`cmma`) matmuls; `Kernel` in `kernels.rs` picks the same three
  by shape and calls cubek directly. Where a strategy cannot run (no matrix instructions, or
  another backend) burn's matmul runs instead, which without the feature is untuned.
- **Linear layers:** a 3D input runs as a batched matmul with the weight broadcast, 16.5 ms
  against 10 ms as one 2D matmul at the frequency layers' shape (1151 batches of 62).
- **Fallbacks:** each kernel returns `None` off cubecl or without plane operations of one fixed
  size, and the layer takes burn's ops, as the CI tests on `flex` do.

A whole song (Caifanes, "Nubes"), `examples/separate`, a new process each, native first:

| | Native (f16) | GGUF |
|---|---|---|
| Separating | **108.8 s** | 122.2 s |
| GPU memory over idle, peak | ~1.6 GB | ~1.4 GB |
| Resident peak | 1.3 GB | 1.5 GB |

**GPUs without matrix instructions** (seen on a Radeon Vega iGPU, Ryzen 7 5825U, with the
test executable below): the first separation panicked in burn's untuned matmul, which asked
for 35 KB of shared memory where the GPU has 32 KB. Fixed: `matmul` falls back to
`double_unit` (small tiles, then large), `simple_unit`, then `naive`. Forced on the RTX 3050,
that path takes 6.0 s per chunk (31.8 s with `simple_unit` first), masks 3.06e-3 in f16 and
1.75e-6 in f32. **On the Vega it works:** 3 min of test signal in 584 s (0.31× real time),
19.6 s per chunk after a 35.6 s first one, steady from start to end (no throttling seen),
so ~12–13 min for a 4 min song. Plane size 64, so all 696 attentions took our kernels; no
`cmma`, so all 35206 matmuls took the fallbacks. Process peak 2742 MiB (1036 MiB on the RTX
3050): an integrated GPU's memory is system RAM, which likely counts here, not checked.
Not compared with ggml on that machine. Ideas if integrated GPUs need more speed: tune the
fallback for them (tile sizes, f32 against f16 on GPUs whose f16 is not faster), or its CPU.

**Test executable:** `examples/gpu_test.rs` in `crates/separation`, built with `+crt-static`
(needs only system DLLs and the driver's Vulkan loader). Double-clicked, it downloads the
model, separates 3 min of a test signal and writes `watershed-gpu-test.txt` next to itself.
On the RTX 3050: 80.2 s, 2.39 s per chunk after a 13 s first one, 1036 MiB peak.

### On macOS (4.8)

M5 Pro, Metal, **on battery** (no low power mode), 2026-11-04. "Before" is `3aa4663`, the
commit before the own kernels, built in a separate worktree; the two alternated in one
sitting.

`examples/chunk.rs`, one chunk, two runs each (they agreed within 0.05 s):

| | Before | After |
|---|---|---|
| f16, s per chunk | 2.45 | **1.68** |
| f16, first pass | 3.2–3.7 s | 2.9–3.4 s |
| f16, masks vs PyTorch | 3.32e-3 | 2.74e-3 |
| f16, GPU memory reserved | 1211 MiB | 1422 MiB |
| f32, s per chunk | 3.83 | **3.13** |
| f32, masks / worst layer | 2.78e-6 / 1.41e-5 | 1.81e-6 / 7.20e-6 |
| f32, GPU memory reserved | 2362 MiB | 2782 MiB |

Per block in f16: time transformer 1.71 → 1.12 s, frequency 0.61 → 0.49 s. Quiet input
stays finite.

A whole song (Caifanes, 4:39), `examples/separate --native --flac`, a new process each:

| | Before | After | GGUF |
|---|---|---|---|
| Separating | 111.5 s | **79.0 s** | 76.7 s |
| Resident peak | ~1.22 GB | ~1.24 GB | 2.3 GB |
| Footprint peak | ~2.46 GB | ~2.69 GB | 2.3 GB |

- **Level with ggml here,** 2–3 s apart, within the ~5 s of noise seen across a sitting
  (see "Time per song (4.5)"); the GGUF ran last. Before, native was 1.45× the GGUF.
- **Which kernels ran** (the test executable's counters): plane size 32, 4 `cmma`
  configurations; 696 attentions and all 35206 matmuls on our kernels with matrix
  instructions, none on the fallbacks. So "cubecl does not use Metal's matrix
  instructions" was about burn's matmul, not about cubecl.
- **Test executable on Metal:** 51.0 s for the 3 min test signal, 1.72 s per chunk after a
  2.98 s first one (no autotune now), 3.53× real time, 940 MiB peak.
- **Memory:** ~210 MiB more GPU memory reserved per chunk (the 2D matmuls and head
  buffers), ~230 MiB more footprint at the peak for a whole song. Not looked into.
- **Battery made little difference:** the same "before" build gave 107–111 s plugged in
  (4.5), and 111.5 s here.

### Smaller download (4.5)

The `.ckpt` is 699 MB, the q8 GGUF 188 MB. From reading burn's sources, nothing measured:

- burn has quantized tensors (`QuantValue::Q8F` and others), but that is a way of computing.
  The size to win is on disk, and the model casts everything to f16 at load anyway.
- **Weights stored as f16: ~350 MB,** with no loss beyond what f16 inference already has,
  and no burn quantization involved. Done: see "Weights converted to f16".
- **Weights stored as q8: ~190 MB,** but with the GGUF's quality (36–43 dB, not 52–59).

**Test checklist:** 1–4 passed on macOS.
1. Settings: choose Full; drop a song you have not separated: it downloads 700 MB with
   progress and separates, with progress and time left.
2. The stems open and sound right.
3. Cancel while separating: stops within a few seconds and leaves no folder.
4. Close and reopen: the choice is kept.
5. Windows: the same, on Vulkan. Tested by the user: "works well" overall. The model was
   already downloaded, so the download in item 1 was not seen again.

**Time left, seen on Windows:** it sometimes takes a while to appear, starts near 40 min and
settles at 3–4 min. From reading `remaining()` in `separate.rs`, not measured: it is elapsed ×
(1 − f) / f and needs f ≥ 2 %, so nothing shows until the first chunk ends, which is 1/38
of the song. That chunk carries the ~30 s of autotune, so ~65 s for it gives ~40 min. "Sometimes"
fits: only the first separation after starting the app pays it, more on a new build.
Leaving the first chunk out of the estimate should fix it, on GGUF as well.

**Fixed (4.8), tested by the user on Windows: works.** The estimate now uses the rate since the first
progress above zero in a step, so the first chunk is left out. It shows one chunk later than
before (on the RTX 3050, ~70 s in instead of ~65 s) and should start near the real time. A
resumed download also stops counting the bytes already on disk. Unit tests in `separate.rs`
with the Windows numbers. Both backends report once per chunk and start the step at 0.

### GGUF backend removed (4.7)

Done 2026-11-04. Gone: `GgufSeparator`, `bsroformer.rs` (the safe wrapper), the C++ wrapper
and CMake project in `crates/separation/cpp`, `build.rs`, the two submodules (BSRoformer.cpp,
ggml) and `.gitmodules`, the GGUF catalog entry, the "Separation model" choice in Settings
and `Config::separation_model` (an old `config.toml` with that key still loads: unknown keys
are ignored). In CI: the submodule checkout, the Vulkan SDK on Windows, and `libvulkan-dev
glslc spirv-headers` on Linux. The workspace no longer needs CMake.

- **Catalog:** one model, with `architecture`, `config` and `converted` at the top level
  (the `[model.native]` table is gone, since every model is now a checkpoint).
  `ModelEntry::load` returns a `BurnSeparator`.
- **`examples/separate.rs`** lost `--native`, `--cpu` and `--model`.
- **`SeparationError::Native`** became `Model(String)`.
- **The old GGUF file stays in the models folder** of machines that had it (188 MB):
  nothing deletes it. Only the two development machines have it; delete it by hand.
- Before this commit, the GGUF backend can be built from git history for comparisons.
- **Windows:** after pulling this commit, the app builds and separates. Tested by the user:
  works.

### Weights converted to f16

Decided by the user (see "Decisions and open questions"). The app still downloads the 699 MB
`.ckpt`, then converts it to a 352 MB f16 safetensors file and deletes it.

- **`convert_to_half`** (`crates/roformer/src/weights.rs`) writes every tensor in f16 except
  those the model reads as f32 even in f16 mode: the band split and the rotary frequencies
  (`stays_f32` in `model.rs`, next to `load`). `Weights::open` reads a `.safetensors` file
  (header only; tensors on demand, as with the checkpoint) as well as a `.ckpt`.
- **Same model, exactly:** on the tiny model in CI the masks in f16 are bit-identical from
  either file (`converted_weights_give_the_same_masks_in_f16`, checked by breaking the rule:
  with the band split in f16 it fails). On the real chunk on Metal, every layer's error
  against PyTorch is the same to the printed digit, and the speed is the same. In f32 a
  converted file gives f16-rounded weights: the equivalence tests keep using the `.ckpt`.
- **Deterministic:** two conversions give the same bytes (also a test), so `models.toml`
  holds the converted file's size and sha256 (`[model.native.converted]`);
  `examples/convert.rs` prints them. 0.4 s and 388 MB peak on the M5 Pro, so no progress
  or cancel for that step.
- **Install** (`ModelEntry::install`): a `.ckpt` already in the models folder (from an older
  version or an interrupted install) is hashed and used if intact, else downloaded. The
  conversion goes to `<file>.part`, renamed only once size and sha256 match, so an
  interrupted or wrong conversion never looks installed; the `.ckpt` is deleted after
  (and if one is found next to an installed model). Tests on the tiny model cover both.
- **Seen on the M5 Pro:** the cached `.ckpt` was checked, converted and deleted in ~1.4 s
  in all, then the 4:39 song separated in 76.8 s.
- **Not done:** the app shows "Downloading the model" during those ~1.4 s; a model already
  downloaded shows "0 / 699 MB" meanwhile. Fine at that length; worth a "Preparing the
  model…" label if it is slower on Windows.

**Test checklist:** 1 and 2 passed (2 tested by the user on Windows after 4.7, so without
the Settings choice).
1. macOS (already converted here): Settings → Full, drop a song not separated yet: it
   separates and sounds as before.
2. Windows (the `.ckpt` is still in the cache there): the same; the first separation
   converts it. Afterwards the models folder (`%LOCALAPPDATA%\Watershed\cache\models`)
   has the `.safetensors` and no `.ckpt`.
3. A clean download, on either machine: delete the `.safetensors` from the models folder,
   separate with Full: it downloads 699 MB with progress, then separates.

## Changing device without a RAM peak

Phase 4.5. Seen by the user on macOS: ~500 MB more for up to a second when changing output
device. Built on macOS and tested by the user there: works.

- **Cause:** only a device with another sample rate (here the MacBook's speakers run at
  44.1 kHz, the headset and the monitor at 48 kHz). `switch_device` read the song again from
  disk: every stem decoded and resampled in parallel, each unpacked to f32 twice (decoded and
  resampled), the tempo, the key and the peaks computed again, while the old song stayed in
  memory until the new one replaced it. A device with the same rate reloads nothing.
- **Now:** the stems already in memory are resampled (`loader::resample`), one at a time.
  `switch_device` takes them out of the song (the old engine is gone, so nothing else holds
  it; if something did, it would copy them and log a warning), and each one is unpacked and
  freed before resampling, so the memory grows by about one stem (two f32 copies of it,
  ~200 MB for a 4:39 stem), not by a second song. The tempo, the key and the onsets are in
  seconds and are kept; only the peaks are computed again. The waveforms and the mixer stay
  on screen meanwhile, with "Resampling <song> to <rate> Hz… n/6".
- **Time:** six 4:39 stems from 44.1 to 48 kHz take ~1.0 s on the M5 Pro (offline, without
  packing), about what the reload took.
- **Quality:** going back and forth resamples the resampled stems again instead of the files.
  rubato's FFT resampler and the 24-bit packing are both far below hearing, so it should
  not be audible; reopening the song reads the files again.
- **Edge cases:** switching again during the resampling waits for it first (~1 s). Opening
  another song meanwhile ignores the result. If resampling failed, the folder is read again
  from disk. The session is not written during those ~1 s (as before).

**Test checklist** (passed on macOS):
1. With a song playing on the headset (48 kHz), switch to the MacBook's speakers (44.1 kHz):
   the RAM in the top bar should not jump by ~500 MB; playback resumes where it was, with the
   same mixer, offsets, loops and markers.
2. Back to the headset: same. The song sounds the same as before the two switches.
3. Switch twice quickly: it ends on the second device, playing.
4. A device with the same rate: switches at once, as before.

## Solo over mute, start screen

Asked by the user, built and tested by the user on macOS: works.

- **Solo overrides mute:** with any track soloed, exactly the soloed tracks play, muted or
  not; with none soloed, the unmuted ones. `TrackParams::audible` holds the rule for the
  mixer and for the dimmed waveforms. Before, mute won over solo. What Reaper does was not
  confirmed: the search found no Reaper source, only Ardour's manual, where a muted track
  stays muted when soloed unless "Soloing overrides muting" is turned on.
- **Start screen:** the "Separated songs go to" row was left-aligned (an egui horizontal row
  does not center itself): now centered by measuring it. The recent songs were a `Grid` whose
  stripes ran past the last column and whose scroll area showed only three rows: now plain
  rows, up to 720 points wide and centered, the title taking what the folder (180 points,
  dropped when narrow) and time (100 points) columns leave, cut short with "…" if needed. The
  window's minimum width (~1416 points on the MacBook) keeps it from getting narrower there.

- **Unsteady tempo icon:** after the BPM, the ★ became a lightning bolt ⚡, chosen by
  the user after trying both it and a wavy line 〰 in the app. Only glyphs in egui's fonts can be
  used (★, ⚡ and 〰 are there, ∿ is not). Drawn 1.4× the text size, in a hover area as tall
  as the bar: it was a `Label`, which showed a text cursor and had a hover area as small as
  the glyph, so the tooltip often missed. Tested by the user: works.

**Test checklist** (passed on macOS):
1. Mute a track, then solo it: it plays. Unsolo it: silent again. Solo another track: the
   muted one stays silent.
2. Start screen: the folder row and the list are centered; all recent songs show, with a
   scroll bar when they do not fit; resizing the window keeps them centered.

## Transposed vocals

Phase 4.5, asked by the user: transposed one or two semitones (as a singer may ask a band to
do), the voice sounded like "a meme or a robot singing badly on purpose", while the singer
sounds fine singing it lower. Built on macOS and tested by the user there: **it helps going
down**, the voice no longer sounds like a robot singing, "it covers it up better". Going up,
the user hears less of a difference.

Why, as discussed: a singer singing lower moves only the pitch; the **formants** (resonances
of the throat and mouth, which give the vowels and the voice's character) stay where they
are. A plain pitch shift moves the whole spectrum, formants included. On top of that the
phase vocoder smears vibrato, consonants and breaths, and the vocal stem's separation
artifacts move with it.

- **Third bus** (`Bus::Vocal`): tracks with `TrackParams::vocal` set, which the loader sets
  for stems recognized as vocals, go through their own Signalsmith instance (same
  configuration and latency as the others). The pitch lock still wins: a locked vocal goes
  to the locked bus. Instruments are not affected.
- **Settings → Transposed vocals**, two checkboxes, both on by default, saved in
  `config.toml` (`vocal_formants`, `vocal_tonality_limit`), applied at once so they can be
  compared while playing:
  - *Keep the formants:* Signalsmith's formant compensation, `setFormantFactor(1, true)`.
    The fundamental frequency it needs is detected automatically (`setFormantBase` left at
    0). Its README warns it is less sharp than monophonic algorithms such as PSOLA.
  - *Keep the highs closer to the original:* the tonality limit at 8 kHz (the value in
    Signalsmith's README and web demo): above it the spectrum is shifted by a fixed amount
    instead of scaled.
- **Level:** measured on the vocal stem of "Afuera" (offline, whole song), formants on
  lowers it by 0.4 dB at −1 st, 0.7 dB at −2 st and 1.2 dB at +2 st; the tonality limit
  changes nothing measurable. Small, but louder tends to sound better in an A/B. A lone sine
  loses far more (0.35 → 0.13 RMS), since the correction reshapes the spectrum's envelope.
- **Cost:** one more stretcher, ~1–1.5 % of one core (the earlier measurement), plus the
  formant analysis on that bus. Not measured.
- Tests: a vocal sine is transposed with and without the options
  (`vocals_are_transposed_with_their_options`); vocals go to their bus unless locked
  (`vocals_go_to_the_vocal_bus_unless_locked`).

If this is not enough, the next step discussed is a monophonic algorithm (PSOLA, WORLD) for
the vocal stem only, rendered ahead of time when the pitch changes; backing vocals in the
same stem would confuse it. Other untried knobs: `setFormantBase` with the singer's range,
another block size for the vocal bus.

Going up is the harder direction in general: the formants have to be pulled back down
against harmonics that are further apart, so the envelope is estimated from fewer points.
Not looked into; the knobs above (`setFormantBase`, block size) or the monophonic algorithm
would be the places to start if it matters.

**Test checklist** (passed on macOS; going up, less of an improvement):
1. A song with a lead singer at −1 and −2 st: toggle "Keep the formants" while playing. Does
   the voice sound more natural with it on? Same with the tonality limit, and both.
2. At +2 st as well (formants matter more going up).
3. The other stems sound the same as before; drums stay locked.
4. Lock 🔒 the vocals: they keep their pitch, as before.
5. Close and reopen: the checkboxes are kept.

## Search in the recent songs

Phase 4.5, asked by the user. Built on macOS and tested by the user there: works.

- A search field above the start screen's list and at the top of the Recent menu. A song
  matches when every word typed is in its title or its folder, ignoring case
  (`matches_search` in `app.rs`, unit-tested). "No matches" when nothing fits.
- **Enter** in either field opens the first match that can be opened (in the menu, not the
  song already open), and is consumed so it does not also pause playback. Listed in the
  shortcuts window.
- **Menu:** the field is focused and emptied each time the menu opens, so typing filters
  right away. Clicks inside no longer close the menu (`CloseOnClickOutside`, otherwise the
  field could not be clicked); the items and "Clear list" close it themselves.
- **Start screen:** the field is as wide as the list; the text is cleared when a song is
  opened from it. The stripes alternate over the filtered rows.
- The list now keeps 100 songs (`MAX_RECENT`, was 20). Unlimited was discussed: the size is
  no problem (~150 bytes each in `config.toml`), but the config is rewritten on every open,
  every row checks that its folder exists each frame, and deleted folders pile up. The
  Recent menu lists the first 15 matches and then "N more: type to narrow".

**Test checklist** (passed on macOS):
1. Start screen: type part of a title (or two words in any order, any case): the list
   narrows; a word that matches nothing shows "No matches". Enter opens the first match.
2. With a song playing, open Recent: the cursor is already in the field. Type, then Enter:
   the first match opens and playback did not pause first. Click a filtered item: it opens.
3. With more than 15 songs, the menu ends with "N more: type to narrow".
   Close and reopen Recent: the field is empty again. Clicking the field does not close the
   menu; clicking outside does.
4. While typing in either field, shortcut keys (Space, A, B, L, M, digits) do nothing.

## Output device and transport keys

Phase 4.5. Built on macOS and tested by the user there: everything works.

- **System default** is the first entry of the output menu and the choice when
  `config.toml` has no `output_device` (an existing config keeps its fixed device until the
  user picks it). The menu shows "Default: <device>". A stream built from cpal's default
  device follows the system: on macOS (and PipeWire) cpal reroutes it and reports
  `DeviceChanged`, on WASAPI it reports `StreamInvalidated`. Both now restart the engine on
  the default device, so it opens at the new device's own rate and shows its name. A rate
  change reloads the song, with the RAM peak of the next item. The restart waits until the
  window is visible on macOS (see "Output lost when another app opens a Bluetooth headset's
  mic"), but the rerouted stream keeps playing meanwhile.
- **Space and Enter, as in Reaper** (`transport.rs`). Reaper's defaults, from its Quick
  Start guide: Space is "Play/stop", Enter "Pause"; stopping goes back to the edit cursor,
  where playback started. Here: Space plays, and pressed while playing stops and goes back
  to where it started. Enter pauses where it is, and Space or Enter resumes; a Space after
  that still goes back to where playback first started. Any seek by the user (click, ruler,
  arrows, markers, ⏮, ▶ on a section) moves that point, like Reaper's edit cursor. The ▶ / ⏸
  button behaves as Enter. Playing from the song's end starts at 0:00.
- **Arrow keys with snap on** go to the previous or next grid line drawn, so the zoom sets
  the step: every beat when zoomed in, bars (or every n bars) when zoomed out. Without a
  grid, before bar 1, or with snap off, they move 5 s as before. The ⏪ / ⏩ buttons do the
  same.

**Test checklist** (passed on macOS):
1. Output menu: choose System default. Change the output in macOS's sound settings (or
   connect headphones) while playing: the sound moves there and the menu shows the new name.
2. Pick a fixed device, change the system output: Watershed stays on its device. Restart
   the app: the choice is kept.
3. Space plays; Space again stops and goes back to where it started. Enter pauses in place;
   Space or Enter resumes; Space then goes back to the first start.
4. Click elsewhere (or use the arrows) while playing, then Space: it goes back to the click.
5. Snap on, zoomed in on a 4/4 song: ← / → move beat by beat; zoomed out, bar by bar.
   Snap off: 5 s.

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
  marker while paused), the engine seeks, then counts with the stretcher frozen. The
  song starts when the count reaches the frame the stretcher will play first, so the
  count and the song form one continuous pulse (tested). A seek during the count-in
  starts the song at once. `Engine::counting_in()` keeps the playhead still
  meanwhile. Not on loop wraps, nor on seeks while playing. Two ways to count
  (2026-11-12, a checkbox "Count in to the bar line" in the ⏷ menu, `count_in_to_bar`
  in `config.toml`, on by default), see "Count-in on the bar line" below.
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

## Count-in on the bar line

Asked by the user (v0.2, rule agreed 2026-11-11). Done 2026-11-12, **not tried by the
user yet**. Before, the count-in clicked the four beats before the first beat at or after
the playhead, so from beat 2 it counted "2 3 4 1" and a pickup threw it off.

- **On (default), `Beats::count_in_to_bar`:** the count goes to the first bar line at or
  after the playhead, on the grid. From a bar line: the bar before it ("1 2 3 4", song on
  the 1). From the second half of a bar, **by time** (so 7/8 and 9/8 work too): that bar
  from its start, the song comes in at the playhead over the last clicks, and the bar line
  clicks as the "1" (from beat 4: "1 2 3", song on the 4). From the first half: one bar
  more (from beat 2: "1 2 3 4 | 1", song on the 2). These clicks sound even with the Click
  off; beats before bar 1 count at the pace of the grid's first beat.
- **Off:** as in Reaper, one whole bar of clicks in the meter and tempo of the bar the
  playhead is in, then the song from the playhead, wherever it falls in its bar.
- **Nothing before the playhead:** the stretcher's seek fills its pre-roll with the song
  before the target, so ~45 ms of what came before the playhead used to play first (also
  in the old count-in; its test could not see it, its song started at the playhead). With
  a count-in the pre-roll is now silent before the target, with a 2 ms fade-in at it.
  Seeks without a count-in keep the old pre-roll, so a click on the waveform is heard at
  once.
- **Tests:** `count_in_lands_on_a_bar_line` (beats, 4/4, before bar 1, 7/8),
  `clicks_and_count_in_follow_a_tempo_map` (both ways on a tempo map),
  `count_in_to_the_bar_line_overlaps_the_song` (engine: clicks and where the song comes
  in, from beat 4 and from beat 2).

**Test checklist:**
1. "My Own Summer" (Deftones: 72 BPM, bar 1 at 0.92 s, a drum pickup before it), Count-in
   on, from 0:00: "1 2 3", the song starts just before the 4 (0:00 is 0.09 s before it),
   the 4 clicks over the drum pickup, then the 1 on bar 1. Is this what you meant?
2. From a bar line: "1 2 3 4", song on the 1. From beat 2: "1 2 3 4 | 1", song on the 2.
   From beat 4: "1 2 3", song on the 4. Also with the Click on (no doubled clicks).
3. A song in 7/8 or with time signature changes ("46 & 2"): the count follows the bar the
   playhead is in.
4. The checkbox off: one bar of clicks, then the song from the playhead.

## Dynamic tempo

- **Beat tracking** (`OnsetEnvelope::track_beats` in `tempo.rs`): on the summed onset
  envelope the loader already keeps, scaled by its standard deviation over the
  8 s around each frame (floor: 5 % of the song's), so a guitar alone counts as
  much as the band. First the exact pace within ±10 % of the song's BPM (the
  user's value only picks the beat, not half or double: from 130 on Change it
  dropped 34 beats before this). Then a **tempo curve** (`tempo_curve`): for each
  5 s window every 1 s, tempos within ±15 % scored on the window's
  autocorrelation (4 beat periods), relative to the window's best and times the
  window's confidence: onset strength (local / global deviation, at most 1) times
  contrast ((best − mean) / best over the tempos tried, ~0 for noise), chained by
  Viterbi with a cost of 55 · ln(ratio)² per change; with no confident window the
  tempo holds.
- **Vocals weigh 0.25** in the onset sum (`VOCALS_TEMPO_WEIGHT`, applied by the
  loader to `StemKind::Vocals` when other stems exist; the `beats` example by file
  name). Singing follows the beat loosely: alone it gave wrong tempos on several
  songs (BPM table), and in Change's bridge, where it is the loudest stem, the sum
  read 69/138 with it and 130.4 without. Then Ellis's (2007) dynamic
  programming with the local period p(t): score(t) = onset(t) + max over lags in
  [p/2, 2p] of score(t − lag) − 680 · ln(lag / p)², followed back from the best end.
  Each beat is then refined to a fraction of a frame (parabola through the peak) and
  smoothed by a straight line through the 8 beats around it, **weighted by their
  onsets** (without the weights, beats placed in a silent intro pulled the first
  real ones by up to 9 ms). With clicks, steady or speeding up 120 → 132 BPM, and
  from a BPM 6 % off: within 2.3 ms; a quiet intro at 122 BPM into a loud band at
  134: within 10 ms (tests). Takes 20–40 ms per song in release.
- **Change's intro** (found by the user): the first two bars are 122 BPM (bar 1 at
  0.25 s, saved in that song's session), then a little faster, then the band at the
  drums' tempo. The intro guitar plays dotted quarters (0.735 s apart = 122 BPM),
  about 20 times quieter than the band. Before the curve the tracker filled it at
  137. Tried and dropped: a narrower curve (±6–8 %) and ignoring weak windows
  (both put the intro back at ~137). Now: bars 1–2 at 122.5–123 (within 17 ms of
  the user's 122 grid), bars 3–8 at 122–125, the band from bar 9 (0:16) at 131 →
  134 → 137. Same result from a BPM of 122 or 137.
- **Change's bridge** (found by the user: "from 2:31 it goes wild"): a bar starts
  at 2:31.50 and the bridge is ~130 BPM (the user's steady 130 grid fits it
  roughly). The tracker was right up to there (bar 87.1 at 2:31.50, +2 ms), then
  ran on at ~139 for 3 s and gained a beat, so from then on the "1" was on beat 2
  (the beats themselves later fell on the drums again, within ±25 ms). Causes: the
  vocals (above) and the 8 s windows mixing 140 and 130 around the change. Now:
  bar 87.1 at 2:31.50, 94.1 at 2:44.42 (on the user's 130 grid), 103.1 at
  3:01.09, the band's first hit back (drums, bass and guitar together; exactly
  16 bars of 130 after 2:31.50). Windows of 4 and 5 s both do it; 6 s lost the
  beat at the end of the bridge instead. The test
  `follows_a_sudden_drop_without_gaining_a_beat` (140 → 125 with a dotted riff)
  fails with 8 s windows; a drop to 120 still gains a beat.
- **End of the rhythm** (asked by the user: at Change's end "the BPM goes wild"
  as the instruments ring out). `OnsetEnvelope::rhythm_end`: the last onset at
  least 30 % of the 95th percentile (log-compressed flux: notes ringing and
  feedback stay below it, new attacks do not). Past it the beats go on steadily
  at the median gap of the 8 beats before it (`hold_after`), so one odd beat in a
  last fill does not set it. Change: the rhythm ends at 4:44.6 (the drums stop),
  then 148.2 BPM, the pace of the last fill (its beats sit on the drum hits).
  The other test songs end 3–15 s before their files do.
- **Tool's "46 & 2"** (separated by Watershed; the bass intro came out in the
  guitar stem). The user's bar 1 at 0.24 s and 79 BPM. The tracker alone reads
  107 BPM (4/3 of 79: the riff moves in dotted eighths) and, at 79, put the beats
  on the riff's offbeats (−0.3 to −0.4 beat) and ran at 84 BPM over the first
  bars. Two changes: **bar 1 holds a beat** when the user put it on a note (an
  onset of at least 1 local deviation within 60 ms; every path after it goes back
  through it, and it weighs 10⁶ in the smoothing), and **beats in silence keep
  the tempo** (a gap ending on an onset under 0.5 pays 20 000 · ln(lag / p)²
  instead of 680). Now the first bars are at 79.1, on the user's grid (+42 ms on
  average over the first 16 beats). Still wrong: from ~0:12 to ~0:35 the beats
  slide to the offbeats again (−130 to −165 ms) and come back once the drums
  carry the beat (from ~0:35 within ±80 ms of the steady 79 grid, following the
  drummer's drift, which the drum stem shows too). Tried and dropped: a higher
  tightness (2000, 5000: no change) and gaps limited to ±15–33 % of the period
  (the riff's offbeats sound more often than its beats, so on the onsets alone
  they are the better path; the limit only moved the slip, and broke the sudden
  drop test). Only more marks from the user can fix that. Change: unchanged, its
  bar 1 (0.12 s, placed to line the steady grid up with the bridge) is on no note,
  so it does not hold; the bridge bars land as before (87 at 2:31.50, 94, 103).
  Other songs unchanged.
- **Checked by the user after the bridge fix:** "almost perfect"; some parts run a
  little ahead, which makes sense; at 4:05 a short drum fill really speeds up and
  the tempo relaxes after it.
- **Measured** (`cargo run --release -p watershed-dsp --example beats -- <stems>`;
  spread of the local tempo from its 5th to 95th percentile): Karaoke 0.9 %, No
  Dejes Que 1.4 %, Afuera 1.8 %, Cada hogar 1.6 %, On Most Surfaces 1.9 %, Change
  **15 %** (122 in the intro, 136–143 with the band, speeding up around
  1:52–2:20 and 3:15–4:20, ~130 in the bridge at 2:31–3:00, 146 in the fill at
  4:05, 148 in the last fill). Against
  the steady grid, the tracked beats of steady songs drift slowly: Afuera ends 80 ms
  early, No Dejes Que goes from −67 to +61 ms. That is the band (or a BPM a hair
  off), and what this feature is for.
- **Grid** (`grid.rs`): the beats are tracked on load and again when the BPM changes
  by more than 5 % (½, ×2, a tap far off; dragging the BPM does not retrack), and,
  with Dynamic on, when the user's bar 1 moved (once a drag ends: tracking takes
  25–65 ms). With **Dynamic** on, the beats are counted from the user's bar 1 when
  it is on a note; bar 1 is the tracked beat nearest to it, and the grid lines,
  snap, click and count-in follow the tracked beats. Off: as before.
- **Engine**: `watershed_audio::Beats` is now a list of beat times from bar 1 (a
  steady grid is listed up to the song's end), extended at the pace of the first
  and last beat (count-in before bar 1, after the end). It reaches the audio thread
  through the command queue and comes back on the garbage queue, like songs; the
  engine sends it only when it changed. The count-in counts the bar before the
  target at its local pace.
- **UI**, after the BPM: a **⚡** (orange; a ★ before) when the local tempo spreads more than 3 %,
  with the range in its tooltip, and the **Dynamic** toggle (off by default, saved
  as `dynamic_tempo` in the session). With Dynamic on, the BPM shows the tempo of
  the beat at the playhead (times the tempo slider) and cannot be dragged; ½, ×2,
  Tap and ↺ still work. The tracked beats are not saved: they are recomputed on load.
- Checked by Claude on Change: the ⚡ shows; with Dynamic on, the BPM reads 139 at
  2:30 and the bar ruler ends at bar 169 instead of ~160.
- **Not done** (open points in `PLAN.md`): a song with a free section (the tracker
  goes on through it at its pace), correcting single beats, a tempo change on
  purpose (the smoothing turns a step into an 8-beat ramp).

**Test checklist:**
1. Change: the ⚡ shows; turn on Dynamic and Click, play from a few places (also
   2:00 and 3:30, where it speeds up): does the click stay with the drums the whole
   way, with the accent on the "1"?
2. The same song with Dynamic off: the click drifts (by ~0.2 s at times).
3. A steady song (Karaoke, Cada hogar): no ⚡; Dynamic on still clicks in time.
   Afuera with Dynamic: does it sound better than the steady grid near the end?
4. Count-in with Dynamic on, from a bar line late in Change: one bar at the local
   pace, then the song on the "1".
5. Zoomed in with Dynamic: the beat lines fall on the hits; snap lands on them; drag
   the bar ruler: bar 1 moves by whole beats.
6. ½ and ×2 with Dynamic on: the grid follows (half or double beats).
7. Close and reopen: Dynamic is kept; loading feels as fast as before.

## Time signature changes

Design agreed with the user: like Reaper's time signature markers, a change holds
from there on (a short bar is two changes: one to 1/4, one back); the BPM counts
the beat unit of bar 1's signature and **the note value stays the same** across
changes (79 BPM in 4/4 → a 7/8 bar is 7 eighth notes at 158); placed and edited on
the bar ruler.

- **Session:** `[[signatures]]` with `time` (seconds, fixed to the song like the
  markers), `beats_per_bar` and `beat_unit`. Bar 1's signature stays in
  `beats_per_bar` / `beat_unit`. Changes before bar 1 are kept but do nothing
  (drawn faded).
- **Bar ruler**, now two rows: the time signatures on top (orange tags: bar 1's,
  then each change), bars below. **Right-click** the ruler: a change there (on the
  nearest line drawn when the magnet is on), with the signature in force, and its
  editor opens. **Shift+C** (Reaper's shortcut, asked by the user; C alone still
  toggles the click): a change exactly at the playhead, like M for markers, and
  its editor opens over its tag. **Click a tag:** a popup with beats per bar, the beat unit (2, 4,
  8, 16) and Delete (bar 1's has no Delete). **Drag a change** to move it (not past
  its neighbours). Dragging elsewhere still moves bar 1.
- **Steady grid** (`grid.rs`): each stretch starts at its change, at the BPM
  scaled to its beat unit. A change off the grid cuts the bar before it short; a
  beat closer than ¼ beat to the change is left out (no slivers). Bars are
  numbered across changes, a cut bar counting as one.
- **Dynamic:** each change is an anchor for the tracker, as bar 1 already was
  (`track_beats(bpm, &[Anchor])`, `Anchor { time, pace }`): from there the beats go
  at its pace (2 for eighth notes in a song counted in quarters), and on a note it
  holds a beat and every later path goes through it. The gap ending on an anchor
  keeps the pace before it, and the smoothing runs separately on each side of a
  pace change (sharing the anchor's beat). Test: 16 quarter notes, then 7/8 grouped
  2+2+3: every quarter and eighth note within 10 ms. The grid's bars start at the
  tracked beat nearest each change. Retracked when a change is added, edited or
  moved (once a drag ends).
- **Engine:** `Beats` holds meters (`Meter { from, per_bar }`): accents, bar numbers
  (`bar_of`) and the count-in's bar length (the meter before the target) follow
  them. Nothing allocates in the callback.
- `beats` example: `--pace <seconds>:<pace>` (repeatable) adds a change of pace.
- Checked by Claude on a copy of "46 & 2" with 7/8 at bar 5 and 4/4 two bars later:
  the tags show, bars 5–6 are short, 4/4 again from bar 7.
- **Tested by the user** on "46 & 2": 7/8 at 0:37 for two bars, and the 7/8 parts
  from 4:05 to 5:24 (10 changes in all), Dynamic on: the grid and the click line
  up with the song. The marks are what fixed it: from each 7/8 change the tracker
  follows eighth notes (a quarter-note pulse does not fit 3.5 quarters a bar), and
  each change on a note pins a beat that the beats after it count from.

**Test checklist:**
1. Right-click the bar ruler on a bar line: a tag appears with the current
   signature and its popup; set 7/8. The bars after it regroup; the bar numbers go
   on.
2. Right-click a few bars later and set 4/4 again (a short bar: 1/4 then back).
   Also with Shift+C at the playhead: the popup opens there and the click does
   not toggle.
3. Click a tag: change it, or Delete. Click bar 1's tag: it changes the same
   signature as the controls beside the ruler.
4. Drag a tag: the bars follow; it does not pass its neighbours. Dragging the
   ruler elsewhere still moves bar 1.
5. Click and count-in across a change: accents on each new "1"; a count-in into a
   7/8 bar counts the bar before it.
6. With Dynamic on, in "46 & 2": a change on a note pins a beat there; a 7/8 part
   follows eighth notes.
7. Close and reopen: the changes are kept.

## Tempo changes

Asked by the user after "Paranoid Android" lost the beat at 3:27: tempo marks like
Reaper's. As in Reaper, **one kind of mark** carries a tempo, a time signature or
both (Shift+C, right-click on the bar ruler); what it does not set goes on as before.

- **Why it was needed:** the tracker's tempo curve looks within ±15 % of the song's
  BPM. Paranoid Android is 81.8 BPM, its slow part from 3:27 is 62.9 (−23 %): the
  tracker stayed at ~84 and the beats wandered ±200 ms. Measured with the `beats`
  example and the user's 7/8 marks: with a tempo mark at 3:27 of 55, 63 or 70 BPM,
  the slow part reads 62.8–62.9 and holds (±50 ms against a steady grid), so a
  rough tempo is enough. Where the fast riff comes back (~5:28) another tempo mark
  (~82) is needed.
- **Tracker:** `Anchor` has `tempo: Option<f64>`. The tempo curve's Viterbi now
  runs over ratios (±15 %) to a per-frame tempo: the song's BPM, or the last tempo
  mark's from its point on, so a mark moves the curve for free. The gap into a mark
  keeps the tempo before it; the smoothing runs apart on each side of a tempo mark.
  Test: 120 BPM then 80 (a third slower) with a mark of 85: every beat within
  10 ms; without the mark it misses by over 50 ms.
- **Steady grid:** each stretch at its tempo. Tempos count the beat unit of bar 1,
  like the song's BPM.
- **BPM shown:** with Dynamic on, or past a tempo mark, the BPM shows the local
  tempo in bar 1's beat unit. Fixed with it: inside a 7/8 part it read double (the
  eighth notes' tempo).
- **Popup:** "From m:ss", a **Tempo** checkbox with its BPM (20–400, typed or
  dragged) and a **Time signature** checkbox with n/m, a note, Delete. A box turned
  on starts from what is in force before the mark. A new mark starts with the time
  signature in force and no tempo, as before. Tags read `63 BPM`, `7/8`,
  `63 BPM 7/8`, or `Bar` (neither: still starts a bar).
- **Session:** `[[changes]]` with `time` and optional `bpm`, `beats_per_bar`,
  `beat_unit`. `[[signatures]]` from the previous version still loads (serde alias)
  and is written back as `[[changes]]`.
- `beats` example: `--tempo <seconds>:<bpm>` (repeatable), `--list <from>:<to>`
  (every beat in that range with its tempo).
- **First try by the user:** a 70 BPM mark at 3:27.3 "is not followed". Measured
  apart with librosa (autocorrelation per stem, 10–20 s windows): the band from
  3:34 is really 63 BPM (drums, guitar and vocals all 63/126); the fast riff comes
  back at ~5:33 (82). The tracker took 70 as a hint and found 63. The real issue:
  3:27.3 → 3:34.0 is the band letting a chord ring, **two bars of 4/4** for the
  user (the "70-something" they heard is the tempo that fits them: 71.6), while at
  63 the tracker put 7 beats there, so the "1" fell on beat 4 at 3:34.
- **Fixed and Fit** (asked by the user, "forced"): a tempo mark can be **Fixed**:
  exactly its BPM up to the next mark, steady, even with Dynamic on (the tracked
  beats are replaced there). **Fit** (button beside the BPM, when a mark follows)
  sets the tempo that fills whole bars up to the next mark, rounding the bar count
  from the current tempo (`Grid::fit_tempo`). Tag: `71.6 BPM fixed`. Session:
  `fixed = true`. For the cut: 3:27.3 Tempo + Fixed + Fit, 3:34.0 the band at 63
  (on the drum hit, it holds the "1"), ~5:33 back to 82. `Grid::beats` now builds
  the beats per stretch: steady (no Dynamic, or Fixed) or tracked (from the beat
  nearest the stretch's start to before the one nearest the next).
- Checked by Claude on a copy of the user's session with the 3:27 mark Fixed at 70:
  bars 73–74 ring, bar 75 starts at 3:34 with the band.
- **How the user marked it** (works): 3:27.27 `71.36 BPM fixed` (Fit: two bars
  ringing), 3:33.99 `Bar` (no tempo, no signature: only the band's "1"), 5:36.20
  `80.17 BPM fixed` (Fit: one bridge bar), 5:39.19 `Bar` (the fast riff). Checked
  with the `beats` example and these marks: a beat right at 213.992 (the Bar on
  the drum hit holds it), the slow part at 62.8–62.9, the bridge bar at 80.2, then
  81.7–82.2. **Caveat:** a Bar mark has no tempo of its own, so the tracker looks
  around the tempo before it (here the let-ring's 71.36; the band's 63 is 12 %
  below, inside the ±15 % reach). A let-ring fitted much further from the next
  part's tempo would need a tempo on that Bar mark. With Dynamic off, that part is
  drawn at the inherited tempo (71.36), not 63. Idea if it bites: after a Bar
  mark without a tempo, let the tracker estimate its tempo over a wider range.
- Checked by Claude on a copy of "Paranoid Android" with a 63 BPM mark at 3:27: the
  tag shows, Shift+C opens the new popup, the BPM at 3:20 (in 7/8) reads 85.
- **½ and ×2 with tempo marks (fixed):** reported by the user on Opeth's
  "Deliverance": the BPM read close to 200 and ½ / ×2 did not change it. Its bar 1
  is 7/8, so the BPM counts eighth notes (~200 eighths ≈ 100 quarter notes in its
  4/4 parts), and its mark at 1:09.7 sets 171.9 BPM (Fixed). The buttons only
  scaled the song's BPM, while from the first tempo mark on every tempo comes from
  the marks, so nothing moved after 1:09 (measured on the user's session: before
  the mark 123 → 72 → 247, after it 172–198 whatever the button). Now ½ and ×2
  also scale every mark's tempo (`Grid::scale_tempos`).

**Test checklist:**
1. Paranoid Android, Dynamic on: the mark at the cut (3:27.3) with Tempo, Fixed and
   Fit: two whole bars ring. The mark at 3:34.0 with ~63: the "1" with the band;
   the grid and the click follow the slow part.
2. Where the fast riff comes back (~5:33), another mark with ~82: the grid follows
   again.
3. Dynamic off: the grid is steady at each mark's tempo.
4. A mark with only a time signature behaves as before; with both, the tag shows
   both.
5. Close and reopen: the marks are kept (the session now writes `[[changes]]`).

## Remember the window

v0.2, done 2026-11-12. **Tried by the user on macOS: works**, maximized (Zoom) included.
**Still to try on Windows**, above all maximized and closed while minimized. eframe's `persistence` feature saves
the window (`app.ron`: position in physical pixels, content size in points, maximized,
full screen) every 30 s and on exit, restores it, and on Windows moves it back onto a
monitor if its monitor is gone (macOS does that itself). `window.rs` replaces what eframe
saves with the last geometry seen while the window was normal, because eframe takes it
from the window as it is at that moment:
- **Minimized:** Windows reports a minimized window at −32000 with a size of 0; eframe
  would reopen it at 64 × 64. Now it opens as it was before minimizing.
- **Maximized:** eframe would save the maximized size as the normal one, so unmaximizing
  after a restart would not shrink the window. Now it saves the normal size with
  `maximized: true`.
- **Full screen:** a window created in full screen on macOS leaves it at the size of the
  screen. Now it opens normal and goes to full screen on its first frame
  (`watershed_fullscreen` in `app.ron`).

egui's own state is not saved (`persist_egui_memory` is off), only the window. A test
checks that what `window.rs` writes reads back through egui-winit's `WindowSettings`
(their fields are private, so a renamed field would come back as its default).

**Checked on macOS** by driving the window with System Events: position and size restored
across runs with no drift; minimized then Cmd+Q reopens normal; full screen (the green
button) reopens in full screen and leaving it gives back the normal size and place.
Maximized (macOS's Zoom could not be scripted) was tried by the user.

## Cents and reference tuning

v0.2, done 2026-11-12, tried by the user: works.

**Cents.** A `+0 ct` value beside the semitones in the bottom bar: drag or type, −50 to
+50, double-click resets it. The engine and the session keep one `f32` in semitones (the
fraction is the cents), so old sessions load as they were. `split_cents` (`app.rs`) shows
it as whole semitones and cents; exactly half a semitone stays on the semitone nearer to
zero (+50 ct is not shown as +1 st −50 ct). The key shown follows the whole semitones.

**Reference tuning.** The Key tooltip and the top of its menu show the song's tuning as
`A = 430.7 Hz`, with `(about 432)` when within 3 cents of 432, 435, 442 or 444 Hz, and
"as heard" when cents are applied. **Set to 440 Hz** sets the cents to cancel the measured
tuning (Black Hole Sun: +37 ct), keeping the semitones; disabled when already within half
a cent.

**Accuracy, checked** on Black Hole Sun's stems without drums, relabelling their sample
rate to move them to known tunings (`key` example):

| Moved to | Expected | Measured |
|---|---|---|
| A = 440 | 0 | −1 |
| A = 432 | −32 | −32 |
| A = 444 | +16 | +15 |
| A = 425 | −60 | Gm, +39 (G#m −60 is Gm +40) |

The tuning is measured against the nearest semitone, so beyond ±50 cents it reads as the
neighbouring key; Set to 440 Hz then tunes to that key, which is right.

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
- A third bus for vocals came later (see "Transposed vocals").
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

## Waveform shift and tempo at bar 1

Two v0.2 bugs, fixed 2026-11-11.

**Waveform shifting with the volume.** Cause as guessed in the plan: the controls column
grew with its contents, and the fader's value changes width with the value. Now
`controls_column` (`app.rs`) allocates exactly `CONTROLS_WIDTH` for the rulers' and each
track's controls and clips what does not fit; the volume value is as wide as its widest
text (`-59.9 dB`), and the volume slider takes the width the pan leaves (the rows had been
wider than the column: the pan value got cut once the column stopped growing).

**Fixed tempo on bar 1 ignored with Dynamic on** ("Be Quiet and Drive": 80 BPM fixed on
bar 1, the grid followed the tracked ~90 BPM). Not the tracker: a change at bar 1 (or
less than `MIN_CHANGE_GAP` after it, or before it) gave bar 1 its tempo through
`in_force`, but the first segment was always `fixed: false`, and its tempo never reached
the tracker, because `anchors()` skipped changes at bar 1. Now the last change at or
before bar 1 that sets a tempo (`tempo_at_bar_one`) makes the first segment fixed if it
is, goes to the tracker as bar 1's anchor tempo, and its tag is no longer greyed out.
Checked in the app: the intro now holds 80 BPM, 8 bars that end right on the 4/4 mark at
24.26 s. Test: `a_tempo_set_at_bar_one_holds_when_dynamic`.

Tried by the user: both fixes work. **Follow-up, a regression of that fix:** from bar 22
(~60 s) the dynamic grid slowed to ~74 BPM and stayed there until ~3:00, while the drums
keep a clear ~92 BPM pulse (autocorrelation per 16 s window: 0.5–0.65 at 92 BPM, ~0.25 at
74). Cause: a tempo anchor steers the tracker from there on, until another anchor sets a
tempo; the 4/4 marks set none, so the fixed 80 BPM of the intro now steered the whole
song, and the tempo curve only looks within ±15 % of it (68–92 BPM). Fixed: a fixed tempo
is for its own stretch, also for the tracker: the change that ends it gives the tracker
the tempo from before it again (the song's BPM, or the last tempo change that is not
fixed). With Dynamic off nothing changed: the steady grid carries the last tempo on, as
Reaper does. Checked with the `beats` example (92–95 BPM through the song, rising to ~98
at the end, which librosa agrees with) and in the app (the 4/4 mark at 3:41 falls on bar
85), and tried by the user: works. Test: `the_tracker_follows_a_fixed_tempo_only_in_its_stretch`.

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
