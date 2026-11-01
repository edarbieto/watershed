//! From a song to its stems: STFT, masks, inverse STFT, chunk by chunk.

use burn::prelude::*;
use burn::tensor::{DType, TensorData};
use watershed_dsp::{Complex32, StereoBuffer, overlap_add};

use crate::model::BsRoformer;

const CHANNELS: usize = 2;

impl BsRoformer {
    /// One buffer per stem, each as long as `input`.
    pub fn separate(&self, input: &StereoBuffer) -> Vec<StereoBuffer> {
        let stems = self.mask_estimators.len();
        let run = overlap_add(input, stems, self.chunking, |chunk| {
            Ok::<_, std::convert::Infallible>(self.separate_chunk(chunk))
        });
        let Ok(stems) = run;
        stems
    }

    /// One interleaved stereo chunk of the model's size to the same for each stem.
    pub fn separate_chunk(&self, chunk: &[f32]) -> Vec<Vec<f32>> {
        let length = chunk.len() / CHANNELS;
        let spectra = [0, 1].map(|channel| {
            let signal: Vec<f32> = chunk
                .iter()
                .skip(channel)
                .step_by(CHANNELS)
                .copied()
                .collect();
            self.stft.forward(&signal)
        });

        let freqs = self.stft.freqs();
        let frames = self.stft.frames(length);
        let input = TensorData::new(
            model_input(&spectra, freqs, frames),
            [1, freqs * CHANNELS, frames, 2],
        );
        let masks: Vec<f32> = self
            .forward(Tensor::from_data(input, &self.device))
            .cast(DType::F32)
            .into_data()
            .try_to_vec()
            .expect("cast to f32");

        masks
            .chunks_exact(freqs * CHANNELS * frames * 2)
            .map(|masks| {
                let channels = [0, 1].map(|channel| {
                    let masked = apply_mask(&spectra[channel], masks, channel, freqs, frames);
                    self.stft.inverse(&masked, length)
                });
                channels[0]
                    .iter()
                    .zip(&channels[1])
                    .flat_map(|(left, right)| [*left, *right])
                    .collect()
            })
            .collect()
    }
}

/// Real and imaginary parts as (frequency * channel, time, 2), which is what the
/// model reads, from spectrograms stored frame after frame.
fn model_input(spectra: &[Vec<Complex32>; CHANNELS], freqs: usize, frames: usize) -> Vec<f32> {
    let mut input = Vec::with_capacity(freqs * CHANNELS * frames * 2);
    for freq in 0..freqs {
        for spectrum in spectra {
            for frame in 0..frames {
                let bin = spectrum[frame * freqs + freq];
                input.extend([bin.re, bin.im]);
            }
        }
    }
    input
}

/// `spectrum` times one stem's complex masks, with the DC bin silenced as the original
/// model does. `masks` is (frequency * channel, time, 2).
fn apply_mask(
    spectrum: &[Complex32],
    masks: &[f32],
    channel: usize,
    freqs: usize,
    frames: usize,
) -> Vec<Complex32> {
    let mut masked = vec![Complex32::ZERO; spectrum.len()];
    for freq in 1..freqs {
        let row = &masks[(freq * CHANNELS + channel) * frames * 2..][..frames * 2];
        for (frame, mask) in row.as_chunks::<2>().0.iter().enumerate() {
            let index = frame * freqs + freq;
            masked[index] = spectrum[index] * Complex32::new(mask[0], mask[1]);
        }
    }
    masked
}
