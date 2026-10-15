use std::fs::File;
use std::path::Path;

use symphonia::core::codecs::audio::AudioDecoderOptions;
use symphonia::core::errors::Error as SymphoniaError;
use symphonia::core::formats::probe::Hint;
use symphonia::core::formats::{FormatOptions, TrackType};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;

use crate::StereoBuffer;

#[derive(Debug, thiserror::Error)]
pub enum DecodeError {
    #[error("cannot open file: {0}")]
    Io(#[from] std::io::Error),
    #[error("unsupported audio file: {0}")]
    Unsupported(String),
    #[error("decoding failed: {0}")]
    Decode(#[from] SymphoniaError),
}

/// Decodes a whole audio file into memory as stereo `f32` at its native sample rate.
///
/// Mono files are duplicated to both channels. Files with more than two channels
/// keep only the first two.
pub fn decode_file(path: &Path) -> Result<StereoBuffer, DecodeError> {
    let file = File::open(path)?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }

    let mut format = symphonia::default::get_probe()
        .probe(
            &hint,
            mss,
            FormatOptions::default(),
            MetadataOptions::default(),
        )
        .map_err(|e| DecodeError::Unsupported(e.to_string()))?;

    let track = format
        .default_track(TrackType::Audio)
        .ok_or_else(|| DecodeError::Unsupported("no audio track".into()))?;
    let track_id = track.id;
    let codec_params = track
        .codec_params
        .as_ref()
        .and_then(|p| p.audio())
        .ok_or_else(|| DecodeError::Unsupported("no audio codec parameters".into()))?;
    let mut decoder = symphonia::default::get_codecs()
        .make_audio_decoder(codec_params, &AudioDecoderOptions::default())
        .map_err(|e| DecodeError::Unsupported(e.to_string()))?;

    let mut sample_rate = 0;
    let mut out = Vec::new();
    let mut packet_buf: Vec<f32> = Vec::new();

    while let Some(packet) = format.next_packet()? {
        if packet.track_id != track_id {
            continue;
        }
        let decoded = match decoder.decode(&packet) {
            Ok(decoded) => decoded,
            // A corrupt packet is skipped, like most players do.
            Err(SymphoniaError::DecodeError(_)) => continue,
            Err(e) => return Err(e.into()),
        };

        let spec = decoded.spec();
        sample_rate = spec.rate();
        let channels = spec.channels().count();
        if channels == 0 {
            continue;
        }

        packet_buf.resize(decoded.samples_interleaved(), 0.0);
        decoded.copy_to_slice_interleaved(&mut packet_buf);

        out.reserve(packet_buf.len() / channels * 2);
        for frame in packet_buf.chunks_exact(channels) {
            let left = frame[0];
            let right = if channels > 1 { frame[1] } else { left };
            out.push(left);
            out.push(right);
        }
    }

    if sample_rate == 0 {
        return Err(DecodeError::Unsupported("file contains no audio".into()));
    }
    Ok(StereoBuffer::new(sample_rate, out))
}
