# Changelog

What changed in each version of Watershed. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and versions follow
[Semantic Versioning](https://semver.org/). The full list of what the app does is in
[docs/FEATURES.md](docs/FEATURES.md).

## [Unreleased]

### Added

- **Credits and licenses:** every download now carries `THIRD-PARTY-LICENSES.txt` with
  the licenses of the code, crates and fonts Watershed is built on, and the About window
  credits them. The README has a Credits section.

## [0.2.0] - 2026-11-15

### Added

- **Undo and redo** for everything saved with a song (mixer, loops, markers, grid, tempo
  and pitch, renamed or deleted tracks): Cmd+Z / Cmd+Shift+Z on macOS, Ctrl+Z /
  Ctrl+Shift+Z / Ctrl+Y elsewhere, and buttons in the top bar.
- **Fine pitch in cents** (−50 to +50) next to the semitones, and the song's **reference
  tuning** (for example A = 431 Hz) in the key's tooltip, with a button that retunes the
  song to A = 440 Hz.
- **Chords on the grid:** an estimated chord lane under the bar ruler (major and minor
  triads), transposed with the pitch. Off by default; it is an estimate, not a
  transcription.
- **Count-in that lands on the bar line:** it counts the bar before the next bar line,
  so a song started mid-bar or with a pickup comes in at its real place. A setting turns
  it back into one plain bar of clicks.
- **Track selection:** click a track, move with ↑ / ↓, then M mutes it, S solos it and
  Delete removes it. A deleted track keeps its file and can be restored from the Edit
  menu.
- **Drag the playhead line** over the waveforms to move it.
- **Menus:** a native menu bar on macOS and Windows (a built-in one on Linux) with every
  command and its key; every key now has a menu item. New keys for tempo and pitch steps.
- **Appearance setting:** light, dark or follow the system, with a new light theme.
- **New look:** Geist fonts, Material Symbols icons, and the controls grouped with labels
  in the bottom bar.
- **Settings, shortcuts and About open as real windows** above the main one.
- The window reopens where it was: place, size, maximized or full screen.
- **Song tags kept after separating:** title, artist, album, track, year and genre are
  read from the song file, saved with the session and written into the stems. The recent
  songs show "Title · Artist" and can be searched by artist and album.
- The shortcuts window lists the mouse gestures too.
- A [website](https://edarbieto.github.io/watershed/) with downloads for each system.

### Changed

- **Add a marker is now Shift+M** (M mutes the selected track).
- The ruler has one row per job: time, loop, markers, bar 1, tempo and time signature
  changes, and bars.
- Release files no longer carry the version in their names
  (`Watershed-macos-arm64.dmg`, `Watershed-windows-x86_64.zip`,
  `Watershed-linux-x86_64.AppImage`), so the download links always point to the latest.

### Fixed

- A track's waveform no longer shifts against the ruler when its volume changes.
- A fixed tempo set on bar 1 is kept when the grid is dynamic, and no longer steers the
  beat tracker past the next tempo mark.
- The count-in uses the time signature of the bar the playhead is in.
- The Windows menu bar follows the app's theme.

## [0.1.1] - 2026-11-05

### Added

- **Auto update** from GitHub Releases: Settings → Updates downloads the new version,
  checks it and installs it in place of the old one.

### Fixed

- File dialogs no longer block the app (resizing one crashed it on macOS).
- The dialogs for a song and for a folder of stems remember their own start folders.

## [0.1.0] - 2026-11-05

The first release.

### Added

- **Separation on your computer:** drop a song (mp3, flac, wav, ogg, m4a) and it is split
  into vocals, drums, bass, guitar, piano and other with BS-RoFormer, on the GPU (Metal on
  macOS, Vulkan on Windows and Linux), with progress, time left and Cancel. The model
  downloads on first use and is stored in f16 (350 MB). Stems are saved as Ogg Vorbis or
  24-bit FLAC in a library folder.
- **Multitrack player:** a waveform, volume, pan, mute and solo per stem; master volume
  and output device. Folders of stems from other tools open too, recognized by name in
  English or Spanish. Tracks can be renamed and dragged in time.
- **Tempo and pitch:** slow down or speed up without changing the key, transpose in
  semitones; drums stay untransposed by default and vocals keep their formants.
- **Loops and markers:** A-B loops, several saved per song, named markers and sections.
- **Tempo and key:** estimated BPM and key, tap tempo, a beat grid with a bar ruler and
  snap, a click and count-in, **dynamic tempo** that follows bands not playing to a click,
  and tempo and time signature changes.
- **Sessions:** the mix, loops, markers and grid are saved next to the stems, and recent
  songs are listed (and searchable) on the start screen.
- Waveform zoom, a shortcuts window, CPU / GPU / RAM use in the top bar.
- Builds for Windows (zip), macOS (dmg) and Linux (AppImage).

[Unreleased]: https://github.com/edarbieto/watershed/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/edarbieto/watershed/compare/v0.1.1...v0.2.0
[0.1.1]: https://github.com/edarbieto/watershed/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/edarbieto/watershed/releases/tag/v0.1.0
