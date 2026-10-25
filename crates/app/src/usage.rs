//! CPU, GPU and memory used by the Watershed process, sampled about once per
//! second. Operating systems measure this per process, which includes all its
//! windows and threads (UI, audio, loading); see [`gpu`] for the GPU's exceptions.

mod gpu;

pub use gpu::WHOLE_GPU;

use std::time::{Duration, Instant};

use sysinfo::{MemoryRefreshKind, Pid, ProcessRefreshKind, ProcessesToUpdate, System};

/// How often the numbers are refreshed. CPU usage is averaged over this interval.
pub const INTERVAL: Duration = Duration::from_secs(1);

/// The latest sample.
#[derive(Clone, Copy, Debug, Default)]
pub struct Sample {
    /// Share of the whole machine (all cores), in percent.
    pub cpu_percent: f32,
    /// The busiest GPU engine's share of time, in percent, if the system reports it.
    pub gpu_percent: Option<f32>,
    /// Memory in use, in bytes: on macOS the footprint, so it matches Activity
    /// Monitor's "Memory" column; elsewhere the resident size, like Task Manager.
    pub memory: u64,
    /// Share of the installed memory, in percent.
    pub memory_percent: f32,
}

pub struct Usage {
    system: System,
    pid: Option<Pid>,
    cores: usize,
    gpu: gpu::Gpu,
    last: Option<Instant>,
    sample: Option<Sample>,
}

impl Usage {
    pub fn new() -> Self {
        let mut system = System::new();
        system.refresh_memory_specifics(MemoryRefreshKind::nothing().with_ram());
        Self {
            system,
            pid: sysinfo::get_current_pid().ok(),
            cores: std::thread::available_parallelism().map_or(1, |n| n.get()),
            gpu: gpu::Gpu::new(),
            last: None,
            sample: None,
        }
    }

    /// The latest sample, refreshed if [`INTERVAL`] has passed. `None` until two
    /// refreshes have been made, since CPU usage needs a time span to measure.
    pub fn sample(&mut self) -> Option<Sample> {
        let now = Instant::now();
        if self
            .last
            .is_none_or(|last| now.duration_since(last) >= INTERVAL)
        {
            let first = self.last.is_none();
            self.last = Some(now);
            self.refresh(first);
        }
        self.sample
    }

    fn refresh(&mut self, first: bool) {
        let Some(pid) = self.pid else {
            return;
        };
        self.system.refresh_processes_specifics(
            ProcessesToUpdate::Some(&[pid]),
            false,
            ProcessRefreshKind::nothing().with_cpu().with_memory(),
        );
        let gpu_percent = self.gpu.sample();
        if first {
            return;
        }
        let Some(process) = self.system.process(pid) else {
            return;
        };
        let total = self.system.total_memory().max(1);
        let memory = footprint().unwrap_or_else(|| process.memory());
        self.sample = Some(Sample {
            // sysinfo reports a share of one core, up to 100 % per core.
            cpu_percent: process.cpu_usage() / self.cores as f32,
            gpu_percent,
            memory,
            memory_percent: memory as f32 / total as f32 * 100.0,
        });
    }
}

/// The process's physical memory footprint on macOS.
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
    let pid = i32::try_from(std::process::id()).ok()?;
    // SAFETY: flavor 0 (RUSAGE_INFO_V0) fills exactly this struct.
    let status = unsafe { proc_pid_rusage(pid, 0, &mut info) };
    (status == 0).then_some(info.phys_footprint)
}

#[cfg(not(target_os = "macos"))]
fn footprint() -> Option<u64> {
    None
}
