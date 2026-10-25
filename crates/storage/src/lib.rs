//! Sessions, model cache and app configuration on disk.

use std::path::{Path, PathBuf};

use directories::{ProjectDirs, UserDirs};
use serde::{Deserialize, Serialize};

mod session;

pub use session::{Marker, SESSION_FILE, SavedLoop, Session, TrackState};

fn project_dirs() -> Option<ProjectDirs> {
    ProjectDirs::from("", "", "Watershed")
}

/// Where downloaded separation models live: a `models` folder in the OS cache
/// folder, since they can always be downloaded again.
pub fn models_dir() -> Option<PathBuf> {
    project_dirs().map(|d| d.cache_dir().join("models"))
}

/// User settings, stored as `config.toml` in the OS config folder.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// Id of the last selected output device.
    pub output_device: Option<String>,
    /// Folder the last song was opened from, to start file dialogs there.
    pub last_folder: Option<PathBuf>,
    /// Song folders opened recently, most recent first.
    pub recent: Vec<RecentFolder>,
    /// Where separated songs are written. `None` uses [`default_library_folder`].
    pub library_folder: Option<PathBuf>,
    /// Format of the stems of new separations.
    pub stem_format: StemFormat,
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

/// How many recent folders are kept.
pub const MAX_RECENT: usize = 20;

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
            last_folder: Some(PathBuf::from("songs")),
            recent: vec![RecentFolder {
                path: PathBuf::from("songs/one"),
                opened: 1_700_000_000,
            }],
            library_folder: Some(PathBuf::from("library")),
            stem_format: StemFormat::Lossless,
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
        assert_eq!(config.recent.len(), MAX_RECENT);
        assert_eq!(config.recent[0].path, Path::new("song24"));

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
        assert_eq!(config.recent[0].path, Path::new("song24"));
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
        std::fs::remove_dir_all(dir).unwrap();
    }
}
