//! Separates a song on a background thread: downloads the model if needed, loads
//! it, and writes the stems into a new folder in the library.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, channel};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use eframe::egui;
use watershed_separation::bsroformer::LoadOptions;
use watershed_separation::{SeparationError, Separator, Stage, catalog, separate_song};
use watershed_storage::{Session, StemFormat};

use crate::stems::song_title;

/// What a separation job is doing.
#[derive(Clone, Debug, PartialEq)]
pub enum Step {
    Downloading {
        done: u64,
        total: u64,
    },
    LoadingModel,
    /// `device` is where the model runs, e.g. `Vulkan0 (NVIDIA GeForce RTX 3050)`.
    Running {
        stage: Stage,
        fraction: f32,
        device: String,
    },
}

impl Step {
    /// Fraction of the current step done, if known.
    pub fn fraction(&self) -> Option<f32> {
        match self {
            Self::Downloading { done, total } if *total > 0 => {
                Some((*done as f64 / *total as f64) as f32)
            }
            // Decoding reports only its start and end.
            Self::Running {
                stage: Stage::Decoding,
                ..
            } => None,
            Self::Running { fraction, .. } => Some(*fraction),
            Self::LoadingModel | Self::Downloading { .. } => None,
        }
    }

    /// Short enough to fit in the progress bar.
    pub fn label(&self) -> String {
        match self {
            Self::Downloading { done, total } => format!(
                "Downloading the model: {:.0} / {:.0} MB",
                *done as f64 / 1e6,
                *total as f64 / 1e6
            ),
            Self::LoadingModel => "Loading the model…".into(),
            Self::Running {
                stage, fraction, ..
            } => match stage {
                Stage::Decoding => "Decoding the song…".into(),
                Stage::Separating => format!("Separating: {:.0} %", fraction * 100.0),
                Stage::Writing => "Writing the stems…".into(),
            },
        }
    }

    /// Where the model runs, once it is loaded.
    pub fn device(&self) -> Option<&str> {
        match self {
            Self::Running { device, .. } => Some(device),
            _ => None,
        }
    }

    /// Whether `other` is the same step (progress aside), to time each step.
    fn same_as(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Running { stage: a, .. }, Self::Running { stage: b, .. }) => a == b,
            _ => std::mem::discriminant(self) == std::mem::discriminant(other),
        }
    }
}

struct Progress {
    step: Step,
    /// When the current step started, to estimate the time left.
    since: Instant,
}

/// A separation in progress.
pub struct Job {
    pub title: String,
    progress: Arc<Mutex<Progress>>,
    cancel: Arc<AtomicBool>,
    result: Receiver<Result<PathBuf, String>>,
}

impl Job {
    pub fn step(&self) -> Step {
        self.progress.lock().unwrap().step.clone()
    }

    /// Time left in the current step, estimated from its progress so far.
    pub fn remaining(&self) -> Option<Duration> {
        let progress = self.progress.lock().unwrap();
        let fraction = progress.step.fraction()?;
        let elapsed = progress.since.elapsed();
        // Too early to tell, or done.
        if fraction < 0.02 || fraction >= 1.0 || elapsed < Duration::from_secs(3) {
            return None;
        }
        Some(elapsed.mul_f32((1.0 - fraction) / fraction))
    }

    pub fn cancel(&self) {
        self.cancel.store(true, Ordering::Relaxed);
    }

    pub fn is_cancelling(&self) -> bool {
        self.cancel.load(Ordering::Relaxed)
    }

    /// The song folder once the job has finished, or why it failed. A cancelled
    /// job ends with `Err` too.
    pub fn poll(&self) -> Option<Result<PathBuf, String>> {
        self.result.try_recv().ok()
    }
}

/// A rough duration for estimates: "45 s", "3 min 05 s".
pub fn format_eta(eta: Duration) -> String {
    let secs = eta.as_secs_f64().round() as u64;
    if secs < 60 {
        format!("{secs} s")
    } else {
        format!("{} min {:02} s", secs / 60, secs % 60)
    }
}

/// Where a song's stems go in the library.
#[derive(Debug, PartialEq, Eq)]
pub enum Target {
    /// Separated before: open this folder instead.
    Existing(PathBuf),
    New(PathBuf),
}

/// Picks the folder for `song` in `library`: `<title>` if it is free or holds an
/// earlier separation of this song, else `<title> (2)`, `<title> (3)`, ...
pub fn target_folder(library: &Path, song: &Path) -> Target {
    let title = folder_name(&song_title(song));
    for n in 1.. {
        let folder = if n == 1 {
            library.join(&title)
        } else {
            library.join(format!("{title} ({n})"))
        };
        if !folder.exists() {
            return Target::New(folder);
        }
        if Session::path_in(&folder).is_file() {
            return Target::Existing(folder);
        }
    }
    unreachable!()
}

/// A title usable as a folder name on every platform.
fn folder_name(title: &str) -> String {
    let name: String = title
        .chars()
        .map(|c| match c {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => '_',
            c if c.is_control() => '_',
            c => c,
        })
        .collect();
    // Windows drops trailing dots and spaces.
    let name = name.trim().trim_end_matches(['.', ' ']);
    if name.is_empty() {
        "Untitled".into()
    } else {
        name.to_owned()
    }
}

/// Starts separating `song` into `folder` with the default model, downloading it
/// into `models_dir` first if needed, and writing the stems in `format`.
pub fn start(
    song: PathBuf,
    folder: PathBuf,
    models_dir: PathBuf,
    format: StemFormat,
    ctx: egui::Context,
) -> Job {
    let title = song_title(&song);
    let progress = Arc::new(Mutex::new(Progress {
        step: Step::LoadingModel,
        since: Instant::now(),
    }));
    let cancel = Arc::new(AtomicBool::new(false));
    let (tx, result) = channel();

    let thread_progress = progress.clone();
    let thread_cancel = cancel.clone();
    std::thread::spawn(move || {
        let set = |step: Step| {
            let mut progress = thread_progress.lock().unwrap();
            if !progress.step.same_as(&step) {
                progress.since = Instant::now();
            }
            progress.step = step;
        };
        let cancelled = || thread_cancel.load(Ordering::Relaxed);
        let outcome = run(&song, &folder, &models_dir, format, &set, &cancelled);
        let _ = tx.send(outcome.map(|()| folder));
        ctx.request_repaint();
    });

    Job {
        title,
        progress,
        cancel,
        result,
    }
}

fn run(
    song: &Path,
    folder: &Path,
    models_dir: &Path,
    format: StemFormat,
    set: &(dyn Fn(Step) + Sync),
    cancel: &(dyn Fn() -> bool + Sync),
) -> Result<(), String> {
    let model = catalog::default_model();
    if !model.is_installed(models_dir) {
        set(Step::Downloading {
            done: 0,
            total: model.size,
        });
    }
    model
        .install(
            models_dir,
            &|done, total| set(Step::Downloading { done, total }),
            cancel,
        )
        .map_err(|e| format!("Cannot download the model: {e}"))?;

    set(Step::LoadingModel);
    let mut separator = model
        .load(models_dir, &LoadOptions::default())
        .map_err(|e| format!("Cannot load the model: {e}"))?;
    let device = separator.device();
    log::info!("separating {} on {device}", song.display());
    let started = Instant::now();
    separate_song(
        &mut separator,
        song,
        folder,
        format,
        &|stage, fraction| {
            set(Step::Running {
                stage,
                fraction,
                device: device.clone(),
            })
        },
        cancel,
    )
    .map_err(|e| match e {
        SeparationError::Cancelled => "Separation cancelled.".to_owned(),
        e => format!("Cannot separate {}: {e}", song.display()),
    })?;
    log::info!("separated in {:.1} s", started.elapsed().as_secs_f64());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn folder_names_are_safe() {
        assert_eq!(
            folder_name("AC/DC: Back in Black?"),
            "AC_DC_ Back in Black_"
        );
        assert_eq!(folder_name("Song. "), "Song");
        assert_eq!(folder_name("..."), "Untitled");
    }

    #[test]
    fn target_reuses_a_separated_song_and_avoids_other_folders() {
        let library =
            std::env::temp_dir().join(format!("watershed-library-{}", std::process::id()));
        std::fs::create_dir_all(&library).unwrap();
        let song = Path::new("downloads/My Song.mp3");

        assert_eq!(
            target_folder(&library, song),
            Target::New(library.join("My Song"))
        );
        // An unrelated folder with the same name is left alone.
        std::fs::create_dir_all(library.join("My Song")).unwrap();
        assert_eq!(
            target_folder(&library, song),
            Target::New(library.join("My Song (2)"))
        );
        // A folder with a session is an earlier separation.
        std::fs::create_dir_all(library.join("My Song (2)")).unwrap();
        std::fs::write(Session::path_in(&library.join("My Song (2)")), "").unwrap();
        assert_eq!(
            target_folder(&library, song),
            Target::Existing(library.join("My Song (2)"))
        );
        std::fs::remove_dir_all(&library).unwrap();
    }

    #[test]
    fn formats_eta() {
        assert_eq!(format_eta(Duration::from_millis(44_600)), "45 s");
        assert_eq!(format_eta(Duration::from_secs(185)), "3 min 05 s");
    }

    #[test]
    fn steps_have_labels_and_fractions() {
        let download = Step::Downloading {
            done: 47_000_000,
            total: 188_000_000,
        };
        assert_eq!(download.label(), "Downloading the model: 47 / 188 MB");
        assert_eq!(download.fraction(), Some(0.25));
        assert_eq!(Step::LoadingModel.fraction(), None);
        let separating = Step::Running {
            stage: Stage::Separating,
            fraction: 0.5,
            device: "Vulkan0 (GPU)".into(),
        };
        assert_eq!(separating.label(), "Separating: 50 %");
        assert_eq!(separating.device(), Some("Vulkan0 (GPU)"));
        let decoding = Step::Running {
            stage: Stage::Decoding,
            fraction: 1.0,
            device: String::new(),
        };
        assert_eq!(decoding.fraction(), None);
        assert!(separating.same_as(&Step::Running {
            stage: Stage::Separating,
            fraction: 0.9,
            device: "Vulkan0 (GPU)".into(),
        }));
        assert!(!separating.same_as(&Step::LoadingModel));
    }
}
