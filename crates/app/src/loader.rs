//! Loads a folder of stems on background threads.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{Receiver, channel};

use eframe::egui::{self, Color32};
use watershed_audio::{Samples, Song};
use watershed_dsp::Peaks;
use watershed_storage::Session;

use crate::stems::{StemKind, default_names, song_title, track_colors};

pub const AUDIO_EXTENSIONS: &[&str] = &["mp3", "flac", "wav", "ogg", "m4a", "aac", "aiff"];

/// A song ready to play: the audio plus what the UI needs to draw it.
pub struct LoadedSong {
    pub title: String,
    pub files: Vec<PathBuf>,
    pub song: Arc<Song>,
    pub colors: Vec<Color32>,
    /// Names shown for the tracks: the user's, or else the default ones.
    pub names: Vec<String>,
    /// Names from the file names, used when the user clears a name.
    pub default_names: Vec<String>,
    /// Left and right peaks of each track.
    pub peaks: Vec<[Peaks; 2]>,
    /// The session's [`Session::stem_gain`], already undone in `song`; kept so the
    /// session is saved with it.
    pub stem_gain: f32,
}

/// A load in progress.
pub struct Loading {
    pub title: String,
    pub total: usize,
    done: Arc<AtomicUsize>,
    result: Receiver<Result<LoadedSong, String>>,
}

impl Loading {
    pub fn done(&self) -> usize {
        self.done.load(Ordering::Relaxed)
    }

    /// Returns the result once loading has finished.
    pub fn poll(&self) -> Option<Result<LoadedSong, String>> {
        self.result.try_recv().ok()
    }
}

/// Expands dropped or picked paths into a sorted list of audio files. A single
/// folder means "all audio files in it".
pub fn collect_files(paths: &[PathBuf]) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = paths
        .iter()
        .flat_map(|p| {
            if p.is_dir() {
                std::fs::read_dir(p)
                    .map(|entries| entries.flatten().map(|e| e.path()).collect())
                    .unwrap_or_default()
            } else {
                vec![p.clone()]
            }
        })
        .filter(|p| is_audio(p))
        .collect();
    files.sort_by_key(|p| (StemKind::from_path(p), p.clone()));
    files
}

pub fn is_audio(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| AUDIO_EXTENSIONS.contains(&e.to_lowercase().as_str()))
}

/// Starts decoding `files` in parallel, resampled to `sample_rate`.
pub fn start(files: Vec<PathBuf>, sample_rate: u32, ctx: egui::Context) -> Loading {
    let title = files
        .first()
        .and_then(|f| f.parent())
        .map(song_title)
        .unwrap_or_default();
    let total = files.len();
    let done = Arc::new(AtomicUsize::new(0));
    let (tx, result) = channel();

    let thread_title = title.clone();
    let thread_done = done.clone();
    std::thread::spawn(move || {
        let outcome = load(thread_title, files, sample_rate, &thread_done);
        let _ = tx.send(outcome);
        ctx.request_repaint();
    });

    Loading {
        title,
        total,
        done,
        result,
    }
}

fn load(
    title: String,
    files: Vec<PathBuf>,
    sample_rate: u32,
    done: &AtomicUsize,
) -> Result<LoadedSong, String> {
    if files.is_empty() {
        return Err("no audio files found".into());
    }
    let session = files[0]
        .parent()
        .and_then(|folder| Session::load_from(&Session::path_in(folder)))
        .unwrap_or_default();
    let compensation = session.stem_gain_compensation();

    // Each stem is packed and its peaks computed in its own thread, so the
    // full-size f32 copies never all exist at once.
    type Decoded = (Samples, [Peaks; 2]);
    let decoded: Vec<Result<Decoded, String>> = std::thread::scope(|scope| {
        let handles: Vec<_> = files
            .iter()
            .map(|path| {
                scope.spawn(move || {
                    let name = path.file_name().unwrap_or_default().to_string_lossy();
                    let audio =
                        watershed_dsp::decode_file(path).map_err(|e| format!("{name}: {e}"))?;
                    let mut audio = watershed_dsp::resample(audio, sample_rate)
                        .map_err(|e| format!("{name}: {e}"))?;
                    if compensation != 1.0 {
                        audio.samples.iter_mut().for_each(|s| *s *= compensation);
                    }
                    let peaks = Peaks::stereo(&audio.samples, Peaks::DEFAULT_FRAMES_PER_BUCKET);
                    let samples = Samples::from_f32(&audio.samples);
                    done.fetch_add(1, Ordering::Relaxed);
                    Ok((samples, peaks))
                })
            })
            .collect();
        handles
            .into_iter()
            .map(|h| h.join().unwrap_or_else(|_| Err("decoder crashed".into())))
            .collect()
    });

    let decoded = decoded.into_iter().collect::<Result<Vec<_>, _>>()?;
    let (samples, peaks): (Vec<_>, Vec<_>) = decoded.into_iter().unzip();
    let kinds: Vec<StemKind> = files.iter().map(|f| StemKind::from_path(f)).collect();
    let default_names = default_names(&files, &kinds);
    let names: Vec<String> = files
        .iter()
        .zip(&default_names)
        .map(|(file, default)| {
            let file = file.file_name().unwrap_or_default().to_string_lossy();
            session
                .track(&file)
                .and_then(|t| t.name.clone())
                .unwrap_or_else(|| default.clone())
        })
        .collect();
    let tracks = default_names.iter().cloned().zip(samples).collect();

    let song = Arc::new(Song::new(sample_rate, tracks));

    // The instrumental stem is the sum of the others; start it muted when they are present.
    if kinds.len() > 1 {
        for (track, kind) in song.tracks.iter().zip(&kinds) {
            if *kind == StemKind::Instrumental {
                track.params.set_mute(true);
            }
        }
    }

    Ok(LoadedSong {
        title,
        files,
        song,
        colors: track_colors(&kinds),
        names,
        default_names,
        peaks,
        stem_gain: session.stem_gain,
    })
}
