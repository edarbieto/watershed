//! Separates one song with the catalog's model into a new folder of stems (Ogg Vorbis,
//! or FLAC with `--flac`) plus a session, ready to open in the app. The model is
//! downloaded into the models cache folder on first use.
//!
//! ```sh
//! cargo run --release -p watershed-separation --example separate -- song.mp3 out-dir [--flac] [--memory rss.csv] [--repeat N]
//! ```
//!
//! `--memory` samples the process's resident memory every 10 ms, prints its peak
//! per stage and writes the samples to a CSV file. On macOS it also samples the
//! memory footprint (Activity Monitor's "Memory"). Both include large blocks that
//! macOS's malloc keeps after they are freed, for reuse; to see only the memory
//! in use, run with `MallocLargeCache=0` (undocumented, for measuring only).
//!
//! `--repeat N` separates the song N times in one process, into `out-dir/run-1` and
//! so on, each on a new thread that loads and frees the model, as the app does. With
//! `--memory` it prints the memory left after each run: flat is a fixed cost, growing
//! is a leak. On macOS it also writes `vmmap --summary` next to the CSV after each run.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use anyhow::{Context, bail};
use watershed_separation::{Separator, Stage, catalog};
use watershed_storage::StemFormat;

fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);
    let (mut flac, mut memory_csv, mut repeat, mut paths) = (false, None, None, Vec::new());
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--flac" => flac = true,
            "--memory" => {
                memory_csv = Some(PathBuf::from(args.next().context("--memory needs a file")?))
            }
            "--repeat" => {
                let count = args.next().context("--repeat needs a number")?;
                repeat = Some(count.parse::<u32>().context("--repeat needs a number")?);
            }
            _ => paths.push(arg),
        }
    }
    let [song_path, out_dir] = &paths[..] else {
        bail!("usage: separate <song> <out-dir> [--flac] [--memory rss.csv] [--repeat N]");
    };

    let entry = catalog::default_model();
    let models_dir = watershed_storage::models_dir().context("no cache folder on this system")?;
    if !entry.is_installed(&models_dir) {
        println!("installing {} in {}", entry.name, models_dir.display());
    }
    entry.install(
        &models_dir,
        &|done, total| eprint!("\r{:6.1} / {:.1} MB", done as f64 / 1e6, total as f64 / 1e6),
        &|| false,
    )?;
    eprintln!();

    let memory = memory_csv.map(MemoryLog::start);

    if let Some(memory) = &memory {
        memory.report("before loading");
    }
    let separate = |run: u32| -> anyhow::Result<()> {
        if let Some(memory) = &memory {
            memory.set_phase(0);
        }
        let start = Instant::now();
        let mut separator = entry.load(&models_dir)?;
        println!(
            "loaded in {:.1} s on {}: {} stems at {} Hz",
            start.elapsed().as_secs_f64(),
            separator.device(),
            separator.stem_names().len(),
            separator.sample_rate()
        );

        let folder = match repeat {
            Some(_) => Path::new(out_dir).join(format!("run-{run}")),
            None => PathBuf::from(out_dir),
        };
        let start = Instant::now();
        let stage_start = Mutex::new((Stage::Decoding, Instant::now()));
        let separated = watershed_separation::separate_song(
            &mut separator,
            Path::new(song_path),
            &folder,
            if flac {
                StemFormat::Lossless
            } else {
                StemFormat::Lossy
            },
            &|stage, f| {
                if let Some(memory) = &memory {
                    memory.set_phase(stage as u8 + 1);
                }
                let mut current = stage_start.lock().unwrap();
                if current.0 != stage {
                    eprintln!(
                        "\r{:?}: {:.1} s",
                        current.0,
                        current.1.elapsed().as_secs_f64()
                    );
                    *current = (stage, Instant::now());
                }
                eprint!("\r{stage:?} {:5.1} %", f * 100.0);
            },
            &|| false,
        )?;
        let (stage, since) = *stage_start.lock().unwrap();
        eprintln!("\r{stage:?}: {:.1} s      ", since.elapsed().as_secs_f64());

        println!(
            "done in {:.1} s, stem gain {:.1} dB: {} files in {}",
            start.elapsed().as_secs_f64(),
            20.0 * separated.stem_gain.log10(),
            separated.files.len(),
            separated.folder.display()
        );
        drop(separator);
        if let Some(memory) = &memory {
            memory.set_phase(PHASES.len() as u8 - 1);
        }
        Ok(())
    };
    for run in 1..=repeat.unwrap_or(1) {
        std::thread::scope(|scope| {
            let thread = scope.spawn(|| separate(run));
            thread.join().expect("separation thread panicked")
        })?;
        if let Some(memory) = &memory {
            std::thread::sleep(Duration::from_secs(2));
            memory.report(&format!("after run {run}"));
            let usage = watershed_roformer::default_device().memory_pool_usage();
            println!("  burn's pools: {usage:?}");
            memory.vmmap(run);
        }
    }
    if let Some(memory) = memory {
        memory.finish()?;
    }
    Ok(())
}

/// Phase 0 is loading the model, then one per [`Stage`], then after freeing the model.
const PHASES: [&str; 5] = ["Loading", "Decoding", "Separating", "Writing", "Freed"];

struct Sample {
    seconds: f64,
    phase: u8,
    resident: u64,
    footprint: Option<u64>,
}

/// Samples the resident memory on a background thread.
struct MemoryLog {
    phase: Arc<AtomicU8>,
    stop: Arc<AtomicBool>,
    thread: std::thread::JoinHandle<Vec<Sample>>,
    csv: PathBuf,
}

impl MemoryLog {
    fn start(csv: PathBuf) -> Self {
        let phase = Arc::new(AtomicU8::new(0));
        let stop = Arc::new(AtomicBool::new(false));
        let (thread_phase, thread_stop) = (phase.clone(), stop.clone());
        let thread = std::thread::spawn(move || {
            use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System};
            let pid: Pid = sysinfo::get_current_pid().expect("no pid");
            let mut system = System::new();
            let start = Instant::now();
            let mut samples = Vec::new();
            while !thread_stop.load(Ordering::Relaxed) {
                system.refresh_processes_specifics(
                    ProcessesToUpdate::Some(&[pid]),
                    false,
                    ProcessRefreshKind::nothing().with_memory(),
                );
                if let Some(process) = system.process(pid) {
                    samples.push(Sample {
                        seconds: start.elapsed().as_secs_f64(),
                        phase: thread_phase.load(Ordering::Relaxed),
                        resident: process.memory(),
                        footprint: footprint(),
                    });
                }
                std::thread::sleep(Duration::from_millis(10));
            }
            samples
        });
        Self {
            phase,
            stop,
            thread,
            csv,
        }
    }

    fn set_phase(&self, phase: u8) {
        self.phase.store(phase, Ordering::Relaxed);
    }

    fn report(&self, moment: &str) {
        let resident = sysinfo::get_current_pid().ok().and_then(|pid| {
            use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System};
            let mut system = System::new();
            system.refresh_processes_specifics(
                ProcessesToUpdate::Some(&[pid]),
                false,
                ProcessRefreshKind::nothing().with_memory(),
            );
            Some(system.process(pid)?.memory())
        });
        let mib = |bytes: Option<u64>| bytes.unwrap_or(0) / (1024 * 1024);
        println!(
            "{moment}: resident {} MiB, footprint {} MiB",
            mib(resident),
            mib(footprint())
        );
    }

    /// Saves what the memory is made of, on macOS.
    fn vmmap(&self, run: u32) {
        if cfg!(target_os = "macos") {
            let summary = std::process::Command::new("vmmap")
                .args(["--summary", &std::process::id().to_string()])
                .output();
            match summary {
                Ok(summary) => {
                    let path = self.csv.with_extension(format!("run-{run}.vmmap.txt"));
                    if let Err(e) = std::fs::write(&path, summary.stdout) {
                        eprintln!("cannot write {}: {e}", path.display());
                    }
                }
                Err(e) => eprintln!("vmmap failed: {e}"),
            }
        }
    }

    fn finish(self) -> anyhow::Result<()> {
        self.stop.store(true, Ordering::Relaxed);
        let samples = self.thread.join().expect("memory sampler panicked");
        let mib = |bytes: u64| bytes as f64 / (1024.0 * 1024.0);
        println!("memory, MiB: peak / at the end of each phase");
        println!("  {:<10} {:>17} {:>17}", "", "resident", "footprint");
        for (i, name) in PHASES.iter().enumerate() {
            let phase: Vec<_> = samples.iter().filter(|s| s.phase as usize == i).collect();
            let Some(last) = phase.last() else { continue };
            let peak_resident = phase.iter().map(|s| s.resident).max().unwrap_or(0);
            let footprint = match (
                phase.iter().filter_map(|s| s.footprint).max(),
                last.footprint,
            ) {
                (Some(peak), Some(end)) => format!("{:7.0} / {:7.0}", mib(peak), mib(end)),
                _ => "-".into(),
            };
            println!(
                "  {name:<10} {:7.0} / {:7.0} {footprint:>17}",
                mib(peak_resident),
                mib(last.resident)
            );
        }
        let mut csv = String::from("seconds,phase,resident_mib,footprint_mib\n");
        for s in &samples {
            let footprint = s
                .footprint
                .map(|f| format!("{:.1}", mib(f)))
                .unwrap_or_default();
            csv.push_str(&format!(
                "{:.3},{},{:.1},{footprint}\n",
                s.seconds,
                PHASES[s.phase as usize],
                mib(s.resident)
            ));
        }
        std::fs::write(&self.csv, csv)
            .with_context(|| format!("cannot write {}", self.csv.display()))?;
        Ok(())
    }
}

/// The process's physical memory footprint, as Activity Monitor shows it.
#[cfg(target_os = "macos")]
fn footprint() -> Option<u64> {
    // `rusage_info_v0` from <sys/resource.h>.
    #[repr(C)]
    #[derive(Default)]
    struct RusageInfoV0 {
        uuid: [u8; 16],
        user_time: u64,
        system_time: u64,
        pkg_idle_wkups: u64,
        interrupt_wkups: u64,
        pageins: u64,
        wired_size: u64,
        resident_size: u64,
        phys_footprint: u64,
        proc_start_abstime: u64,
        proc_exit_abstime: u64,
    }
    unsafe extern "C" {
        fn proc_pid_rusage(pid: i32, flavor: i32, buffer: *mut RusageInfoV0) -> i32;
    }
    let mut info = RusageInfoV0::default();
    // SAFETY: flavor 0 (RUSAGE_INFO_V0) fills exactly this struct.
    let status = unsafe { proc_pid_rusage(std::process::id() as i32, 0, &mut info) };
    (status == 0).then_some(info.phys_footprint)
}

#[cfg(not(target_os = "macos"))]
fn footprint() -> Option<u64> {
    None
}
