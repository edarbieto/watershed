//! Real-time playback engine: mixes stems in a single cpal callback.
//!
//! The UI thread owns an [`Engine`]. Songs are handed to the audio thread through a
//! lock-free queue and handed back for dropping, so the audio callback never
//! allocates or frees memory. Mixer parameters are plain atomics.
//!
//! Signal path: stems → mixer → tempo and pitch (Signalsmith Stretch) → master gain
//! → limiter → device.

mod atomic;
mod engine;
mod limiter;
mod mixer;
mod song;
mod stretch;

pub use atomic::AtomicF32;
pub use engine::{Engine, EngineError, MIN_LOOP_SECS, Notify, OutputDevice, output_devices};
pub use song::{Song, Track, TrackParams};
pub use stretch::{MAX_SEMITONES, MAX_TEMPO, MIN_TEMPO};
