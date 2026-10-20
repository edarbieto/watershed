//! Separates a song file into a folder of FLAC stems that the player opens.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use watershed_storage::Session;

use crate::{SeparationError, Separator};

/// What [`separate_song`] is doing, reported with the fraction of that stage done.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stage {
    /// Decoding the song and resampling it to the model's rate.
    Decoding,
    Separating,
    /// Writing the stem files.
    Writing,
}

/// A song folder written by [`separate_song`].
#[derive(Clone, Debug, PartialEq)]
pub struct SeparatedSong {
    pub folder: PathBuf,
    /// One FLAC file per stem, in the separator's order.
    pub files: Vec<PathBuf>,
    /// The common gain applied to all stems to keep them under full scale, also
    /// stored in the folder's session.
    pub stem_gain: f32,
}

/// Decodes `song`, separates it and writes one 24-bit FLAC per stem
/// (`vocals.flac`, ...) plus a session with the common stem gain into `folder`,
/// which must not exist yet.
///
/// Everything is written to a `.partial` folder next to it first and renamed at
/// the end, so a cancelled or failed run never leaves a half-written song folder.
pub fn separate_song(
    separator: &mut dyn Separator,
    song: &Path,
    folder: &Path,
    progress: &(dyn Fn(Stage, f32) + Sync),
    cancel: &(dyn Fn() -> bool + Sync),
) -> Result<SeparatedSong, SeparationError> {
    if folder.exists() {
        return Err(SeparationError::OutputExists(folder.to_path_buf()));
    }
    let partial = partial_folder(folder);
    let result = write_partial(separator, song, &partial, progress, cancel).and_then(|gain| {
        std::fs::rename(&partial, folder).map_err(|source| SeparationError::Io {
            path: folder.to_path_buf(),
            source,
        })?;
        Ok(gain)
    });
    let stem_gain = match result {
        Ok(gain) => gain,
        Err(e) => {
            if let Err(remove) = std::fs::remove_dir_all(&partial)
                && remove.kind() != std::io::ErrorKind::NotFound
            {
                log::warn!("cannot remove {}: {remove}", partial.display());
            }
            return Err(e);
        }
    };
    Ok(SeparatedSong {
        folder: folder.to_path_buf(),
        files: separator
            .stem_names()
            .iter()
            .map(|name| folder.join(stem_file_name(name)))
            .collect(),
        stem_gain,
    })
}

/// Does the work of [`separate_song`] into `partial` and returns the stem gain.
fn write_partial(
    separator: &mut dyn Separator,
    song: &Path,
    partial: &Path,
    progress: &(dyn Fn(Stage, f32) + Sync),
    cancel: &(dyn Fn() -> bool + Sync),
) -> Result<f32, SeparationError> {
    let io_error = |path: &Path| {
        let path = path.to_path_buf();
        move |source| SeparationError::Io { path, source }
    };
    // Left over from a run that crashed.
    if partial.exists() {
        std::fs::remove_dir_all(partial).map_err(io_error(partial))?;
    }
    std::fs::create_dir_all(partial).map_err(io_error(partial))?;

    progress(Stage::Decoding, 0.0);
    let audio = watershed_dsp::decode_file(song)?;
    let audio = watershed_dsp::resample(audio, separator.sample_rate())?;
    progress(Stage::Decoding, 1.0);
    if cancel() {
        return Err(SeparationError::Cancelled);
    }

    // Models report their first progress only after a whole chunk.
    progress(Stage::Separating, 0.0);
    let stems = separator.separate(&audio, &|f| progress(Stage::Separating, f), cancel)?;
    drop(audio);
    let names = separator.stem_names();
    if stems.len() != names.len() {
        return Err(SeparationError::StemCount {
            model: stems.len(),
            names: names.len(),
        });
    }

    let gain = watershed_dsp::gain_to_fit(&stems);
    progress(Stage::Writing, 0.0);
    // One thread per stem: the encoder is single-threaded.
    let written = AtomicUsize::new(0);
    std::thread::scope(|scope| {
        let handles: Vec<_> = stems
            .iter()
            .zip(names)
            .map(|(stem, name)| {
                let written = &written;
                scope.spawn(move || {
                    if cancel() {
                        return Err(SeparationError::Cancelled);
                    }
                    watershed_dsp::write_flac(&partial.join(stem_file_name(name)), stem, gain)?;
                    let done = written.fetch_add(1, Ordering::Relaxed) + 1;
                    progress(Stage::Writing, done as f32 / names.len() as f32);
                    Ok(())
                })
            })
            .collect();
        handles
            .into_iter()
            .try_for_each(|h| h.join().expect("FLAC writer panicked"))
    })?;
    // The threads may report out of order.
    progress(Stage::Writing, 1.0);

    let session_path = Session::path_in(partial);
    Session {
        stem_gain: gain,
        ..Session::default()
    }
    .save_to(&session_path)
    .map_err(io_error(&session_path))?;
    Ok(gain)
}

fn stem_file_name(name: &str) -> String {
    format!("{name}.flac")
}

fn partial_folder(folder: &Path) -> PathBuf {
    let mut name = folder.file_name().unwrap_or_default().to_os_string();
    name.push(".partial");
    folder.with_file_name(name)
}

#[cfg(test)]
mod tests {
    use watershed_dsp::StereoBuffer;

    use super::*;

    /// Splits the input into left-only and right-only stems, at twice the level so
    /// they exceed full scale.
    struct FakeSeparator {
        names: Vec<String>,
    }

    impl FakeSeparator {
        fn new() -> Self {
            Self {
                names: vec!["left".into(), "right".into()],
            }
        }
    }

    impl Separator for FakeSeparator {
        fn sample_rate(&self) -> u32 {
            44_100
        }

        fn stem_names(&self) -> &[String] {
            &self.names
        }

        fn device(&self) -> String {
            "test".into()
        }

        fn separate(
            &mut self,
            input: &StereoBuffer,
            progress: &(dyn Fn(f32) + Sync),
            cancel: &(dyn Fn() -> bool + Sync),
        ) -> Result<Vec<StereoBuffer>, SeparationError> {
            if cancel() {
                return Err(SeparationError::Cancelled);
            }
            progress(1.0);
            let keep = |channel: usize| {
                let samples = input
                    .samples
                    .iter()
                    .enumerate()
                    .map(|(i, s)| if i % 2 == channel { s * 2.0 } else { 0.0 })
                    .collect();
                StereoBuffer::new(input.sample_rate, samples)
            };
            Ok(vec![keep(0), keep(1)])
        }
    }

    struct TestDir(PathBuf);

    impl TestDir {
        fn new(name: &str) -> Self {
            let dir = std::env::temp_dir()
                .join(format!("watershed-separate-{name}-{}", std::process::id()));
            std::fs::remove_dir_all(&dir).ok();
            std::fs::create_dir_all(&dir).unwrap();
            Self(dir)
        }

        /// A one-second song at 48 kHz, so it gets resampled, peaking at 0.75.
        fn song(&self) -> PathBuf {
            let samples = (0..48_000)
                .flat_map(|i| {
                    let s = (i as f32 * 440.0 * std::f32::consts::TAU / 48_000.0).sin();
                    [s * 0.75, s * 0.25]
                })
                .collect();
            let path = self.0.join("song.flac");
            watershed_dsp::write_flac(&path, &StereoBuffer::new(48_000, samples), 1.0).unwrap();
            path
        }
    }

    impl Drop for TestDir {
        fn drop(&mut self) {
            std::fs::remove_dir_all(&self.0).ok();
        }
    }

    #[test]
    fn writes_stems_and_session_with_a_common_gain() {
        let dir = TestDir::new("ok");
        let song = dir.song();
        let folder = dir.0.join("Song");
        let stages = std::sync::Mutex::new(Vec::new());
        let separated = separate_song(
            &mut FakeSeparator::new(),
            &song,
            &folder,
            &|stage, f| stages.lock().unwrap().push((stage, f)),
            &|| false,
        )
        .unwrap();

        assert_eq!(
            separated.files,
            [folder.join("left.flac"), folder.join("right.flac")]
        );
        // The left stem peaks near 1.5: the gain brings it back under 1.0.
        assert!(
            (separated.stem_gain - 0.666).abs() < 0.01,
            "{}",
            separated.stem_gain
        );
        let session = Session::load_from(&Session::path_in(&folder)).unwrap();
        assert_eq!(session.stem_gain, separated.stem_gain);

        let left = watershed_dsp::decode_file(&separated.files[0]).unwrap();
        let right = watershed_dsp::decode_file(&separated.files[1]).unwrap();
        assert_eq!(left.sample_rate, 44_100);
        assert_eq!(left.frames(), 44_100);
        let peak = |b: &StereoBuffer| b.samples.iter().fold(0.0f32, |p, s| p.max(s.abs()));
        // Undoing the gain restores the original level and balance (3:1).
        let compensation = session.stem_gain_compensation();
        assert!((peak(&left) * compensation - 1.5).abs() < 0.02);
        assert!((peak(&right) * compensation - 0.5).abs() < 0.02);

        assert!(!partial_folder(&folder).exists());
        let stages = stages.into_inner().unwrap();
        assert_eq!(stages.first(), Some(&(Stage::Decoding, 0.0)));
        assert_eq!(stages.last(), Some(&(Stage::Writing, 1.0)));
    }

    #[test]
    fn cancelling_leaves_no_folder() {
        let dir = TestDir::new("cancel");
        let song = dir.song();
        let folder = dir.0.join("Song");
        // Cancel once the stems are being written.
        let polls = AtomicUsize::new(0);
        let result = separate_song(
            &mut FakeSeparator::new(),
            &song,
            &folder,
            &|_, _| {},
            &|| polls.fetch_add(1, Ordering::Relaxed) >= 3,
        );
        assert!(
            matches!(result, Err(SeparationError::Cancelled)),
            "{result:?}"
        );
        assert!(!folder.exists());
        assert!(!partial_folder(&folder).exists());
    }

    #[test]
    fn never_overwrites_an_existing_folder() {
        let dir = TestDir::new("exists");
        let song = dir.song();
        let result = separate_song(
            &mut FakeSeparator::new(),
            &song,
            &dir.0,
            &|_, _| {},
            &|| false,
        );
        assert!(matches!(result, Err(SeparationError::OutputExists(_))));
        assert!(song.exists());
    }
}
