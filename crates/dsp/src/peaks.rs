/// Min/max summary of a waveform, used to draw it at any zoom level without
/// touching the full sample data.
#[derive(Clone, Debug, Default)]
pub struct Peaks {
    /// Frames summarized by each bucket.
    pub frames_per_bucket: usize,
    /// Total frames in the source audio.
    pub total_frames: usize,
    pub min: Vec<f32>,
    pub max: Vec<f32>,
}

impl Peaks {
    pub const DEFAULT_FRAMES_PER_BUCKET: usize = 256;

    /// Summarizes one channel (0 left, 1 right) of interleaved stereo `samples`.
    pub fn compute(samples: &[f32], channel: usize, frames_per_bucket: usize) -> Self {
        let frames_per_bucket = frames_per_bucket.max(1);
        let (min, max) = samples
            .chunks(frames_per_bucket * 2)
            .map(|chunk| {
                chunk
                    .iter()
                    .skip(channel)
                    .step_by(2)
                    .fold((0.0f32, 0.0f32), |(lo, hi), &s| (lo.min(s), hi.max(s)))
            })
            .unzip();
        Self {
            frames_per_bucket,
            total_frames: samples.len() / 2,
            min,
            max,
        }
    }

    /// Left and right peaks of interleaved stereo `samples`.
    pub fn stereo(samples: &[f32], frames_per_bucket: usize) -> [Self; 2] {
        [0, 1].map(|channel| Self::compute(samples, channel, frames_per_bucket))
    }

    /// True if every sample is below about -80 dBFS.
    pub fn is_silent(&self) -> bool {
        self.max.iter().chain(&self.min).all(|s| s.abs() < 1e-4)
    }

    /// Min and max over the frame range `[start, end)`; silence past the end.
    pub fn range(&self, start: usize, end: usize) -> (f32, f32) {
        if self.min.is_empty() || end <= start || start >= self.total_frames {
            return (0.0, 0.0);
        }
        let first = (start / self.frames_per_bucket).min(self.min.len() - 1);
        let last = end
            .div_ceil(self.frames_per_bucket)
            .clamp(first + 1, self.min.len());
        let lo = self.min[first..last].iter().copied().fold(0.0, f32::min);
        let hi = self.max[first..last].iter().copied().fold(0.0, f32::max);
        (lo, hi)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn buckets_track_extremes() {
        // 8 frames: a spike of +0.9 on the left in frame 1, -0.5 on the right in frame 6.
        let mut samples = vec![0.0; 16];
        samples[2] = 0.9;
        samples[13] = -0.5;
        let [left, right] = Peaks::stereo(&samples, 4);

        assert_eq!(left.min.len(), 2);
        assert_eq!((left.min[0], left.max[0]), (0.0, 0.9));
        assert_eq!(left.range(0, 8), (0.0, 0.9));
        assert_eq!((right.min[1], right.max[1]), (-0.5, 0.0));
        assert_eq!(right.range(0, 8), (-0.5, 0.0));
        assert_eq!(right.range(5, 6), (-0.5, 0.0));
        assert!(!right.is_silent());
        assert!(Peaks::compute(&samples[..8], 1, 4).is_silent());
    }

    #[test]
    fn silent_past_the_end() {
        // A track shorter than the song: its last bucket must not repeat.
        let peaks = Peaks::compute(&[0.5; 8], 0, 4);
        assert_eq!(peaks.range(2, 3), (0.0, 0.5));
        assert_eq!(peaks.range(4, 8), (0.0, 0.0));
    }

    #[test]
    fn empty_audio_is_flat() {
        let peaks = Peaks::compute(&[], 0, 256);
        assert_eq!(peaks.range(0, 100), (0.0, 0.0));
    }
}
