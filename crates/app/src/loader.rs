//! Loads a folder of stems on background threads.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{Receiver, channel};

use eframe::egui;
use watershed_audio::Song;
use watershed_dsp::Peaks;

use crate::stems::{StemKind, song_title};

const AUDIO_EXTENSIONS: &[&str] = &["mp3", "flac", "wav", "ogg", "m4a", "aac", "aiff"];

/// A song ready to play: the audio plus what the UI needs to draw it.
pub struct LoadedSong {
    pub title: String,
    pub files: Vec<PathBuf>,
    pub song: Arc<Song>,
    pub kinds: Vec<StemKind>,
    /// Left and right peaks of each track.
    pub peaks: Vec<[Peaks; 2]>,
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

fn is_audio(path: &Path) -> bool {
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

    let decoded: Vec<Result<Vec<f32>, String>> = std::thread::scope(|scope| {
        let handles: Vec<_> = files
            .iter()
            .map(|path| {
                scope.spawn(move || {
                    let name = path.file_name().unwrap_or_default().to_string_lossy();
                    let audio =
                        watershed_dsp::decode_file(path).map_err(|e| format!("{name}: {e}"))?;
                    let audio = watershed_dsp::resample(audio, sample_rate)
                        .map_err(|e| format!("{name}: {e}"))?;
                    done.fetch_add(1, Ordering::Relaxed);
                    Ok(audio.samples)
                })
            })
            .collect();
        handles
            .into_iter()
            .map(|h| h.join().unwrap_or_else(|_| Err("decoder crashed".into())))
            .collect()
    });

    let kinds: Vec<StemKind> = files.iter().map(|f| StemKind::from_path(f)).collect();
    let tracks = files
        .iter()
        .zip(&kinds)
        .zip(decoded)
        .map(|((path, kind), samples)| {
            let name = kind.label().map(str::to_owned).unwrap_or_else(|| {
                path.file_stem()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned()
            });
            samples.map(|s| (name, s))
        })
        .collect::<Result<Vec<_>, _>>()?;

    let song = Arc::new(Song::new(sample_rate, tracks));
    let peaks = song
        .tracks
        .iter()
        .map(|t| Peaks::stereo(&t.samples, Peaks::DEFAULT_FRAMES_PER_BUCKET))
        .collect();

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
        kinds,
        peaks,
    })
}
