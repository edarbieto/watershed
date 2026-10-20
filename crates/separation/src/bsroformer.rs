//! BS-RoFormer GGUF models through BSRoformer.cpp + ggml (built by `build.rs`).
//!
//! The C interface is `cpp/wrapper.h`; [`Model`] is the safe wrapper around it.

use std::ffi::{CStr, CString, c_char, c_int, c_void};
use std::num::NonZeroUsize;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Path, PathBuf};
use std::ptr::NonNull;

#[derive(Debug, thiserror::Error)]
pub enum BsrError {
    #[error("model path is not valid UTF-8 or contains a NUL byte: {0}")]
    InvalidPath(PathBuf),
    #[error("cannot load model: {0}")]
    Load(String),
    #[error("input must be interleaved stereo (an even number of samples)")]
    NotStereo,
    #[error("separation failed: {0}")]
    Process(String),
    #[error("separation cancelled")]
    Cancelled,
}

/// Which ggml backend to run on.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Device {
    /// The best one available: Metal on macOS, Vulkan on Windows and Linux, else CPU.
    #[default]
    Auto,
    Cpu,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct LoadOptions {
    pub device: Device,
    /// Threads for the CPU backend. `None` uses the number of physical cores
    /// (ggml's own default is 4, and extra hyperthreads barely help).
    pub cpu_threads: Option<NonZeroUsize>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ModelInfo {
    pub sample_rate: u32,
    pub num_stems: usize,
}

/// A loaded model. Weights live on the chosen device until it is dropped.
pub struct Model {
    raw: NonNull<ffi::WsBsrModel>,
    info: ModelInfo,
    backend_name: String,
    device_description: String,
}

// SAFETY: the C++ model is not tied to the thread that created it; `process`
// takes `&mut self`, so it is never used from two threads at once.
unsafe impl Send for Model {}

const ERR_LEN: usize = 1024;

impl Model {
    pub fn load(path: &Path, options: &LoadOptions) -> Result<Self, BsrError> {
        let c_path = path
            .to_str()
            .and_then(|s| CString::new(s).ok())
            .ok_or_else(|| BsrError::InvalidPath(path.to_path_buf()))?;
        let threads = options
            .cpu_threads
            .map(NonZeroUsize::get)
            .or_else(sysinfo::System::physical_core_count)
            .unwrap_or(0);
        let threads = c_int::try_from(threads).unwrap_or(c_int::MAX);
        let force_cpu = c_int::from(options.device == Device::Cpu);

        let mut err = [0 as c_char; ERR_LEN];
        // SAFETY: valid NUL-terminated path and an error buffer of the given length.
        let raw = unsafe {
            ffi::ws_bsr_load(
                c_path.as_ptr(),
                force_cpu,
                threads,
                err.as_mut_ptr(),
                ERR_LEN,
            )
        };
        let raw = NonNull::new(raw).ok_or_else(|| BsrError::Load(message(&err)))?;

        let mut c_info = ffi::WsBsrInfo::default();
        // SAFETY: `raw` is a live model; the returned strings live as long as it.
        let (backend_name, device_description) = unsafe {
            ffi::ws_bsr_info(raw.as_ptr(), &mut c_info);
            (
                CStr::from_ptr(ffi::ws_bsr_backend_name(raw.as_ptr()))
                    .to_string_lossy()
                    .into_owned(),
                // CPU names come padded with spaces.
                CStr::from_ptr(ffi::ws_bsr_device_description(raw.as_ptr()))
                    .to_string_lossy()
                    .trim()
                    .to_owned(),
            )
        };
        let model = Self {
            raw,
            info: ModelInfo {
                sample_rate: u32::try_from(c_info.sample_rate).unwrap_or(0),
                num_stems: usize::try_from(c_info.num_stems).unwrap_or(0),
            },
            backend_name,
            device_description,
        };
        if model.info.sample_rate == 0 || model.info.num_stems == 0 {
            return Err(BsrError::Load(format!(
                "invalid model parameters: {:?}",
                model.info
            )));
        }
        Ok(model)
    }

    pub fn info(&self) -> ModelInfo {
        self.info
    }

    /// ggml backend name, e.g. `Vulkan0`, `MTL0` or `CPU`.
    pub fn backend_name(&self) -> &str {
        &self.backend_name
    }

    /// The device behind the backend, e.g. the GPU's name.
    pub fn device_description(&self) -> &str {
        &self.device_description
    }

    pub fn is_cpu(&self) -> bool {
        self.backend_name.eq_ignore_ascii_case("cpu")
    }

    /// Separates interleaved stereo audio at the model's sample rate into
    /// `num_stems` interleaved stereo buffers of the same length, using the chunk
    /// size and overlap stored in the model.
    ///
    /// `progress` gets the fraction done (0..1) from a worker thread. `cancel` is
    /// polled while it runs; returning `true` stops with [`BsrError::Cancelled`].
    pub fn process(
        &mut self,
        input: &[f32],
        progress: &(dyn Fn(f32) + Sync),
        cancel: &(dyn Fn() -> bool + Sync),
    ) -> Result<Vec<Vec<f32>>, BsrError> {
        if !input.len().is_multiple_of(2) {
            return Err(BsrError::NotStereo);
        }
        let mut stems = vec![vec![0.0f32; input.len()]; self.info.num_stems];
        if input.is_empty() {
            return Ok(stems);
        }
        let outputs: Vec<*mut f32> = stems.iter_mut().map(|s| s.as_mut_ptr()).collect();
        let callbacks = Callbacks { progress, cancel };
        let mut err = [0 as c_char; ERR_LEN];

        // SAFETY: `outputs` holds `num_stems` buffers of `input.len()` floats each;
        // `callbacks` outlives the call and its closures are `Sync`, as the progress
        // trampoline runs on a worker thread.
        let status = unsafe {
            ffi::ws_bsr_process(
                self.raw.as_ptr(),
                input.as_ptr(),
                input.len() / 2,
                0,
                0,
                outputs.as_ptr(),
                Some(progress_trampoline),
                Some(cancel_trampoline),
                (&raw const callbacks).cast_mut().cast(),
                err.as_mut_ptr(),
                ERR_LEN,
            )
        };
        match status {
            ffi::WS_BSR_OK => Ok(stems),
            ffi::WS_BSR_CANCELLED => Err(BsrError::Cancelled),
            _ => Err(BsrError::Process(message(&err))),
        }
    }
}

impl Drop for Model {
    fn drop(&mut self) {
        // SAFETY: `raw` came from `ws_bsr_load` and is freed exactly once.
        unsafe { ffi::ws_bsr_free(self.raw.as_ptr()) }
    }
}

struct Callbacks<'a> {
    progress: &'a (dyn Fn(f32) + Sync),
    cancel: &'a (dyn Fn() -> bool + Sync),
}

// A panic must not unwind into C++: progress panics are dropped, cancel panics
// cancel.
extern "C" fn progress_trampoline(user: *mut c_void, fraction: f32) {
    // SAFETY: `user` points to the `Callbacks` alive for the whole `process` call.
    let callbacks = unsafe { &*user.cast::<Callbacks>() };
    let _ = catch_unwind(AssertUnwindSafe(|| (callbacks.progress)(fraction)));
}

extern "C" fn cancel_trampoline(user: *mut c_void) -> c_int {
    // SAFETY: as in `progress_trampoline`.
    let callbacks = unsafe { &*user.cast::<Callbacks>() };
    c_int::from(catch_unwind(AssertUnwindSafe(|| (callbacks.cancel)())).unwrap_or(true))
}

fn message(err: &[c_char]) -> String {
    let bytes: Vec<u8> = err
        .iter()
        .take_while(|&&c| c != 0)
        .map(|&c| c as u8)
        .collect();
    String::from_utf8_lossy(&bytes).into_owned()
}

mod ffi {
    use std::ffi::{c_char, c_int, c_void};

    pub const WS_BSR_OK: c_int = 0;
    pub const WS_BSR_CANCELLED: c_int = 2;

    #[repr(C)]
    pub struct WsBsrModel {
        _private: [u8; 0],
    }

    #[repr(C)]
    #[derive(Default)]
    pub struct WsBsrInfo {
        pub sample_rate: c_int,
        pub num_stems: c_int,
        pub chunk_size: c_int,
        pub num_overlap: c_int,
    }

    pub type ProgressFn = extern "C" fn(user: *mut c_void, fraction: f32);
    pub type CancelFn = extern "C" fn(user: *mut c_void) -> c_int;

    unsafe extern "C" {
        pub fn ws_bsr_load(
            model_path_utf8: *const c_char,
            force_cpu: c_int,
            n_threads: c_int,
            err: *mut c_char,
            err_len: usize,
        ) -> *mut WsBsrModel;
        pub fn ws_bsr_free(model: *mut WsBsrModel);
        pub fn ws_bsr_info(model: *const WsBsrModel, out: *mut WsBsrInfo);
        pub fn ws_bsr_backend_name(model: *const WsBsrModel) -> *const c_char;
        pub fn ws_bsr_device_description(model: *const WsBsrModel) -> *const c_char;
        #[allow(clippy::too_many_arguments)]
        pub fn ws_bsr_process(
            model: *mut WsBsrModel,
            input: *const f32,
            num_frames: usize,
            chunk_size: c_int,
            num_overlap: c_int,
            outputs: *const *mut f32,
            progress: Option<ProgressFn>,
            cancel: Option<CancelFn>,
            user: *mut c_void,
            err: *mut c_char,
            err_len: usize,
        ) -> c_int;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_model_file_is_an_error_not_a_crash() {
        let path = std::env::temp_dir().join("watershed-not-a-model.gguf");
        std::fs::write(&path, b"this is not a GGUF file").unwrap();
        let result = Model::load(&path, &LoadOptions::default());
        std::fs::remove_file(&path).ok();
        match result {
            Err(BsrError::Load(msg)) => assert!(msg.contains("GGUF"), "{msg}"),
            Err(e) => panic!("unexpected error: {e}"),
            Ok(_) => panic!("garbage loaded as a model"),
        }
    }

    /// Runs a real model when `WATERSHED_TEST_GGUF` points to one (not in CI).
    #[test]
    fn separates_a_short_clip_when_a_model_is_available() {
        let Some(path) = std::env::var_os("WATERSHED_TEST_GGUF") else {
            eprintln!("WATERSHED_TEST_GGUF not set, skipping");
            return;
        };
        let mut model = Model::load(Path::new(&path), &LoadOptions::default()).unwrap();
        let info = model.info();
        assert_eq!(info.sample_rate, 44_100);

        // One second of a 440 Hz tone.
        let input: Vec<f32> = (0..info.sample_rate as usize)
            .flat_map(|i| {
                let s = (i as f32 * 440.0 * std::f32::consts::TAU / 44_100.0).sin() * 0.5;
                [s, s]
            })
            .collect();
        let last = std::sync::Mutex::new(0.0f32);
        let stems = model
            .process(&input, &|f| *last.lock().unwrap() = f, &|| false)
            .unwrap();
        assert_eq!(stems.len(), info.num_stems);
        assert!(stems.iter().all(|s| s.len() == input.len()));
        assert!(stems.iter().flatten().all(|x| x.is_finite()));
        assert!(*last.lock().unwrap() > 0.99);
    }
}
