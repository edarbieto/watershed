//! Sessions, model cache and app configuration on disk.

use std::path::{Path, PathBuf};

use directories::{ProjectDirs, UserDirs};
use serde::{Deserialize, Serialize};

mod session;

pub use session::{GridChange, Marker, SESSION_FILE, SavedLoop, Session, TrackState};

fn project_dirs() -> Option<ProjectDirs> {
    ProjectDirs::from("", "", "Watershed")
}

/// Where downloaded separation models live: a `models` folder in the OS cache
/// folder, since they can always be downloaded again.
pub fn models_dir() -> Option<PathBuf> {
    project_dirs().map(|d| d.cache_dir().join("models"))
}

/// User settings, stored as `config.toml` in the OS config folder.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// Id of the last selected output device; `None` follows the system default.
    pub output_device: Option<String>,
    /// Folder the last song to separate was picked from, to start that dialog there.
    pub last_song_folder: Option<PathBuf>,
    /// The last folder of stems opened, to start the "Open folder" dialog next to it.
    pub last_stems_folder: Option<PathBuf>,
    /// Song folders opened recently, most recent first.
    pub recent: Vec<RecentFolder>,
    /// Where separated songs are written. `None` uses [`default_library_folder`].
    pub library_folder: Option<PathBuf>,
    /// Format of the stems of new separations.
    pub stem_format: StemFormat,
    /// Whether a click on a waveform moves the playhead to the nearest grid line.
    pub snap_to_grid: bool,
    /// Whether the chord lane shows the estimated chords.
    pub show_chords: bool,
    /// The metronome's sound, channels and level, the same for every song.
    pub click_sound: ClickSound,
    pub click_channels: ClickChannels,
    /// In dB, independent of the master volume.
    pub click_volume_db: f32,
    /// Whether the count-in lands on a bar line, overlapping the song when playback
    /// starts late in a bar. Off, it is one whole bar before the playhead.
    pub count_in_to_bar: bool,
    /// How vocal stems are transposed: keeping their formants, and with a tonality
    /// limit that leaves the highs closer to the original.
    pub vocal_formants: bool,
    pub vocal_tonality_limit: bool,
    /// Whether the app looks for a newer release when it starts.
    pub check_for_updates: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            output_device: None,
            last_song_folder: None,
            last_stems_folder: None,
            recent: Vec::new(),
            library_folder: None,
            stem_format: StemFormat::default(),
            snap_to_grid: true,
            show_chords: true,
            click_sound: ClickSound::default(),
            click_channels: ClickChannels::default(),
            click_volume_db: -6.0,
            count_in_to_bar: true,
            vocal_formants: true,
            vocal_tonality_limit: true,
            check_for_updates: true,
        }
    }
}

/// The sound of the metronome click.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ClickSound {
    #[default]
    Click,
    Metronome,
    Wood,
}

/// Which output channels the click plays on.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ClickChannels {
    Left,
    #[default]
    Both,
    Right,
}

/// The format separated stems are written in.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StemFormat {
    /// Ogg Vorbis at about 256 kbps: about 4× smaller than FLAC and transparent.
    #[default]
    Lossy,
    /// 24-bit FLAC, for taking the stems into any DAW.
    Lossless,
}

impl StemFormat {
    pub fn extension(self) -> &'static str {
        match self {
            Self::Lossy => "ogg",
            Self::Lossless => "flac",
        }
    }
}

/// The default folder for separated songs: `Watershed` in the user's music folder,
/// or in the home folder if there is none.
pub fn default_library_folder() -> Option<PathBuf> {
    let dirs = UserDirs::new()?;
    let parent = dirs.audio_dir().unwrap_or(dirs.home_dir());
    Some(parent.join("Watershed"))
}

/// How many recent folders are kept. The list has a search field, so it can be long;
/// past a few hundred, rewriting the config and checking every folder would start to show.
pub const MAX_RECENT: usize = 100;

/// A song folder in the recent list.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RecentFolder {
    pub path: PathBuf,
    /// When it was last opened, in seconds since the Unix epoch.
    pub opened: u64,
}

impl Config {
    /// Where separated songs are written.
    pub fn library_folder(&self) -> Option<PathBuf> {
        self.library_folder.clone().or_else(default_library_folder)
    }

    /// Puts `folder` first in the recent list, opened at `now` (Unix seconds).
    pub fn remember(&mut self, folder: &Path, now: u64) {
        self.forget(folder);
        self.recent.insert(
            0,
            RecentFolder {
                path: folder.to_owned(),
                opened: now,
            },
        );
        self.recent.truncate(MAX_RECENT);
    }

    /// Removes `folder` from the recent list.
    pub fn forget(&mut self, folder: &Path) {
        self.recent.retain(|r| r.path != folder);
    }

    pub fn path() -> Option<PathBuf> {
        project_dirs().map(|d| d.config_dir().join("config.toml"))
    }

    /// Loads the config, falling back to defaults if it is missing or invalid.
    pub fn load() -> Self {
        Self::path()
            .map(|p| Self::load_from(&p))
            .unwrap_or_default()
    }

    pub fn load_from(path: &Path) -> Self {
        match std::fs::read_to_string(path) {
            Ok(text) => toml::from_str(&text).unwrap_or_else(|e| {
                log::warn!("ignoring invalid config {}: {e}", path.display());
                Self::default()
            }),
            Err(_) => Self::default(),
        }
    }

    /// Saves the config. Failures are logged, not fatal.
    pub fn save(&self) {
        if let Some(path) = Self::path()
            && let Err(e) = self.save_to(&path)
        {
            log::warn!("cannot save config to {}: {e}", path.display());
        }
    }

    pub fn save_to(&self, path: &Path) -> std::io::Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let text = toml::to_string_pretty(self).map_err(std::io::Error::other)?;
        std::fs::write(path, text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        let dir = std::env::temp_dir().join(format!("watershed-test-{}", std::process::id()));
        let path = dir.join("config.toml");
        let config = Config {
            output_device: Some("wasapi:{0.0.0.00000000}.{abc}".into()),
            last_song_folder: Some(PathBuf::from("mp3s")),
            last_stems_folder: Some(PathBuf::from("songs/one")),
            recent: vec![RecentFolder {
                path: PathBuf::from("songs/one"),
                opened: 1_700_000_000,
            }],
            library_folder: Some(PathBuf::from("library")),
            stem_format: StemFormat::Lossless,
            snap_to_grid: false,
            show_chords: false,
            click_sound: ClickSound::Wood,
            click_channels: ClickChannels::Left,
            click_volume_db: -12.5,
            count_in_to_bar: false,
            vocal_formants: false,
            vocal_tonality_limit: false,
            check_for_updates: false,
        };
        config.save_to(&path).unwrap();
        assert_eq!(Config::load_from(&path), config);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn recent_list_keeps_the_latest_first_without_duplicates() {
        let mut config = Config::default();
        for i in 0..MAX_RECENT + 5 {
            config.remember(Path::new(&format!("song{i}")), i as u64);
        }
        let latest = PathBuf::from(format!("song{}", MAX_RECENT + 4));
        assert_eq!(config.recent.len(), MAX_RECENT);
        assert_eq!(config.recent[0].path, latest);

        config.remember(Path::new("song10"), 100);
        assert_eq!(config.recent.len(), MAX_RECENT);
        assert_eq!(config.recent[0].path, Path::new("song10"));
        assert_eq!(config.recent[0].opened, 100);
        assert_eq!(
            config
                .recent
                .iter()
                .filter(|r| r.path == Path::new("song10"))
                .count(),
            1
        );

        config.forget(Path::new("song10"));
        assert_eq!(config.recent[0].path, latest);
    }

    #[test]
    fn missing_or_invalid_is_default() {
        assert_eq!(
            Config::load_from(Path::new("does/not/exist.toml")),
            Config::default()
        );
        let dir = std::env::temp_dir().join(format!("watershed-test-bad-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.toml");
        std::fs::write(&path, "output_device = [").unwrap();
        assert_eq!(Config::load_from(&path), Config::default());
        // A config from before snapping keeps it on.
        std::fs::write(&path, "stem_format = \"lossless\"\n").unwrap();
        assert!(Config::load_from(&path).snap_to_grid);
        std::fs::remove_dir_all(dir).unwrap();
    }
}
