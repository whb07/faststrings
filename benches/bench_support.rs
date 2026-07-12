use core::arch::x86_64::{_mm_clflush, _mm_mfence};
use std::alloc::{Layout, alloc, dealloc, handle_alloc_error};
use std::ptr::NonNull;

pub struct AlignedBuffer {
    ptr: NonNull<u8>,
    len: usize,
    align: usize,
}

impl AlignedBuffer {
    pub fn new(len: usize, align: usize) -> Self {
        let layout = Layout::from_size_align(len.max(1), align).unwrap();
        let ptr =
            NonNull::new(unsafe { alloc(layout) }).unwrap_or_else(|| handle_alloc_error(layout));
        let mut value = Self {
            ptr,
            len: len.max(1),
            align,
        };
        for i in 0..value.len {
            unsafe { value.ptr.as_ptr().add(i).write((i % 251) as u8) };
        }
        value
    }

    #[inline]
    pub fn ptr(&self, offset: usize) -> *const u8 {
        assert!(offset <= self.len);
        unsafe { self.ptr.as_ptr().add(offset) }
    }

    #[inline]
    pub fn mut_ptr(&mut self, offset: usize) -> *mut u8 {
        assert!(offset <= self.len);
        unsafe { self.ptr.as_ptr().add(offset) }
    }
}

impl Drop for AlignedBuffer {
    fn drop(&mut self) {
        unsafe {
            dealloc(
                self.ptr.as_ptr(),
                Layout::from_size_align_unchecked(self.len, self.align),
            );
        }
    }
}

/// Evict every cache line intersecting this range. The fence ensures eviction
/// completes before Criterion starts timing the operation.
#[allow(dead_code)]
#[inline(never)]
pub unsafe fn flush_range(ptr: *const u8, len: usize) {
    let start = (ptr as usize) & !63;
    let end = (ptr as usize).saturating_add(len);
    let mut line = start;
    while line < end {
        unsafe { _mm_clflush(line as *const u8) };
        line += 64;
    }
    unsafe { _mm_mfence() };
}
