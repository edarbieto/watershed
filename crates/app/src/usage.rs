//! CPU and memory used by the Watershed process, sampled about once per second.
//! Operating systems measure this per process, which includes all its windows and
//! threads (UI, audio, loading).

use std::time::{Duration, Instant};

use sysinfo::{MemoryRefreshKind, Pid, ProcessRefreshKind, ProcessesToUpdate, System};

/// How often the numbers are refreshed. CPU usage is averaged over this interval.
pub const INTERVAL: Duration = Duration::from_secs(1);

/// The latest sample.
#[derive(Clone, Copy, Debug, Default)]
pub struct Sample {
    /// Share of the whole machine (all cores), in percent.
    pub cpu_percent: f32,
    /// Resident memory, in bytes.
    pub memory: u64,
    /// Share of the installed memory, in percent.
    pub memory_percent: f32,
}

pub struct Usage {
    system: System,
    pid: Option<Pid>,
    cores: usize,
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
        if first {
            return;
        }
        let Some(process) = self.system.process(pid) else {
            return;
        };
        let total = self.system.total_memory().max(1);
        self.sample = Some(Sample {
            // sysinfo reports a share of one core, up to 100 % per core.
            cpu_percent: process.cpu_usage() / self.cores as f32,
            memory: process.memory(),
            memory_percent: process.memory() as f32 / total as f32 * 100.0,
        });
    }
}
