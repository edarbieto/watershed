# Features

Everything Watershed does, by topic. What changed in each version is in the
[changelog](../CHANGELOG.md). Press **?** in the app for every key and mouse gesture.

## Separating songs

- **Drop a song** (mp3, flac, wav, ogg or m4a) on the window, or use File → Separate a song.
  It is split into six stems: vocals, drums, bass, guitar, piano and other.
- **All on your computer:** nothing is uploaded. The model, BS-RoFormer SW, runs on the GPU:
  Metal on Apple Silicon Macs, Vulkan on Windows and Linux. A 4 minute song takes about
  1–2 minutes on a laptop or a mid-range graphics card.
- **The model downloads once**, on the first separation (700 MB, checked by sha256), and is
  stored at half the size (350 MB). An interrupted download resumes.
- **Progress, time left and Cancel** while it works.
- **Stems go to a library folder** (`Music/Watershed` by default, changed in Settings), one
  folder per song, as Ogg Vorbis (~25 MB per song) or 24-bit FLAC.
- **The song's tags are kept:** title, artist, album, track, year and genre go into the
  session and into every stem, so other players and DAWs show them too.
- A song is never separated twice: its folder opens as you left it.

## The player

- **One track per stem** with its stereo waveform, volume, pan, mute and solo. Solo wins over
  mute. Master volume and output device in the top bar; the default device can be followed
  as it changes.
- **Stems from other tools open too** (for example [MVSEP](https://mvsep.com)): open the folder,
  drop it on the window, or pass it on the command line. Stems are recognized by name, in
  English or Spanish (`vocals`/`voz`, `drums`/`bateria`, `bass`/`bajo`,
  `guitar`/`guitarra`, `piano`/`teclado`, `other`, `instrumental`), and each kind has its own
  color.
- **Select a track** by clicking it (↑ / ↓ moves the selection): M mutes it, S solos it,
  Delete removes it. A removed track keeps its file and comes back from the Edit menu.
- **Rename a track** with a double-click on its name.
- **Move a track in time** when it starts late: unlock it and drag its waveform.
- **Seek** by clicking or dragging the time row, or dragging the playhead line.
- **Zoom** with + / −, Cmd or Ctrl + scroll, or a pinch; the view follows the playhead.
- **Space and Enter as in Reaper:** Space plays, or stops and goes back to where playback
  started; Enter pauses where it is.

## Tempo and pitch

- **Slow down or speed up without changing the key,** in 5 % steps or to any value.
- **Transpose** in semitones and **cents** (−50 to +50), without changing the speed.
- **Drums stay untransposed** by default (a lock per track), and **vocals keep their
  formants** when transposed, so they do not sound like chipmunks (a setting).
- A master limiter keeps loud stems from clipping at the output.

## Loops, markers and sections

- **A-B loops:** A and B at the playhead, or drag over a waveform or the loop row. L turns
  the loop on and off.
- **Save as many loops as you like** per song, name them, and play any of them from the
  markers and loops panel.
- **Markers** (Shift+M, or double-click the markers row) name the song's sections: verse,
  chorus, solo. Keys 1–9 jump to them, and a section can be played or looped on its own.

## Tempo, grid and click

- **BPM and key are estimated** when a song opens. Tap tempo (T) and ½ / ×2 correct the BPM.
- **Reference tuning:** the key's tooltip shows the song's tuning (for example A = 431 Hz),
  and a button retunes it to A = 440 Hz.
- **Beat grid** with a bar ruler, bar 1 that can be moved, and snap to the grid.
- **Dynamic tempo:** the grid and the click follow the song's own beats, for bands that did
  not play to a click.
- **Tempo and time signature changes,** marked on the ruler as in a DAW, or a fixed tempo for
  a stretch of the song.
- **Click** in the left, right or both ears, with three sounds (click, metronome, wood) and
  its own volume, and a
  **count-in** that lands on the bar line, so a song started mid-bar or with a pickup comes
  in at its real place.
- **Chords** (optional, H): an estimated chord lane under the bar ruler, transposed with the
  pitch. It is a guide, not a transcription.

## Sessions and the library

- **Everything is saved with the song** in `.watershed.toml` next to the stems: mix, loops,
  markers, grid, tempo, pitch, track names and offsets.
- **Undo and redo** for all of it, with the keys or the buttons in the top bar.
- **Recent songs** on the start screen and in the File menu, shown by title and artist, and
  searchable by title, artist, album or folder name.

## The app

- **Menus** with every command and its key: native on macOS and Windows, built in on Linux.
- **Light, dark or system** appearance.
- **CPU, GPU and RAM use** in the top bar.
- The window reopens where it was, at the same size, maximized or full screen.
- **Updates itself:** Settings → Updates downloads the new version, checks it and installs it.
- Runs next to your DAW: Watershed plays the band, your instrument goes through Reaper (or
  whatever you use). On Windows it uses shared-mode WASAPI, so it can share an interface
  with Reaper on ASIO at the same sample rate.
- Windows 10 and 11, macOS 11 or newer on Apple Silicon, Linux on x86_64. Separating needs a
  GPU; playing stems does not.
