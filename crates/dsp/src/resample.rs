use rubato::audioadapter_buffers::direct::InterleavedSlice;
use rubato::{Fft, FixedSync, Resampler};

use crate::StereoBuffer;

#[derive(Debug, thiserror::Error)]
#[error("resampling failed: {0}")]
pub struct ResampleError(String);

/// Converts a whole buffer to `target_rate` with a high-quality FFT resampler.
///
/// Returns the input unchanged when the rates already match.
pub fn resample(input: StereoBuffer, target_rate: u32) -> Result<StereoBuffer, ResampleError> {
    if input.sample_rate == target_rate || input.frames() == 0 {
        return Ok(StereoBuffer::new(target_rate, input.samples));
    }

    let err = |e: &dyn std::fmt::Display| ResampleError(e.to_string());
    let mut resampler = Fft::<f32>::new(
        input.sample_rate as usize,
        target_rate as usize,
        4096,
        2,
        FixedSync::Input,
    )
    .map_err(|e| err(&e))?;

    let frames = input.frames();
    let adapter = InterleavedSlice::new(&input.samples, 2, frames).map_err(|e| err(&e))?;
    let output = resampler
        .process_all(&adapter, frames, None)
        .map_err(|e| err(&e))?;

    Ok(StereoBuffer::new(target_rate, output.take_data()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sine(rate: u32, freq: f32, frames: usize) -> StereoBuffer {
        let samples = (0..frames)
            .flat_map(|i| {
                let v = (std::f32::consts::TAU * freq * i as f32 / rate as f32).sin();
                [v, v]
            })
            .collect();
        StereoBuffer::new(rate, samples)
    }

    #[test]
    fn same_rate_is_passthrough() {
        let input = sine(48_000, 440.0, 1000);
        let output = resample(input.clone(), 48_000).unwrap();
        assert_eq!(output, input);
    }

    #[test]
    fn keeps_duration_and_pitch() {
        let input = sine(44_100, 440.0, 44_100);
        let output = resample(input, 48_000).unwrap();
        assert_eq!(output.sample_rate, 48_000);
        assert!(
            (output.frames() as i64 - 48_000).abs() <= 1,
            "{}",
            output.frames()
        );

        // Count rising zero crossings on the left channel, away from the edges.
        let left: Vec<f32> = output.samples.iter().step_by(2).copied().collect();
        let crossings = left[4800..43_200]
            .windows(2)
            .filter(|w| w[0] < 0.0 && w[1] >= 0.0)
            .count();
        // 0.8 s of a 440 Hz tone.
        assert!((350..=354).contains(&crossings), "{crossings}");
    }
}
