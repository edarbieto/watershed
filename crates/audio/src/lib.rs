//! Real-time playback engine: mixes stems in a single cpal callback.
//!
//! The UI thread owns an [`Engine`]. Songs are handed to the audio thread through a
//! lock-free queue and handed back for dropping, so the audio callback never
//! allocates or frees memory. Mixer parameters are plain atomics.

mod atomic;
mod engine;
mod mixer;
mod song;

pub use atomic::AtomicF32;
pub use engine::{Engine, EngineError, OutputDevice, output_devices};
pub use song::{Song, Track, TrackParams};
