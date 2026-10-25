//! GPU usage, sampled by [`super::Usage`] about once per second.
//!
//! Each system measures it differently:
//! - Windows: Watershed's own use, from the "GPU Engine" performance counters
//!   (what Task Manager shows per process), for any vendor.
//! - Linux: Watershed's own use, from the DRM fdinfo files in `/proc` (what `nvtop`
//!   reads). amdgpu, i915, xe and most other drivers provide them; NVIDIA's
//!   proprietary driver does not, so there is no number there.
//! - macOS: the whole GPU, from IOKit (what Activity Monitor's GPU History shows),
//!   since macOS does not report GPU use per process.
//!
//! The number is the busiest engine's share of time: GPUs have separate engines
//! (3D, compute, copy, video), and the busiest one is what limits the work.

/// Whether the number covers every app using the GPU, not only Watershed.
pub const WHOLE_GPU: bool = cfg!(target_os = "macos");

pub struct Gpu(Option<imp::Gpu>);

impl Gpu {
    pub fn new() -> Self {
        Self(imp::Gpu::new())
    }

    /// Usage in percent since the previous call, or `None` if the system does not
    /// report it (or this is the first call and it needs a time span to measure).
    pub fn sample(&mut self) -> Option<f32> {
        self.0
            .as_mut()?
            .sample()
            .map(|percent| percent.clamp(0.0, 100.0))
    }
}

#[cfg(target_os = "macos")]
mod imp {
    use std::ffi::{CStr, c_char, c_void};

    type CFTypeRef = *const c_void;
    type IoObject = u32;

    const UTF8: u32 = 0x0800_0100; // kCFStringEncodingUTF8
    const SINT64: isize = 4; // kCFNumberSInt64Type

    #[link(name = "CoreFoundation", kind = "framework")]
    unsafe extern "C" {
        fn CFStringCreateWithCString(
            allocator: CFTypeRef,
            string: *const c_char,
            encoding: u32,
        ) -> CFTypeRef;
        fn CFDictionaryGetValue(dict: CFTypeRef, key: CFTypeRef) -> CFTypeRef;
        fn CFGetTypeID(cf: CFTypeRef) -> usize;
        fn CFDictionaryGetTypeID() -> usize;
        fn CFNumberGetTypeID() -> usize;
        fn CFNumberGetValue(number: CFTypeRef, kind: isize, value: *mut c_void) -> bool;
        fn CFRelease(cf: CFTypeRef);
    }

    #[link(name = "IOKit", kind = "framework")]
    unsafe extern "C" {
        fn IOServiceMatching(name: *const c_char) -> CFTypeRef;
        fn IOServiceGetMatchingServices(
            main_port: u32,
            matching: CFTypeRef,
            iterator: *mut IoObject,
        ) -> i32;
        fn IOIteratorNext(iterator: IoObject) -> IoObject;
        fn IOObjectRelease(object: IoObject) -> i32;
        fn IORegistryEntryCreateCFProperty(
            entry: IoObject,
            key: CFTypeRef,
            allocator: CFTypeRef,
            options: u32,
        ) -> CFTypeRef;
    }

    /// The GPUs' IOKit services, and the keys to read from them.
    pub struct Gpu {
        services: Vec<IoObject>,
        statistics: CFTypeRef,
        utilization: CFTypeRef,
    }

    impl Gpu {
        pub fn new() -> Option<Self> {
            let mut services = Vec::new();
            // SAFETY: the matching dictionary is consumed by the lookup; every
            // service the iterator returns is kept and released on drop.
            unsafe {
                let matching = IOServiceMatching(c"IOAccelerator".as_ptr());
                let mut iterator = 0;
                if matching.is_null()
                    || IOServiceGetMatchingServices(0, matching, &mut iterator) != 0
                {
                    return None;
                }
                loop {
                    let service = IOIteratorNext(iterator);
                    if service == 0 {
                        break;
                    }
                    services.push(service);
                }
                IOObjectRelease(iterator);
            }
            let gpu = Self {
                services,
                statistics: string(c"PerformanceStatistics"),
                utilization: string(c"Device Utilization %"),
            };
            (!gpu.services.is_empty() && !gpu.statistics.is_null() && !gpu.utilization.is_null())
                .then_some(gpu)
        }

        /// The driver's own recent average, so no previous sample is needed.
        pub fn sample(&mut self) -> Option<f32> {
            self.services
                .iter()
                .filter_map(|&service| self.read(service))
                .reduce(f32::max)
        }

        fn read(&self, service: IoObject) -> Option<f32> {
            // SAFETY: the property is released before returning; the value inside
            // it is only read while it is alive, after checking its types.
            unsafe {
                let statistics =
                    IORegistryEntryCreateCFProperty(service, self.statistics, std::ptr::null(), 0);
                if statistics.is_null() {
                    return None;
                }
                let mut percent: i64 = 0;
                let found = CFGetTypeID(statistics) == CFDictionaryGetTypeID() && {
                    let value = CFDictionaryGetValue(statistics, self.utilization);
                    !value.is_null()
                        && CFGetTypeID(value) == CFNumberGetTypeID()
                        && CFNumberGetValue(value, SINT64, (&raw mut percent).cast())
                };
                CFRelease(statistics);
                found.then_some(percent as f32)
            }
        }
    }

    impl Drop for Gpu {
        fn drop(&mut self) {
            // SAFETY: each was obtained once and is released once.
            unsafe {
                for &service in &self.services {
                    IOObjectRelease(service);
                }
                for key in [self.statistics, self.utilization] {
                    if !key.is_null() {
                        CFRelease(key);
                    }
                }
            }
        }
    }

    fn string(text: &CStr) -> CFTypeRef {
        // SAFETY: copies a valid C string; null on failure, checked by the caller.
        unsafe { CFStringCreateWithCString(std::ptr::null(), text.as_ptr(), UTF8) }
    }
}

#[cfg(windows)]
mod imp {
    use std::ptr::{null, null_mut};

    use windows_sys::Win32::System::Performance::{
        PDH_CSTATUS_NEW_DATA, PDH_CSTATUS_NO_INSTANCE, PDH_FMT_COUNTERVALUE_ITEM_W, PDH_FMT_DOUBLE,
        PDH_HCOUNTER, PDH_HQUERY, PDH_MORE_DATA, PDH_NO_DATA, PdhAddEnglishCounterW, PdhCloseQuery,
        PdhCollectQueryData, PdhGetFormattedCounterArrayW, PdhOpenQueryW,
    };

    /// A query on this process's instances of the "GPU Engine" counters: one per
    /// engine it has used, named like `pid_1234_luid_..._eng_0_engtype_3D`.
    pub struct Gpu {
        query: PDH_HQUERY,
        counter: PDH_HCOUNTER,
    }

    impl Gpu {
        pub fn new() -> Option<Self> {
            let path: Vec<u16> = format!(
                "\\GPU Engine(pid_{}_*)\\Utilization Percentage",
                std::process::id()
            )
            .encode_utf16()
            .chain([0])
            .collect();
            let mut query = null_mut();
            let mut counter = null_mut();
            // SAFETY: plain PDH calls on handles this struct owns; the query is
            // closed on failure or on drop.
            unsafe {
                if PdhOpenQueryW(null(), 0, &mut query) != 0 {
                    return None;
                }
                if PdhAddEnglishCounterW(query, path.as_ptr(), 0, &mut counter) != 0 {
                    PdhCloseQuery(query);
                    return None;
                }
                // Rates need a first data point to compare against.
                PdhCollectQueryData(query);
            }
            Some(Self { query, counter })
        }

        pub fn sample(&mut self) -> Option<f32> {
            // SAFETY: the buffer is sized as PDH asks, aligned for the items it
            // holds (their names point into it too), and outlives the slice.
            unsafe {
                if PdhCollectQueryData(self.query) != 0 {
                    return None;
                }
                let (mut size, mut count) = (0, 0);
                let status = PdhGetFormattedCounterArrayW(
                    self.counter,
                    PDH_FMT_DOUBLE,
                    &mut size,
                    &mut count,
                    null_mut(),
                );
                match status {
                    PDH_MORE_DATA => {}
                    // No engine used yet.
                    0 | PDH_CSTATUS_NO_INSTANCE | PDH_NO_DATA => return Some(0.0),
                    _ => return None,
                }
                let mut buffer = vec![0u64; (size as usize).div_ceil(8)];
                let items = buffer.as_mut_ptr().cast::<PDH_FMT_COUNTERVALUE_ITEM_W>();
                if PdhGetFormattedCounterArrayW(
                    self.counter,
                    PDH_FMT_DOUBLE,
                    &mut size,
                    &mut count,
                    items,
                ) != 0
                {
                    return None;
                }
                let busiest = std::slice::from_raw_parts(items, count as usize)
                    .iter()
                    .filter(|item| item.FmtValue.CStatus <= PDH_CSTATUS_NEW_DATA)
                    .map(|item| item.FmtValue.Anonymous.doubleValue)
                    .fold(0.0, f64::max);
                Some(busiest as f32)
            }
        }
    }

    impl Drop for Gpu {
        fn drop(&mut self) {
            // SAFETY: closing the query also frees its counter.
            unsafe { PdhCloseQuery(self.query) };
        }
    }
}

#[cfg(target_os = "linux")]
mod imp {
    use std::time::Instant;

    use super::fdinfo::Times;

    pub struct Gpu {
        last: Option<(Instant, Times)>,
    }

    impl Gpu {
        pub fn new() -> Option<Self> {
            let mut gpu = Self { last: None };
            gpu.sample();
            Some(gpu)
        }

        pub fn sample(&mut self) -> Option<f32> {
            let now = Instant::now();
            let times = read()?;
            let percent = self
                .last
                .as_ref()
                .map(|(then, before)| before.percent(&times, now - *then));
            self.last = Some((now, times));
            percent
        }
    }

    /// The engine times of every GPU client this process has open, or `None` if
    /// the driver reports none.
    fn read() -> Option<Times> {
        let mut times = Times::default();
        for entry in std::fs::read_dir("/proc/self/fdinfo").ok()?.flatten() {
            if let Ok(text) = std::fs::read_to_string(entry.path()) {
                times.add(&text);
            }
        }
        (!times.is_empty()).then_some(times)
    }
}

#[cfg(not(any(target_os = "macos", windows, target_os = "linux")))]
mod imp {
    pub struct Gpu;

    impl Gpu {
        pub fn new() -> Option<Self> {
            None
        }

        pub fn sample(&mut self) -> Option<f32> {
            None
        }
    }
}

/// Parsing of Linux's DRM fdinfo files, kept apart (and built everywhere) so its
/// tests run on any system.
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
mod fdinfo {
    use std::collections::HashMap;
    use std::time::Duration;

    /// Busy time of each engine of each GPU client (an open device, possibly
    /// behind several file descriptors).
    #[derive(Debug, Default)]
    pub struct Times {
        /// (device, client id, engine) → (busy nanoseconds, engines of that kind).
        engines: HashMap<(String, String, String), (u64, u64)>,
    }

    impl Times {
        /// Adds the client described by one fdinfo file; files of other kinds
        /// have no `drm-client-id` and are skipped.
        pub fn add(&mut self, text: &str) {
            let mut device = "";
            let mut client = None;
            let mut busy = Vec::new();
            let mut capacity = HashMap::new();
            for line in text.lines() {
                let Some((key, value)) = line.split_once(':') else {
                    continue;
                };
                let value = value.trim();
                match key {
                    "drm-pdev" => device = value,
                    "drm-client-id" => client = Some(value),
                    _ => {
                        if let Some(engine) = key.strip_prefix("drm-engine-capacity-") {
                            capacity.insert(engine, value.parse().unwrap_or(1));
                        } else if let Some(engine) = key.strip_prefix("drm-engine-")
                            && let Some(ns) = value.strip_suffix("ns")
                            && let Ok(ns) = ns.trim().parse()
                        {
                            busy.push((engine, ns));
                        }
                    }
                }
            }
            let Some(client) = client else {
                return;
            };
            for (engine, ns) in busy {
                let capacity = capacity.get(engine).copied().unwrap_or(1u64).max(1);
                let key = (device.to_owned(), client.to_owned(), engine.to_owned());
                self.engines.insert(key, (ns, capacity));
            }
        }

        pub fn is_empty(&self) -> bool {
            self.engines.is_empty()
        }

        /// The busiest engine's share of `elapsed` between `self` and `later`, in
        /// percent, adding up all clients on the same device.
        pub fn percent(&self, later: &Times, elapsed: Duration) -> f32 {
            let elapsed = elapsed.as_nanos().max(1) as f64;
            let mut shares: HashMap<(&str, &str), f64> = HashMap::new();
            for (key, &(ns, capacity)) in &later.engines {
                // A client opened since the last sample counts from now on.
                let before = self.engines.get(key).map_or(ns, |&(before, _)| before);
                let share = ns.saturating_sub(before) as f64 / (elapsed * capacity as f64);
                *shares.entry((&key.0, &key.2)).or_default() += share;
            }
            (shares.into_values().fold(0.0, f64::max) * 100.0) as f32
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        fn client(id: u32, gfx_ns: u64, video_ns: u64) -> String {
            format!(
                "pos:\t0\nflags:\t02100002\ndrm-driver:\tamdgpu\ndrm-pdev:\t0000:03:00.0\n\
                 drm-client-id:\t{id}\ndrm-engine-gfx:\t{gfx_ns} ns\n\
                 drm-engine-dec:\t{video_ns} ns\ndrm-memory-vram:\t1024 KiB\n"
            )
        }

        fn times(files: &[String]) -> Times {
            let mut times = Times::default();
            for file in files {
                times.add(file);
            }
            times
        }

        const SECOND: Duration = Duration::from_secs(1);

        #[test]
        fn busiest_engine_over_the_interval() {
            let before = times(&[client(1, 1_000, 0)]);
            let after = times(&[client(1, 500_001_000, 100_000_000)]);
            assert_eq!(before.percent(&after, SECOND), 50.0);
        }

        #[test]
        fn clients_add_up_and_shared_ones_count_once() {
            let before = times(&[client(1, 0, 0), client(2, 0, 0)]);
            // Client 1 behind two file descriptors.
            let after = times(&[
                client(1, 200_000_000, 0),
                client(1, 200_000_000, 0),
                client(2, 300_000_000, 0),
            ]);
            assert_eq!(before.percent(&after, SECOND), 50.0);
        }

        #[test]
        fn engines_with_several_instances_and_new_clients() {
            let file = "drm-pdev: 0000:00:02.0\ndrm-client-id: 7\n\
                        drm-engine-render: 0 ns\ndrm-engine-capacity-render: 2\n";
            let before = times(&[file.to_owned()]);
            let after = times(&[
                file.replace("render: 0 ns", "render: 1000000000 ns"),
                client(9, 900_000_000, 0),
            ]);
            assert_eq!(before.percent(&after, SECOND), 50.0);
        }

        #[test]
        fn other_files_are_skipped() {
            assert!(times(&["pos:\t0\nflags:\t02\nmnt_id:\t24\n".to_owned()]).is_empty());
        }
    }
}
