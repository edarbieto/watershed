//! Per-song session: `.watershed.toml` in the song folder, next to the stems.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// File name of the session, inside the song folder.
pub const SESSION_FILE: &str = ".watershed.toml";

/// Version written to new files, so later formats can migrate old ones.
const VERSION: u32 = 1;

/// Everything restored when a song folder is opened again. Times are in seconds.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Session {
    pub version: u32,
    /// Playback position when the session was last saved.
    pub position: f64,
    /// Song frames per output frame: 1.0 is the original speed.
    pub tempo: f32,
    pub semitones: f32,
    /// Current loop ends and whether playback loops between them.
    pub loop_a: Option<f64>,
    pub loop_b: Option<f64>,
    pub looping: bool,
    pub tracks: Vec<TrackState>,
    pub loops: Vec<SavedLoop>,
    /// In time order.
    pub markers: Vec<Marker>,
}

impl Default for Session {
    fn default() -> Self {
        Self {
            version: VERSION,
            position: 0.0,
            tempo: 1.0,
            semitones: 0.0,
            loop_a: None,
            loop_b: None,
            looping: false,
            tracks: Vec::new(),
            loops: Vec::new(),
            markers: Vec::new(),
        }
    }
}

/// Mixer settings of one stem, matched to its file by name so that adding or
/// removing files from the folder keeps the other settings.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct TrackState {
    pub file: String,
    /// Linear gain: 1.0 is 0 dB.
    pub gain: f32,
    /// -1.0 is full left, 1.0 full right.
    pub pan: f32,
    pub mute: bool,
    pub solo: bool,
}

impl Default for TrackState {
    fn default() -> Self {
        Self {
            file: String::new(),
            gain: 1.0,
            pan: 0.0,
            mute: false,
            solo: false,
        }
    }
}

/// A loop the user saved, to come back to later.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SavedLoop {
    pub name: String,
    pub start: f64,
    pub end: f64,
}

/// A named point in the song. A section runs from it to the next marker.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Marker {
    pub name: String,
    pub time: f64,
}

impl Session {
    /// Path of the session file for a song folder.
    pub fn path_in(folder: &Path) -> PathBuf {
        folder.join(SESSION_FILE)
    }

    /// Loads a session. `None` if the file is missing or invalid (an invalid file is
    /// logged and later overwritten).
    pub fn load_from(path: &Path) -> Option<Self> {
        let text = std::fs::read_to_string(path).ok()?;
        toml::from_str(&text)
            .inspect_err(|e| log::warn!("ignoring invalid session {}: {e}", path.display()))
            .ok()
    }

    /// Saves the session through a temporary file and a rename, so a crash while
    /// writing never leaves a half-written file.
    pub fn save_to(&self, path: &Path) -> std::io::Result<()> {
        let text = toml::to_string_pretty(self).map_err(std::io::Error::other)?;
        let tmp = path.with_extension("toml.tmp");
        std::fs::write(&tmp, text)?;
        std::fs::rename(&tmp, path)
    }

    pub fn track(&self, file: &str) -> Option<&TrackState> {
        self.tracks.iter().find(|t| t.file == file)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("watershed-{name}-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn round_trip() {
        let dir = temp_dir("session");
        let path = Session::path_in(&dir);
        let session = Session {
            position: 83.25,
            tempo: 0.75,
            semitones: -2.0,
            loop_a: Some(10.0),
            loop_b: Some(20.5),
            looping: true,
            tracks: vec![
                TrackState {
                    file: "vocals.flac".into(),
                    gain: 0.5,
                    pan: -0.3,
                    mute: true,
                    solo: false,
                },
                TrackState {
                    file: "drums.flac".into(),
                    ..TrackState::default()
                },
            ],
            loops: vec![SavedLoop {
                name: "Chorus".into(),
                start: 60.0,
                end: 75.5,
            }],
            markers: vec![Marker {
                name: "Verse".into(),
                time: 12.5,
            }],
            ..Session::default()
        };
        session.save_to(&path).unwrap();
        assert_eq!(Session::load_from(&path), Some(session));
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn missing_fields_take_defaults() {
        let session: Session = toml::from_str("tempo = 0.5\n[[tracks]]\nfile = \"bass.wav\"\n")
            .expect("valid session");
        assert_eq!(session.tempo, 0.5);
        assert_eq!(session.semitones, 0.0);
        assert_eq!(
            session.track("bass.wav"),
            Some(&TrackState {
                file: "bass.wav".into(),
                ..TrackState::default()
            })
        );
        assert_eq!(session.track("drums.wav"), None);
    }

    #[test]
    fn missing_or_invalid_is_none() {
        assert_eq!(Session::load_from(Path::new("does/not/exist.toml")), None);
        let dir = temp_dir("session-bad");
        let path = Session::path_in(&dir);
        std::fs::write(&path, "tempo = [").unwrap();
        assert_eq!(Session::load_from(&path), None);
        std::fs::remove_dir_all(dir).unwrap();
    }
}
