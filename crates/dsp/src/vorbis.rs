use std::fs::File;
use std::io::{BufWriter, Write};
use std::num::{NonZeroU8, NonZeroU32};
use std::path::Path;

use vorbis_rs::{VorbisBitrateManagementStrategy, VorbisEncoderBuilder, VorbisError};

use crate::StereoBuffer;

/// libvorbis quality, from -0.2 to 1.0: 0.8 ("q8") is about 256 kbps for stereo at
/// 44.1 kHz, transparent for nearly everyone.
const QUALITY: f32 = 0.8;
/// Frames per block handed to the encoder; libvorbis suggests about 1024.
const BLOCK: usize = 1024;

#[derive(Debug, thiserror::Error)]
#[error("cannot write {}: {source}", path.display())]
pub struct VorbisWriteError {
    path: std::path::PathBuf,
    source: VorbisError,
}

/// Writes stereo audio as an Ogg Vorbis file.
///
/// Vorbis stores floating point, so samples beyond full scale survive and need no
/// gain, unlike FLAC.
pub fn write_vorbis(path: &Path, audio: &StereoBuffer) -> Result<(), VorbisWriteError> {
    let write = || -> Result<(), VorbisError> {
        let sink = BufWriter::new(File::create(path)?);
        let rate = NonZeroU32::new(audio.sample_rate).ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::InvalidInput, "sample rate is 0")
        })?;
        let channels = NonZeroU8::new(2).unwrap_or(NonZeroU8::MIN);
        // Any serial works for a file with one stream; a fixed one keeps output
        // reproducible.
        let mut encoder = VorbisEncoderBuilder::new_with_serial(rate, channels, sink, 1)
            .bitrate_management_strategy(VorbisBitrateManagementStrategy::QualityVbr {
                target_quality: QUALITY,
            })
            .build()?;
        let mut left = Vec::with_capacity(BLOCK);
        let mut right = Vec::with_capacity(BLOCK);
        for frames in audio.samples.chunks(BLOCK * 2) {
            left.clear();
            right.clear();
            for frame in frames.as_chunks::<2>().0 {
                left.push(frame[0]);
                right.push(frame[1]);
            }
            encoder.encode_audio_block([&left, &right])?;
        }
        encoder.finish()?.flush()?;
        Ok(())
    };
    write().map_err(|source| VorbisWriteError {
        path: path.to_path_buf(),
        source,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_samples_beyond_full_scale_and_the_length() {
        let dir = std::env::temp_dir().join(format!("watershed-vorbis-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("hot.ogg");
        // One second of a 440 Hz sine peaking at 1.5 on the left, 0.5 on the right.
        let samples = (0..44_100)
            .flat_map(|i| {
                let s = (i as f32 * 440.0 * std::f32::consts::TAU / 44_100.0).sin();
                [s * 1.5, s * 0.5]
            })
            .collect();
        write_vorbis(&path, &StereoBuffer::new(44_100, samples)).unwrap();

        let decoded = crate::decode_file(&path).unwrap();
        assert_eq!(decoded.sample_rate, 44_100);
        assert_eq!(decoded.frames(), 44_100);
        let peak = |channel: usize| {
            decoded
                .samples
                .iter()
                .skip(channel)
                .step_by(2)
                .fold(0.0f32, |p, s| p.max(s.abs()))
        };
        assert!((peak(0) - 1.5).abs() < 0.05, "left peak {}", peak(0));
        assert!((peak(1) - 0.5).abs() < 0.05, "right peak {}", peak(1));
        // About 256 kbps: a second is about 32 kB (a pure sine takes less).
        let size = std::fs::metadata(&path).unwrap().len();
        assert!(size < 40_000, "{size} bytes");
        std::fs::remove_dir_all(dir).unwrap();
    }
}
