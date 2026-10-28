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
    /// Gain applied to every stem when it was written, so that integer formats like
    /// FLAC do not clip the hot ones (1.0: none). Loading undoes it.
    pub stem_gain: f32,
    /// Tempo of the song at its original speed, set by the user; `None` uses the
    /// estimate made when the song is loaded.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bpm: Option<f64>,
    /// Whether the beat grid (and the BPM) is on.
    pub grid: bool,
    /// Time signature: beats per bar, and the note value of a beat.
    pub beats_per_bar: u32,
    pub beat_unit: u32,
    /// Where bar 1 starts, set by the user; `None` uses the estimate.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub grid_start: Option<f64>,
    /// The grid follows the song's own beats instead of a steady BPM.
    pub dynamic_tempo: bool,
    /// Key of the song at its original pitch, set by the user (e.g. `"F#m"`, `"Eb"`);
    /// `None` uses the estimate.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,
    /// Metronome while playing, following the grid.
    pub click: bool,
    /// One bar of clicks before playback starts.
    pub count_in: bool,
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
            stem_gain: 1.0,
            bpm: None,
            grid: true,
            beats_per_bar: 4,
            beat_unit: 4,
            grid_start: None,
            dynamic_tempo: false,
            key: None,
            click: false,
            count_in: false,
        }
    }
}

/// Mixer settings of one stem, matched to its file by name so that adding or
/// removing files from the folder keeps the other settings.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct TrackState {
    pub file: String,
    /// Name given by the user; `None` shows the name from the file.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Linear gain: 1.0 is 0 dB.
    pub gain: f32,
    /// -1.0 is full left, 1.0 full right.
    pub pan: f32,
    pub mute: bool,
    pub solo: bool,
    /// Whether the track skips the transposition; `None` (sessions from before
    /// pitch lock) uses the default for its kind: locked for drums.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pitch_lock: Option<bool>,
    /// Seconds the track is moved by: positive starts it later.
    #[serde(skip_serializing_if = "is_zero")]
    pub offset: f64,
}

fn is_zero(value: &f64) -> bool {
    *value == 0.0
}

impl Default for TrackState {
    fn default() -> Self {
        Self {
            file: String::new(),
            name: None,
            gain: 1.0,
            pan: 0.0,
            mute: false,
            solo: false,
            pitch_lock: None,
            offset: 0.0,
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
        write_hidden(&tmp, text.as_bytes())?;
        std::fs::rename(&tmp, path)
    }

    pub fn track(&self, file: &str) -> Option<&TrackState> {
        self.tracks.iter().find(|t| t.file == file)
    }

    /// The factor to multiply loaded stems by to undo [`Self::stem_gain`].
    pub fn stem_gain_compensation(&self) -> f32 {
        if self.stem_gain.is_finite() && self.stem_gain > 0.0 {
            1.0 / self.stem_gain
        } else {
            1.0
        }
    }
}

/// Writes a new file. On Windows it is marked hidden, since only macOS and Linux hide
/// dot files by their name; the rename that follows keeps the attribute.
fn write_hidden(path: &Path, contents: &[u8]) -> std::io::Result<()> {
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        const FILE_ATTRIBUTE_HIDDEN: u32 = 0x2;
        options.attributes(FILE_ATTRIBUTE_HIDDEN);
    }
    std::io::Write::write_all(&mut options.open(path)?, contents)
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
                    name: Some("Lead vocals".into()),
                    gain: 0.5,
                    pan: -0.3,
                    mute: true,
                    solo: false,
                    pitch_lock: Some(true),
                    offset: -0.25,
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
            stem_gain: 0.5,
            bpm: Some(127.5),
            grid: false,
            beats_per_bar: 7,
            beat_unit: 8,
            grid_start: Some(3.25),
            dynamic_tempo: true,
            key: Some("F#m".into()),
            click: true,
            count_in: true,
            ..Session::default()
        };
        session.save_to(&path).unwrap();
        assert_eq!(Session::load_from(&path), Some(session));
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[cfg(windows)]
    #[test]
    fn hidden_on_windows() {
        use std::os::windows::fs::MetadataExt;
        let dir = temp_dir("session-hidden");
        let path = Session::path_in(&dir);
        // A visible file from before is replaced; saving again over the hidden one works.
        std::fs::write(&path, "").unwrap();
        let session = Session {
            tempo: 0.5,
            ..Session::default()
        };
        session.save_to(&path).unwrap();
        session.save_to(&path).unwrap();
        let attributes = std::fs::metadata(&path).unwrap().file_attributes();
        assert_eq!(attributes & 0x2, 0x2, "not hidden: {attributes:#x}");
        assert_eq!(Session::load_from(&path), Some(session));
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn missing_fields_take_defaults() {
        let session: Session = toml::from_str("tempo = 0.5\n[[tracks]]\nfile = \"bass.wav\"\n")
            .expect("valid session");
        assert_eq!(session.tempo, 0.5);
        assert_eq!(session.semitones, 0.0);
        assert_eq!(session.stem_gain, 1.0);
        assert_eq!(session.bpm, None);
        assert!(session.grid);
        assert_eq!((session.beats_per_bar, session.beat_unit), (4, 4));
        assert!(!session.click && !session.count_in);
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
    fn stem_gain_compensation_ignores_bad_values() {
        let with_gain = |stem_gain| Session {
            stem_gain,
            ..Session::default()
        };
        assert_eq!(with_gain(0.5).stem_gain_compensation(), 2.0);
        assert_eq!(with_gain(0.0).stem_gain_compensation(), 1.0);
        assert_eq!(with_gain(f32::NAN).stem_gain_compensation(), 1.0);
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
