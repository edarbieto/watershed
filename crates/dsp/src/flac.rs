use std::path::Path;

use flac_codec::encode::{FlacSampleWriter, Options};

use crate::StereoBuffer;

const BITS_PER_SAMPLE: u32 = 24;
const FULL_SCALE: f32 = ((1 << (BITS_PER_SAMPLE - 1)) - 1) as f32;

#[derive(Debug, thiserror::Error)]
#[error("cannot write {}: {source}", path.display())]
pub struct FlacError {
    path: std::path::PathBuf,
    source: flac_codec::Error,
}

/// Writes stereo audio as a 24-bit FLAC file, multiplied by `gain` first.
///
/// Samples beyond full scale after the gain are clipped, so callers pick a gain
/// that keeps the peak under 1.0 (see [`gain_to_fit`]).
pub fn write_flac(path: &Path, audio: &StereoBuffer, gain: f32) -> Result<(), FlacError> {
    // Converted in chunks so a whole song never sits in memory twice.
    const CHUNK: usize = 1 << 16;
    let write = || {
        let mut writer = FlacSampleWriter::create(
            path,
            Options::default(),
            audio.sample_rate,
            BITS_PER_SAMPLE,
            2,
            Some(audio.samples.len() as u64),
        )?;
        let mut chunk = Vec::with_capacity(CHUNK);
        for samples in audio.samples.chunks(CHUNK) {
            chunk.clear();
            chunk.extend(
                samples
                    .iter()
                    .map(|&s| ((s * gain).clamp(-1.0, 1.0) * FULL_SCALE).round() as i32),
            );
            writer.write(&chunk)?;
        }
        writer.finalize()
    };
    write().map_err(|source| FlacError {
        path: path.to_path_buf(),
        source,
    })
}

/// The gain that brings the loudest sample of all `buffers` just under full scale,
/// or 1.0 if none exceeds it. Applying one gain to all stems of a song keeps
/// their balance.
pub fn gain_to_fit<'a>(buffers: impl IntoIterator<Item = &'a StereoBuffer>) -> f32 {
    const TARGET: f32 = 0.999;
    let peak = buffers
        .into_iter()
        .flat_map(|b| &b.samples)
        .fold(0.0f32, |peak, s| peak.max(s.abs()));
    if peak <= TARGET { 1.0 } else { TARGET / peak }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_through_the_decoder() {
        let samples: Vec<f32> = (0..44_100)
            .flat_map(|i| {
                let s = (i as f32 * 440.0 * std::f32::consts::TAU / 44_100.0).sin();
                [s * 0.5, s * -0.25]
            })
            .collect();
        let audio = StereoBuffer::new(44_100, samples);
        let path = std::env::temp_dir().join(format!("watershed-flac-{}.flac", std::process::id()));
        write_flac(&path, &audio, 2.0).unwrap();
        let decoded = crate::decode_file(&path);
        std::fs::remove_file(&path).ok();
        let decoded = decoded.unwrap();

        assert_eq!(decoded.sample_rate, 44_100);
        assert_eq!(decoded.samples.len(), audio.samples.len());
        let max_error = decoded
            .samples
            .iter()
            .zip(&audio.samples)
            .map(|(d, s)| (d - s * 2.0).abs())
            .fold(0.0f32, f32::max);
        assert!(max_error < 1e-6, "max error {max_error}");
    }

    /// flacenc, used before, only had 16-bit Rice codes: loud 24-bit audio came out
    /// several times larger than uncompressed.
    #[test]
    fn loud_noise_is_not_larger_than_raw_audio() {
        let mut state = 1u32;
        let samples: Vec<f32> = (0..2 * 44_100)
            .map(|_| {
                state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                (state as f32 / u32::MAX as f32) * 2.0 - 1.0
            })
            .collect();
        let audio = StereoBuffer::new(44_100, samples);
        let path =
            std::env::temp_dir().join(format!("watershed-noise-{}.flac", std::process::id()));
        write_flac(&path, &audio, 0.999).unwrap();
        let size = std::fs::metadata(&path).unwrap().len();
        std::fs::remove_file(&path).ok();
        let raw = audio.samples.len() as u64 * 3;
        // Verbatim frames plus headers; flacenc wrote several times this.
        assert!(size < raw + raw / 10, "{size} bytes for {raw} raw");
    }

    #[test]
    fn gain_to_fit_scales_only_hot_audio() {
        let quiet = StereoBuffer::new(44_100, vec![0.5, -0.9]);
        let hot = StereoBuffer::new(44_100, vec![0.1, -1.998]);
        assert_eq!(gain_to_fit([&quiet]), 1.0);
        assert_eq!(gain_to_fit([&quiet, &hot]), 0.5);
        assert_eq!(gain_to_fit([]), 1.0);
    }
}
