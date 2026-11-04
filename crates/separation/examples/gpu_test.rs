//! A test of the native backend on someone else's machine: double-click it, wait, and
//! send back the one text file it writes next to itself, `watershed-gpu-test.txt`.
//!
//! It downloads the native model (~700 MB) into the models cache folder on first use,
//! then separates three minutes of a test signal, or a song dropped onto the `.exe`, and
//! logs the machine, the GPU, the time of each chunk, the peak memory and which kernels
//! ran. Nothing else is written: the stems are thrown away.
//!
//! ```sh
//! RUSTFLAGS="-C target-feature=+crt-static" cargo build --release -p watershed-separation --example gpu_test
//! ```
//! `+crt-static` puts the C runtime inside the `.exe`, so it runs on a Windows with no
//! Visual C++ redistributable installed.

use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use watershed_dsp::StereoBuffer;
use watershed_separation::{BurnSeparator, Separator, catalog};

const MODEL_ID: &str = "bs-roformer-sw";
const TEST_SECONDS: usize = 180;
const SAMPLE_RATE: u32 = 44100;

/// Writes each line to the console and to the log file, flushed at once, so that a crash
/// or a closed window still leaves everything up to that point.
struct Log(Mutex<Option<File>>);

impl Log {
    fn line(&self, text: &str) {
        println!("{text}");
        if let Some(file) = self.0.lock().expect("log lock").as_mut() {
            let _ = writeln!(file, "{text}");
            let _ = file.flush();
        }
    }
}

fn main() {
    let path = log_path();
    let log = Arc::new(Log(Mutex::new(File::create(&path).ok())));
    let panic_log = log.clone();
    std::panic::set_hook(Box::new(move |info| {
        panic_log.line(&format!("PANIC: {info}"));
    }));

    log.line("Watershed GPU test");
    log.line(&format!("log file: {}", path.display()));
    if let Err(error) = run(&log) {
        log.line(&format!("ERROR: {error}"));
    }
    log.line("");
    log.line(&format!("Done. Please send this file: {}", path.display()));
    println!("Press Enter to close.");
    let _ = std::io::stdin().read_line(&mut String::new());
}

fn log_path() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(Path::to_path_buf))
        .unwrap_or_default()
        .join("watershed-gpu-test.txt")
}

fn run(log: &Log) -> Result<(), String> {
    let unix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    log.line(&format!(
        "version: {}, unix time: {unix}",
        env!("CARGO_PKG_VERSION")
    ));
    system_info(log);

    let entry = catalog::find(MODEL_ID).ok_or("the native model is not in the catalog")?;
    let models = watershed_storage::models_dir().ok_or("no cache folder on this system")?;
    if !entry.is_installed(&models) {
        log.line(&format!(
            "downloading the model ({} MB) to {}",
            entry.size / 1_000_000,
            models.display()
        ));
        let shown = AtomicU64::new(0);
        entry
            .install(
                &models,
                &|done, total| {
                    let percent = done * 100 / total.max(1);
                    if percent >= shown.load(Ordering::Relaxed) + 5 {
                        shown.store(percent, Ordering::Relaxed);
                        println!("  download {percent} %");
                    }
                },
                &|| false,
            )
            .map_err(|e| format!("download failed: {e}"))?;
        log.line("download done");
    }

    let input = match std::env::args_os().nth(1) {
        Some(song) => {
            let song = PathBuf::from(song);
            log.line(&format!("input: a song, {}", song.display()));
            let audio = watershed_dsp::decode_file(&song).map_err(|e| e.to_string())?;
            watershed_dsp::resample(audio, SAMPLE_RATE).map_err(|e| e.to_string())?
        }
        None => {
            log.line(&format!("input: {TEST_SECONDS} s of test signal"));
            test_signal()
        }
    };
    log.line(&format!("length: {:.1} s", input.duration_secs()));

    let peak = Arc::new(AtomicU64::new(0));
    let stop = Arc::new(AtomicBool::new(false));
    let sampler = memory_sampler(peak.clone(), stop.clone());

    let start = Instant::now();
    let config = models.join(
        &entry
            .native
            .as_ref()
            .ok_or("the model is not a native one")?
            .config
            .file,
    );
    let mut separator = BurnSeparator::load(&entry.path_in(&models), &config, entry.stems.clone())
        .map_err(|e| format!("cannot load the model: {e}"))?;
    log.line(&format!(
        "model loaded in {:.1} s on {}",
        start.elapsed().as_secs_f32(),
        separator.device()
    ));

    let start = Instant::now();
    let last = Mutex::new((0usize, Instant::now()));
    let chunk_times = Mutex::new(Vec::new());
    let result = separator.separate(
        &input,
        &|fraction| {
            let mut last = last.lock().expect("lock");
            let seconds = last.1.elapsed().as_secs_f64();
            last.0 += 1;
            last.1 = Instant::now();
            chunk_times.lock().expect("lock").push(seconds);
            log.line(&format!(
                "chunk {:3}: {seconds:6.2} s ({:5.1} %)",
                last.0,
                fraction * 100.0
            ));
        },
        &|| false,
    );
    let total = start.elapsed().as_secs_f64();
    stop.store(true, Ordering::Relaxed);
    let _ = sampler.join();
    result.map_err(|e| format!("separation failed: {e}"))?;

    let times = chunk_times.into_inner().expect("lock");
    log.line("");
    log.line("RESULT");
    log.line(&format!("separating: {total:.1} s"));
    if times.len() > 1 {
        let rest = &times[1..];
        log.line(&format!(
            "chunks: {}, first {:.2} s, the rest {:.2} s on average",
            times.len(),
            times[0],
            rest.iter().sum::<f64>() / rest.len() as f64
        ));
    }
    log.line(&format!(
        "speed: {:.2}x real time",
        input.duration_secs() / total
    ));
    log.line(&format!(
        "process memory, peak: {} MiB",
        peak.load(Ordering::Relaxed) >> 20
    ));
    log.line(&watershed_roformer::diagnostics(
        &watershed_roformer::default_device(),
    ));
    Ok(())
}

fn system_info(log: &Log) {
    use sysinfo::System;
    let mut system = System::new_all();
    system.refresh_all();
    log.line(&format!(
        "OS: {}",
        System::long_os_version().unwrap_or_default()
    ));
    let cpu = system
        .cpus()
        .first()
        .map(|c| c.brand().trim().to_string())
        .unwrap_or_default();
    log.line(&format!("CPU: {cpu}, {} threads", system.cpus().len()));
    log.line(&format!(
        "RAM: {} MiB total, {} MiB available",
        system.total_memory() >> 20,
        system.available_memory() >> 20
    ));
}

/// Three minutes of something like music, the same on every machine: a few tones that
/// change every half second over quiet noise, with a pulse like a kick drum. Speed does
/// not depend on what the audio is, only on its length.
fn test_signal() -> StereoBuffer {
    let frames = TEST_SECONDS * SAMPLE_RATE as usize;
    let mut samples = Vec::with_capacity(frames * 2);
    let mut seed: u32 = 1;
    for frame in 0..frames {
        let t = frame as f32 / SAMPLE_RATE as f32;
        let note = (t * 2.0) as u32 % 8;
        let freq = 110.0 * 2f32.powf(note as f32 / 12.0);
        let tone = (t * freq * std::f32::consts::TAU).sin() * 0.2
            + (t * freq * 1.5 * std::f32::consts::TAU).sin() * 0.1;
        let beat = t.fract();
        let kick = (beat * 60.0 * std::f32::consts::TAU).sin() * (-beat * 20.0).exp() * 0.4;
        seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        let noise = (seed >> 9) as f32 / (1 << 23) as f32 - 0.5;
        samples.push(tone + kick + noise * 0.05);
        samples.push(tone * 0.8 + kick + noise * 0.05);
    }
    StereoBuffer::new(SAMPLE_RATE, samples)
}

/// Samples the process's resident memory every 100 ms and keeps the peak.
fn memory_sampler(peak: Arc<AtomicU64>, stop: Arc<AtomicBool>) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System};
        let Ok(pid) = sysinfo::get_current_pid() else {
            return;
        };
        let mut system = System::new();
        while !stop.load(Ordering::Relaxed) {
            system.refresh_processes_specifics(
                ProcessesToUpdate::Some(&[pid]),
                false,
                ProcessRefreshKind::nothing().with_memory(),
            );
            if let Some(process) = system.process(pid) {
                peak.fetch_max(process.memory(), Ordering::Relaxed);
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
    })
}
