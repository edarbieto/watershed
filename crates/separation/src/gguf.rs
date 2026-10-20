//! [`Separator`] backend for GGUF models through BSRoformer.cpp.

use std::path::Path;

use watershed_dsp::StereoBuffer;

use crate::bsroformer::{LoadOptions, Model};
use crate::{SeparationError, Separator};

/// Stem order of BS-RoFormer SW, the default model (from its `.yaml`). GGUF files
/// do not store stem names; the model catalog will provide them per model.
pub const BS_ROFORMER_SW_STEMS: [&str; 6] = ["bass", "drums", "other", "vocals", "guitar", "piano"];

pub struct GgufSeparator {
    model: Model,
    stem_names: Vec<String>,
}

impl GgufSeparator {
    /// Loads a model whose stems are `stem_names`, in the model's output order.
    pub fn load(
        path: &Path,
        stem_names: Vec<String>,
        options: &LoadOptions,
    ) -> Result<Self, SeparationError> {
        let model = Model::load(path, options)?;
        let num_stems = model.info().num_stems;
        if stem_names.len() != num_stems {
            return Err(SeparationError::StemCount {
                model: num_stems,
                names: stem_names.len(),
            });
        }
        Ok(Self { model, stem_names })
    }

    pub fn model(&self) -> &Model {
        &self.model
    }
}

impl Separator for GgufSeparator {
    fn sample_rate(&self) -> u32 {
        self.model.info().sample_rate
    }

    fn stem_names(&self) -> &[String] {
        &self.stem_names
    }

    fn device(&self) -> String {
        format!(
            "{} ({})",
            self.model.backend_name(),
            self.model.device_description()
        )
    }

    fn separate(
        &mut self,
        input: &StereoBuffer,
        progress: &(dyn Fn(f32) + Sync),
        cancel: &(dyn Fn() -> bool + Sync),
    ) -> Result<Vec<StereoBuffer>, SeparationError> {
        let expected = self.sample_rate();
        if input.sample_rate != expected {
            return Err(SeparationError::SampleRate {
                expected,
                got: input.sample_rate,
            });
        }
        let stems = self.model.process(&input.samples, progress, cancel)?;
        Ok(stems
            .into_iter()
            .map(|samples| StereoBuffer::new(expected, samples))
            .collect())
    }
}
