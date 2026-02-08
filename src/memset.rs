//! Optimized memset with AVX2 and Non-Temporal dispatch
#![allow(unsafe_code)]

use core::arch::x86_64::*;

/// High-performance memset with automatic dispatch.
/// Uses SSE for small sizes to avoid AVX entry fee, AVX2 for medium,
/// and non-temporal stores for large buffers to avoid cache pollution.
///
/// # Safety
///
/// - `dest` must be valid for writes of `n` bytes
/// - AVX2 must be supported if the AVX2 path is taken
#[inline(always)]
pub unsafe fn optimized_memset_unified(dest: *mut u8, value: u8, n: usize) {
    if n <= 64 {
        if n > 0 {
            memset_sse_small(dest, value, n);
        }
        return;
    }
    optimized_memset_avx_dispatch(dest, value, n);
}

// =============================================================================
// SMALL PATH: SSE2 Implementation (0-64 bytes)
// =============================================================================

#[inline(always)]
unsafe fn memset_sse_small(dest: *mut u8, value: u8, n: usize) {
    if n >= 32 {
        // 32-64 bytes: use SSE 16-byte stores
        let v = _mm_set1_epi8(value as i8);
        _mm_storeu_si128(dest as *mut __m128i, v);
        _mm_storeu_si128(dest.add(16) as *mut __m128i, v);
        _mm_storeu_si128(dest.add(n - 32) as *mut __m128i, v);
        _mm_storeu_si128(dest.add(n - 16) as *mut __m128i, v);
        return;
    }

    if n >= 16 {
        let v64 = (value as u64) * 0x0101010101010101;
        core::ptr::write_unaligned(dest as *mut u64, v64);
        core::ptr::write_unaligned(dest.add(8) as *mut u64, v64);
        core::ptr::write_unaligned(dest.add(n - 16) as *mut u64, v64);
        core::ptr::write_unaligned(dest.add(n - 8) as *mut u64, v64);
        return;
    }

    if n >= 8 {
        let v64 = (value as u64) * 0x0101010101010101;
        core::ptr::write_unaligned(dest as *mut u64, v64);
        core::ptr::write_unaligned(dest.add(n - 8) as *mut u64, v64);
        return;
    }

    if n >= 4 {
        let v32 = (value as u32) * 0x01010101;
        core::ptr::write_unaligned(dest as *mut u32, v32);
        core::ptr::write_unaligned(dest.add(n - 4) as *mut u32, v32);
        return;
    }

    if n >= 2 {
        let v16 = (value as u16) | ((value as u16) << 8);
        core::ptr::write_unaligned(dest as *mut u16, v16);
        core::ptr::write_unaligned(dest.add(n - 2) as *mut u16, v16);
        return;
    }

    if n == 1 {
        *dest = value;
    }
}

// =============================================================================
// AVX DISPATCHER: Centralizes AVX state and manages VZEROUPPER
// =============================================================================

const NT_THRESHOLD: usize = 256 * 1024; // 256KB

#[target_feature(enable = "avx2")]
unsafe fn optimized_memset_avx_dispatch(dest: *mut u8, value: u8, n: usize) {
    if n < NT_THRESHOLD {
        optimized_memset_avx2(dest, value, n);
    } else {
        optimized_memset_avx2_nt(dest, value, n);
    }
}

// =============================================================================
// MEDIUM PATH: AVX2 Implementation (65B - 256KB)
// =============================================================================

#[target_feature(enable = "avx2")]
unsafe fn optimized_memset_avx2(dest: *mut u8, value: u8, n: usize) {
    let v = _mm256_set1_epi8(value as i8);

    if n <= 128 {
        _mm256_storeu_si256(dest as *mut __m256i, v);
        _mm256_storeu_si256(dest.add(32) as *mut __m256i, v);
        _mm256_storeu_si256(dest.add(n - 64) as *mut __m256i, v);
        _mm256_storeu_si256(dest.add(n - 32) as *mut __m256i, v);
        return;
    }

    if n <= 256 {
        _mm256_storeu_si256(dest as *mut __m256i, v);
        _mm256_storeu_si256(dest.add(32) as *mut __m256i, v);
        _mm256_storeu_si256(dest.add(64) as *mut __m256i, v);
        _mm256_storeu_si256(dest.add(96) as *mut __m256i, v);
        _mm256_storeu_si256(dest.add(n - 128) as *mut __m256i, v);
        _mm256_storeu_si256(dest.add(n - 96) as *mut __m256i, v);
        _mm256_storeu_si256(dest.add(n - 64) as *mut __m256i, v);
        _mm256_storeu_si256(dest.add(n - 32) as *mut __m256i, v);
        return;
    }

    // Large blocks (>256): Align and loop with 128-byte unroll
    _mm256_storeu_si256(dest as *mut __m256i, v);
    let offset = 32 - ((dest as usize) & 31);
    let mut ptr = dest.add(offset);
    let end = dest.add(n);
    let remaining = n - offset;
    let loop_end = ptr.add(remaining & !127);

    while ptr < loop_end {
        _mm256_store_si256(ptr as *mut __m256i, v);
        _mm256_store_si256(ptr.add(32) as *mut __m256i, v);
        _mm256_store_si256(ptr.add(64) as *mut __m256i, v);
        _mm256_store_si256(ptr.add(96) as *mut __m256i, v);
        ptr = ptr.add(128);
    }

    // Adaptive epilogue based on remainder
    let tail = remaining & 127;
    if tail > 64 {
        _mm256_storeu_si256(end.sub(128) as *mut __m256i, v);
        _mm256_storeu_si256(end.sub(96) as *mut __m256i, v);
    }
    _mm256_storeu_si256(end.sub(64) as *mut __m256i, v);
    _mm256_storeu_si256(end.sub(32) as *mut __m256i, v);
}

// =============================================================================
// LARGE PATH: Non-Temporal Stores (>256KB)
// =============================================================================

#[target_feature(enable = "avx2")]
unsafe fn optimized_memset_avx2_nt(dest: *mut u8, value: u8, n: usize) {
    let v = _mm256_set1_epi8(value as i8);

    // Alignment prologue
    _mm256_storeu_si256(dest as *mut __m256i, v);
    let offset = 32 - ((dest as usize) & 31);
    let mut ptr = dest.add(offset);
    let end = dest.add(n);
    let remaining = n - offset;
    let loop_end = ptr.add(remaining & !127);

    // Main loop: non-temporal stores (bypass cache)
    while ptr < loop_end {
        _mm256_stream_si256(ptr as *mut __m256i, v);
        _mm256_stream_si256(ptr.add(32) as *mut __m256i, v);
        _mm256_stream_si256(ptr.add(64) as *mut __m256i, v);
        _mm256_stream_si256(ptr.add(96) as *mut __m256i, v);
        ptr = ptr.add(128);
    }

    // REQUIRED: fence ensures NT stores are visible before function returns
    _mm_sfence();

    // Tail with regular stores
    let tail = remaining & 127;
    if tail > 64 {
        _mm256_storeu_si256(end.sub(128) as *mut __m256i, v);
        _mm256_storeu_si256(end.sub(96) as *mut __m256i, v);
    }
    _mm256_storeu_si256(end.sub(64) as *mut __m256i, v);
    _mm256_storeu_si256(end.sub(32) as *mut __m256i, v);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memset_optimized_0_to_1024() {
        let mut dst = [0u8; 1024];

        for n in 0..=1024 {
            dst.fill(0xFF);
            unsafe {
                optimized_memset_unified(dst.as_mut_ptr(), 0x42, n);
            }
            for (i, &byte) in dst[..n].iter().enumerate() {
                assert_eq!(byte, 0x42, "Failed at size {} index {}", n, i);
            }
            if n < 1024 {
                assert_eq!(dst[n], 0xFF, "Overwrote at size {} (index {})", n, n);
            }
        }
    }

    #[test]
    fn test_memset_optimized_alignment() {
        let mut dst_full = [0u8; 1100];

        // Test various alignments
        for dst_off in 0..32 {
            for n in [
                0, 1, 7, 8, 15, 16, 31, 32, 63, 64, 65, 127, 128, 129, 255, 256, 257, 511, 512,
                1024,
            ] {
                dst_full.fill(0xFF);
                unsafe {
                    optimized_memset_unified(dst_full.as_mut_ptr().add(dst_off), 0x42, n);
                }
                for (i, &byte) in dst_full[dst_off..dst_off + n].iter().enumerate() {
                    assert_eq!(
                        byte, 0x42,
                        "Failed at size {} with dst_off {} index {}",
                        n, dst_off, i
                    );
                }
            }
        }
    }

    #[test]
    fn test_memset_zero() {
        let mut dst = [0xFFu8; 256];
        unsafe {
            optimized_memset_unified(dst.as_mut_ptr(), 0, 256);
        }
        assert!(dst.iter().all(|&b| b == 0));
    }
}
