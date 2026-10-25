//! Source separation backends behind a common Separator trait.

use std::path::PathBuf;

use watershed_dsp::{DecodeError, FlacError, ResampleError, StereoBuffer, VorbisWriteError};

pub mod bsroformer;
pub mod catalog;
mod download;
mod gguf;
mod song;

pub use catalog::ModelEntry;
pub use download::{DownloadError, sha256_file};
pub use gguf::GgufSeparator;
pub use song::{SeparatedSong, Stage, separate_song};

/// Splits a stereo mix into stems. Implemented by each backend, so the rest of the
/// app does not care which one runs.
pub trait Separator: Send {
    /// The sample rate the input must have; stems come out at the same rate.
    fn sample_rate(&self) -> u32;

    /// Stem names in output order, lower case (`vocals`, `drums`, ...). They become
    /// the stem file names.
    fn stem_names(&self) -> &[String];

    /// Where it runs, for the UI, e.g. `Vulkan0 (NVIDIA GeForce RTX 3050)`.
    fn device(&self) -> String;

    /// Returns one buffer per stem, in [`Self::stem_names`] order, each as long as
    /// `input`.
    ///
    /// `progress` gets the fraction done (0..1), possibly from another thread.
    /// `cancel` is polled while it runs; returning `true` stops with
    /// [`SeparationError::Cancelled`].
    fn separate(
        &mut self,
        input: &StereoBuffer,
        progress: &(dyn Fn(f32) + Sync),
        cancel: &(dyn Fn() -> bool + Sync),
    ) -> Result<Vec<StereoBuffer>, SeparationError>;
}

#[derive(Debug, thiserror::Error)]
pub enum SeparationError {
    #[error("the model has {model} stems but {names} stem names were given")]
    StemCount { model: usize, names: usize },
    #[error("input is at {got} Hz but the model needs {expected} Hz")]
    SampleRate { expected: u32, got: u32 },
    #[error(transparent)]
    Model(bsroformer::BsrError),
    #[error("cannot decode the song: {0}")]
    Decode(#[from] DecodeError),
    #[error(transparent)]
    Resample(#[from] ResampleError),
    #[error("cannot write a stem: {0}")]
    Write(#[from] FlacError),
    #[error("cannot write a stem: {0}")]
    WriteVorbis(#[from] VorbisWriteError),
    #[error("{}: {source}", path.display())]
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("the output folder already exists: {}", .0.display())]
    OutputExists(PathBuf),
    #[error("separation cancelled")]
    Cancelled,
}

impl From<bsroformer::BsrError> for SeparationError {
    fn from(e: bsroformer::BsrError) -> Self {
        match e {
            bsroformer::BsrError::Cancelled => Self::Cancelled,
            e => Self::Model(e),
        }
    }
}
