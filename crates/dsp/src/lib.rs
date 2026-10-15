//! Audio decoding, resampling, waveform peaks and (later) STFT/iSTFT and chunking.

mod decode;
mod peaks;
mod resample;

pub use decode::{DecodeError, decode_file};
pub use peaks::Peaks;
pub use resample::{ResampleError, resample};

/// Stereo audio with interleaved `[left, right, left, right, ...]` samples.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct StereoBuffer {
    pub sample_rate: u32,
    pub samples: Vec<f32>,
}

impl StereoBuffer {
    pub fn new(sample_rate: u32, samples: Vec<f32>) -> Self {
        debug_assert!(
            samples.len().is_multiple_of(2),
            "stereo buffer needs an even sample count"
        );
        Self {
            sample_rate,
            samples,
        }
    }

    /// Number of frames (one left + one right sample each).
    pub fn frames(&self) -> usize {
        self.samples.len() / 2
    }

    pub fn duration_secs(&self) -> f64 {
        self.frames() as f64 / f64::from(self.sample_rate)
    }
}
