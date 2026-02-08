//! Optimized memmove with overlap-aware AVX2 paths.
#![allow(unsafe_code)]

use core::arch::asm;
use core::arch::x86_64::*;

use crate::memcpy::optimized_memcpy_unified;

const REP_MOVSB_FWD_THRESHOLD: usize = 256;
const REP_MOVSB_BWD_THRESHOLD: usize = 256;

/// High-performance memmove with automatic overlap handling.
///
/// # Safety
///
/// - `dest` and `src` must be valid for reads/writes of `n` bytes
/// - Regions may overlap
#[inline(always)]
pub unsafe fn optimized_memmove_unified(dest: *mut u8, src: *const u8, n: usize) -> *mut u8 {
    if n == 0 || core::ptr::eq(dest as *const u8, src) {
        return dest;
    }

    if n == 64 {
        memmove_64_avx2(dest, src);
        return dest;
    }

    if n <= 64 {
        memmove_small_overlap(dest, src, n);
        return dest;
    }

    let d = dest as usize;
    let s = src as usize;

    if d < s {
        // Non-overlap: defer to memcpy fast path.
        if s - d >= n {
            return optimized_memcpy_unified(dest, src, n);
        }

        // Overlap where dest < src: copy low-to-high.
        if n >= REP_MOVSB_FWD_THRESHOLD {
            rep_movsb_forward(dest, src, n);
        } else {
            memmove_forward_avx2(dest, src, n);
        }
        return dest;
    }

    // d > s here.
    // Non-overlap: defer to memcpy fast path.
    if d - s >= n {
        return optimized_memcpy_unified(dest, src, n);
    }

    // Overlap where dest > src: copy high-to-low.
    if n >= REP_MOVSB_BWD_THRESHOLD {
        rep_movsb_backward(dest, src, n);
    } else {
        memmove_backward_avx2(dest, src, n);
    }

    dest
}

#[target_feature(enable = "avx2")]
unsafe fn memmove_64_avx2(dest: *mut u8, src: *const u8) {
    let v0 = _mm256_loadu_si256(src as *const __m256i);
    let v1 = _mm256_loadu_si256(src.add(32) as *const __m256i);
    _mm256_storeu_si256(dest as *mut __m256i, v0);
    _mm256_storeu_si256(dest.add(32) as *mut __m256i, v1);
}

#[inline(always)]
unsafe fn rep_movsb_forward(dest: *mut u8, src: *const u8, n: usize) {
    let mut d = dest;
    let mut s = src;
    let mut c = n;
    asm!(
        "rep movsb",
        inout("rdi") d,
        inout("rsi") s,
        inout("rcx") c,
        options(nostack)
    );
}

#[inline(always)]
unsafe fn rep_movsb_backward(dest: *mut u8, src: *const u8, n: usize) {
    let mut d = dest.add(n - 1);
    let mut s = src.add(n - 1);
    let mut c = n;
    asm!(
        "std",
        "rep movsb",
        "cld",
        inout("rdi") d,
        inout("rsi") s,
        inout("rcx") c,
        options(nostack)
    );
}

#[inline(always)]
unsafe fn memmove_small_overlap(dest: *mut u8, src: *const u8, n: usize) {
    if n >= 32 {
        let v0 = _mm_loadu_si128(src as *const __m128i);
        let v1 = _mm_loadu_si128(src.add(16) as *const __m128i);
        let v2 = _mm_loadu_si128(src.add(n - 32) as *const __m128i);
        let v3 = _mm_loadu_si128(src.add(n - 16) as *const __m128i);
        _mm_storeu_si128(dest as *mut __m128i, v0);
        _mm_storeu_si128(dest.add(16) as *mut __m128i, v1);
        _mm_storeu_si128(dest.add(n - 32) as *mut __m128i, v2);
        _mm_storeu_si128(dest.add(n - 16) as *mut __m128i, v3);
        return;
    }

    if n >= 16 {
        let v0 = _mm_loadu_si128(src as *const __m128i);
        let v1 = _mm_loadu_si128(src.add(n - 16) as *const __m128i);
        _mm_storeu_si128(dest as *mut __m128i, v0);
        _mm_storeu_si128(dest.add(n - 16) as *mut __m128i, v1);
        return;
    }

    if n >= 8 {
        let a = core::ptr::read_unaligned(src as *const u64);
        let b = core::ptr::read_unaligned(src.add(n - 8) as *const u64);
        core::ptr::write_unaligned(dest as *mut u64, a);
        core::ptr::write_unaligned(dest.add(n - 8) as *mut u64, b);
        return;
    }

    if n >= 4 {
        let a = core::ptr::read_unaligned(src as *const u32);
        let b = core::ptr::read_unaligned(src.add(n - 4) as *const u32);
        core::ptr::write_unaligned(dest as *mut u32, a);
        core::ptr::write_unaligned(dest.add(n - 4) as *mut u32, b);
        return;
    }

    if n >= 2 {
        let a = core::ptr::read_unaligned(src as *const u16);
        let b = core::ptr::read_unaligned(src.add(n - 2) as *const u16);
        core::ptr::write_unaligned(dest as *mut u16, a);
        core::ptr::write_unaligned(dest.add(n - 2) as *mut u16, b);
        return;
    }

    if n == 1 {
        *dest = *src;
    }
}

#[target_feature(enable = "avx2")]
unsafe fn memmove_forward_avx2(dest: *mut u8, src: *const u8, n: usize) {
    let mut d = dest;
    let mut s = src;
    let mut rem = n;

    while rem >= 128 {
        let v0 = _mm256_loadu_si256(s as *const __m256i);
        let v1 = _mm256_loadu_si256(s.add(32) as *const __m256i);
        let v2 = _mm256_loadu_si256(s.add(64) as *const __m256i);
        let v3 = _mm256_loadu_si256(s.add(96) as *const __m256i);
        _mm256_storeu_si256(d as *mut __m256i, v0);
        _mm256_storeu_si256(d.add(32) as *mut __m256i, v1);
        _mm256_storeu_si256(d.add(64) as *mut __m256i, v2);
        _mm256_storeu_si256(d.add(96) as *mut __m256i, v3);
        d = d.add(128);
        s = s.add(128);
        rem -= 128;
    }

    while rem >= 32 {
        let v = _mm256_loadu_si256(s as *const __m256i);
        _mm256_storeu_si256(d as *mut __m256i, v);
        d = d.add(32);
        s = s.add(32);
        rem -= 32;
    }

    if rem > 0 {
        memmove_small_overlap(d, s, rem);
    }
}

#[target_feature(enable = "avx2")]
unsafe fn memmove_backward_avx2(dest: *mut u8, src: *const u8, n: usize) {
    let mut rem = n;

    while rem >= 128 {
        rem -= 128;
        let s = src.add(rem);
        let d = dest.add(rem);
        let v0 = _mm256_loadu_si256(s as *const __m256i);
        let v1 = _mm256_loadu_si256(s.add(32) as *const __m256i);
        let v2 = _mm256_loadu_si256(s.add(64) as *const __m256i);
        let v3 = _mm256_loadu_si256(s.add(96) as *const __m256i);
        _mm256_storeu_si256(d.add(96) as *mut __m256i, v3);
        _mm256_storeu_si256(d.add(64) as *mut __m256i, v2);
        _mm256_storeu_si256(d.add(32) as *mut __m256i, v1);
        _mm256_storeu_si256(d as *mut __m256i, v0);
    }

    while rem >= 32 {
        rem -= 32;
        let v = _mm256_loadu_si256(src.add(rem) as *const __m256i);
        _mm256_storeu_si256(dest.add(rem) as *mut __m256i, v);
    }

    if rem > 0 {
        memmove_small_overlap(dest, src, rem);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_buf() -> Vec<u8> {
        let mut buf = vec![0u8; 4096];
        for (i, b) in buf.iter_mut().enumerate() {
            *b = (i % 251) as u8;
        }
        buf
    }

    fn check_case(src_off: usize, dst_off: usize, n: usize) {
        let mut got = make_buf();
        let mut expected = got.clone();

        unsafe {
            optimized_memmove_unified(got.as_mut_ptr().add(dst_off), got.as_ptr().add(src_off), n);
            core::ptr::copy(
                expected.as_ptr().add(src_off),
                expected.as_mut_ptr().add(dst_off),
                n,
            );
        }

        assert_eq!(
            got, expected,
            "mismatch src_off={src_off} dst_off={dst_off} n={n}"
        );
    }

    #[test]
    fn test_memmove_non_overlap() {
        for n in [
            0usize, 1, 2, 3, 4, 7, 8, 15, 16, 31, 32, 63, 64, 65, 127, 128, 129, 255, 256, 257,
            511, 512, 513, 1023,
        ] {
            check_case(0, 2048, n);
            check_case(17, 2200, n);
        }
    }

    #[test]
    fn test_memmove_overlap_forward() {
        for n in [
            0usize, 1, 2, 3, 4, 7, 8, 15, 16, 31, 32, 63, 64, 65, 127, 128, 129, 255, 256, 257,
            511, 512, 513, 1023,
        ] {
            check_case(1, 0, n);
            check_case(7, 0, n);
            check_case(31, 0, n);
        }
    }

    #[test]
    fn test_memmove_overlap_backward() {
        for n in [
            0usize, 1, 2, 3, 4, 7, 8, 15, 16, 31, 32, 63, 64, 65, 127, 128, 129, 255, 256, 257,
            511, 512, 513, 1023,
        ] {
            check_case(0, 1, n);
            check_case(0, 7, n);
            check_case(0, 31, n);
        }
    }
}
