//! macOS: hands large blocks (song buffers) straight to the kernel.
//!
//! macOS's malloc keeps large blocks after they are freed, for reuse, and they
//! keep counting as the app's memory (Activity Monitor, the RAM display) until
//! the system is short of memory. Loading or separating a song frees several
//! full-song buffers, so the app looked hundreds of MB bigger than it was. Blocks
//! of [`LARGE`] bytes or more are mapped and unmapped directly instead, which
//! returns them at once. Smaller ones go to the system allocator as usual. The
//! audio callback never allocates, so this never runs on the audio thread.

use std::alloc::{GlobalAlloc, Layout, System};

/// Blocks from this size on bypass malloc: above what the UI allocates per frame,
/// and small enough to catch the buffers that grow while a stem is decoded
/// (a stem takes ~10 MB per minute). 1 MiB saved little more.
const LARGE: usize = 4 << 20;

pub struct Allocator;

fn is_large(layout: &Layout) -> bool {
    layout.size() >= LARGE
}

// SAFETY: large blocks come from `mmap`, which returns page-aligned (16 KiB on
// Apple Silicon), zeroed memory, enough for any alignment Rust types use below a
// page; larger alignments go to the system allocator. Whether a block is large
// only depends on its layout, which `dealloc` and `realloc` receive unchanged.
unsafe impl GlobalAlloc for Allocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if is_large(&layout) && layout.align() <= page_size() {
            map(layout.size())
        } else {
            // SAFETY: forwarded unchanged.
            unsafe { System.alloc(layout) }
        }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        if is_large(&layout) && layout.align() <= page_size() {
            // Fresh mappings are already zeroed.
            map(layout.size())
        } else {
            // SAFETY: forwarded unchanged.
            unsafe { System.alloc_zeroed(layout) }
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        if is_large(&layout) && layout.align() <= page_size() {
            // SAFETY: `ptr` came from `map` with this size.
            unsafe { libc::munmap(ptr.cast(), layout.size()) };
        } else {
            // SAFETY: forwarded unchanged.
            unsafe { System.dealloc(ptr, layout) }
        }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        // SAFETY: the caller guarantees a valid size for this alignment.
        let new_layout = unsafe { Layout::from_size_align_unchecked(new_size, layout.align()) };
        if !is_large(&layout) && !is_large(&new_layout) {
            // SAFETY: both small, so both belong to the system allocator.
            return unsafe { System.realloc(ptr, layout, new_size) };
        }
        // SAFETY: as in `alloc` and `dealloc`; the copy stays within both blocks.
        unsafe {
            let new = self.alloc(new_layout);
            if !new.is_null() {
                std::ptr::copy_nonoverlapping(ptr, new, layout.size().min(new_size));
                self.dealloc(ptr, layout);
            }
            new
        }
    }
}

fn map(size: usize) -> *mut u8 {
    // SAFETY: an anonymous private mapping touches no existing memory.
    let ptr = unsafe {
        libc::mmap(
            std::ptr::null_mut(),
            size,
            libc::PROT_READ | libc::PROT_WRITE,
            libc::MAP_PRIVATE | libc::MAP_ANON,
            -1,
            0,
        )
    };
    if ptr == libc::MAP_FAILED {
        std::ptr::null_mut()
    } else {
        ptr.cast()
    }
}

fn page_size() -> usize {
    // SAFETY: no arguments, no side effects.
    unsafe { libc::vm_page_size }
}

#[cfg(test)]
mod tests {
    #[test]
    fn large_vectors_grow_shrink_and_free() {
        let mut v: Vec<u32> = Vec::with_capacity(1000);
        v.extend(0..1000);
        // Small to large, large to larger, then back to small.
        v.extend(1000..10_000_000);
        v.extend(10_000_000..12_000_000);
        assert!(v.iter().enumerate().all(|(i, &x)| x == i as u32));
        v.truncate(100);
        v.shrink_to_fit();
        assert!(v.iter().enumerate().all(|(i, &x)| x == i as u32));
        let zeroed = vec![0u64; 5_000_000];
        assert!(zeroed.iter().all(|&x| x == 0));
    }
}
