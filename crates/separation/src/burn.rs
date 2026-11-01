//! [`Separator`] backend for PyTorch checkpoints, on `burn` + `wgpu`.

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::Path;

use watershed_dsp::StereoBuffer;
use watershed_roformer::{BsRoformer, Config, Precision, Weights, default_device};

use crate::{SeparationError, Separator};

pub struct BurnSeparator {
    /// Only empty while dropping.
    model: Option<BsRoformer>,
    stem_names: Vec<String>,
    sample_rate: u32,
}

/// `burn` panics on GPU errors, such as a machine with no usable GPU. Without this the
/// job's thread would die and the app would wait for it forever.
fn guarded<T>(work: impl FnOnce() -> Result<T, SeparationError>) -> Result<T, SeparationError> {
    catch_unwind(AssertUnwindSafe(work)).unwrap_or_else(|panic| {
        let text = panic
            .downcast_ref::<String>()
            .map(String::as_str)
            .or(panic.downcast_ref::<&str>().copied())
            .unwrap_or("unknown error");
        Err(SeparationError::Native(format!("GPU error: {text}")))
    })
}

fn native(error: impl std::fmt::Display) -> SeparationError {
    SeparationError::Native(error.to_string())
}

impl BurnSeparator {
    /// Loads `checkpoint` with the parameters in the `.yaml` at `config`, for a model
    /// whose stems are `stem_names`, in the model's output order.
    ///
    /// Computes in f16, which is as good to the ear as f32 and much faster.
    pub fn load(
        checkpoint: &Path,
        config: &Path,
        stem_names: Vec<String>,
    ) -> Result<Self, SeparationError> {
        let config = Config::open(config).map_err(native)?;
        if stem_names.len() != config.num_stems {
            return Err(SeparationError::StemCount {
                model: config.num_stems,
                names: stem_names.len(),
            });
        }
        let weights = Weights::open(checkpoint).map_err(native)?;
        let model = guarded(|| {
            BsRoformer::load(&weights, &config, Precision::Half, &default_device()).map_err(native)
        })?;
        Ok(Self {
            model: Some(model),
            stem_names,
            sample_rate: config.sample_rate,
        })
    }
}

impl Separator for BurnSeparator {
    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    fn stem_names(&self) -> &[String] {
        &self.stem_names
    }

    fn device(&self) -> String {
        let api = if cfg!(target_os = "macos") {
            "Metal"
        } else {
            "Vulkan"
        };
        format!("{api} (native)")
    }

    fn separate(
        &mut self,
        input: &StereoBuffer,
        progress: &(dyn Fn(f32) + Sync),
        cancel: &(dyn Fn() -> bool + Sync),
    ) -> Result<Vec<StereoBuffer>, SeparationError> {
        if input.sample_rate != self.sample_rate {
            return Err(SeparationError::SampleRate {
                expected: self.sample_rate,
                got: input.sample_rate,
            });
        }
        let model = self.model.as_ref().expect("only empty while dropping");
        guarded(|| {
            model
                .separate(input, progress, cancel)
                .map_err(|_| SeparationError::Cancelled)
        })
    }
}

impl Drop for BurnSeparator {
    /// The GPU's memory pools outlive the model, and on Apple Silicon that memory is
    /// the system's RAM.
    fn drop(&mut self) {
        drop(self.model.take());
        let _ = catch_unwind(|| default_device().memory_cleanup());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_gpu_panic_becomes_an_error() {
        let result: Result<(), _> = guarded(|| panic!("no adapter"));
        assert!(
            matches!(&result, Err(SeparationError::Native(e)) if e.contains("no adapter")),
            "{result:?}"
        );
    }

    #[test]
    fn errors_and_values_pass_through() {
        assert!(matches!(guarded(|| Ok(1)), Ok(1)));
        let result: Result<(), _> = guarded(|| Err(SeparationError::Cancelled));
        assert!(matches!(result, Err(SeparationError::Cancelled)));
    }
}
