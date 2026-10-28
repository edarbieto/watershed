//! Loads a folder of stems on background threads.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{Receiver, channel};

use eframe::egui::{self, Color32};
use watershed_audio::{Samples, Song};
use watershed_dsp::{Chroma, KeyEstimate, OnsetEnvelope, Peaks};
use watershed_storage::Session;

use crate::stems::{StemKind, default_names, song_title, track_colors};

pub const AUDIO_EXTENSIONS: &[&str] = &["mp3", "flac", "wav", "ogg", "m4a", "aac", "aiff"];

/// A song ready to play: the audio plus what the UI needs to draw it.
pub struct LoadedSong {
    pub title: String,
    pub files: Vec<PathBuf>,
    pub song: Arc<Song>,
    pub colors: Vec<Color32>,
    /// Names shown for the tracks: the user's (set from the session by the app),
    /// or else the default ones.
    pub names: Vec<String>,
    /// Names from the file names, used when the user clears a name.
    pub default_names: Vec<String>,
    /// Left and right peaks of each track.
    pub peaks: Vec<[Peaks; 2]>,
    /// The session's [`Session::stem_gain`], already undone in `song`; kept so the
    /// session is saved with it.
    pub stem_gain: f32,
    /// Estimated tempo of the song, if it has a steady enough beat.
    pub bpm: Option<f64>,
    /// Onsets of all stems, kept to place the beat grid at any BPM.
    pub onsets: OnsetEnvelope,
    /// Estimated key, from the pitched stems.
    pub key: Option<KeyEstimate>,
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
    let kinds: Vec<StemKind> = files.iter().map(|f| StemKind::from_path(f)).collect();

    // Each stem is packed and its peaks computed in its own thread, so the
    // full-size f32 copies never all exist at once.
    /// A stem's onsets come with their weight in the song's sum.
    type Decoded = (
        Samples,
        [Peaks; 2],
        Option<(OnsetEnvelope, f32)>,
        Option<Chroma>,
    );
    let decoded: Vec<Result<Decoded, String>> = std::thread::scope(|scope| {
        let handles: Vec<_> = files
            .iter()
            .zip(&kinds)
            .map(|(path, kind)| {
                // The tempo is estimated from all stems together: on test songs the
                // sum was right where single stems, drums included, were sometimes
                // off by half. The instrumental repeats the others.
                let for_tempo = *kind != StemKind::Instrumental || kinds.len() == 1;
                // Singing follows the beat loosely.
                let tempo_weight = if *kind == StemKind::Vocals && kinds.len() > 1 {
                    watershed_dsp::VOCALS_TEMPO_WEIGHT
                } else {
                    1.0
                };
                // The key comes from the pitched stems: drums only blur it.
                let for_key = for_tempo && (*kind != StemKind::Drums || kinds.len() == 1);
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
                    let onsets = for_tempo.then(|| {
                        let onsets = OnsetEnvelope::compute(&audio.samples, audio.sample_rate);
                        (onsets, tempo_weight)
                    });
                    let chroma =
                        for_key.then(|| Chroma::compute(&audio.samples, audio.sample_rate));
                    let samples = Samples::from_f32(&audio.samples);
                    done.fetch_add(1, Ordering::Relaxed);
                    Ok((samples, peaks, onsets, chroma))
                })
            })
            .collect();
        handles
            .into_iter()
            .map(|h| h.join().unwrap_or_else(|_| Err("decoder crashed".into())))
            .collect()
    });

    let decoded = decoded.into_iter().collect::<Result<Vec<_>, _>>()?;
    let mut samples = Vec::with_capacity(decoded.len());
    let mut peaks = Vec::with_capacity(decoded.len());
    let mut onsets = OnsetEnvelope::default();
    let mut chroma = Chroma::default();
    for (track_samples, track_peaks, track_onsets, track_chroma) in decoded {
        samples.push(track_samples);
        peaks.push(track_peaks);
        if let Some((track_onsets, weight)) = track_onsets {
            onsets.add_weighted(&track_onsets, weight);
        }
        if let Some(track_chroma) = track_chroma {
            chroma.add(&track_chroma);
        }
    }
    let default_names = default_names(&files, &kinds);
    let tracks = default_names.iter().cloned().zip(samples).collect();

    let song = Arc::new(Song::new(sample_rate, tracks));

    // Transposed drums sound wrong and gain nothing.
    for (track, kind) in song.tracks.iter().zip(&kinds) {
        track.params.set_pitch_lock(kind.pitch_locked_by_default());
    }
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
        names: default_names.clone(),
        default_names,
        peaks,
        stem_gain: session.stem_gain,
        bpm: onsets.bpm(),
        onsets,
        key: chroma.key(),
    })
}
