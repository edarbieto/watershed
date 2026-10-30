//! BS-RoFormer on `burn` + `wgpu`, the native separation backend.
//!
//! The model takes and returns spectrograms; the STFT stays outside it. Inference
//! only: the layers are plain tensors, with no dropout and no gradients.

mod config;
mod layers;
mod model;
mod weights;

pub use config::Config;
pub use model::{BsRoformer, Precision};
pub use weights::{Weights, WeightsError};

use burn::prelude::Device;
use burn::tensor::DeviceKind;

/// The GPU, on the native graphics API: Metal on macOS, Vulkan elsewhere.
pub fn default_device() -> Device {
    #[cfg(target_os = "macos")]
    return Device::metal(DeviceKind::DefaultDevice);
    #[cfg(not(target_os = "macos"))]
    return Device::vulkan(DeviceKind::DefaultDevice);
}
