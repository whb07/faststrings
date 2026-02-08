//! Optimized memcpy with AVX2 and Non-Temporal dispatch
#![allow(unsafe_code)]

use core::arch::x86_64::*;

/// High-performance memcpy with automatic dispatch.
/// This entry point is NOT marked with AVX2 to ensure that 0-64 byte copies
/// never trigger AVX power-up latency (the "AVX Entry Fee").
///
/// # Safety
///
/// - `dest` and `src` must be valid for reads/writes of `n` bytes
/// - The memory regions must not overlap
/// - AVX2 must be supported if the AVX2 path is taken
#[inline(always)]
pub unsafe fn optimized_memcpy_unified(dest: *mut u8, src: *const u8, n: usize) -> *mut u8 {
    if n <= 64 {
        // SSE/Scalar path: Legacy SSE encoding, no transition penalty.
        // Handles up to 64 bytes to avoid AVX entry fee for cache-line sized moves.
        optimized_memcpy_sse_small(dest, src, n)
    } else {
        // AVX path: Dispatches to specialized AVX2/NT logic.
        optimized_memcpy_avx_dispatch(dest, src, n)
    }
}

// =============================================================================
// SMALL PATH: SSE2 Implementation (0-64 bytes)
// =============================================================================

#[inline(always)]
unsafe fn optimized_memcpy_sse_small(dest: *mut u8, src: *const u8, n: usize) -> *mut u8 {
    // n < 64 guaranteed by dispatch

    if n >= 32 {
        // 32-63 bytes: 4 × 16-byte loads/stores (overlapping)
        // SAFETY: Unaligned loads are valid for any alignment; caller guarantees
        // `src` is readable for `n` bytes and non-overlapping with `dest`.
        let v0 = _mm_loadu_si128(src as *const __m128i);
        let v1 = _mm_loadu_si128(src.add(16) as *const __m128i);
        let v2 = _mm_loadu_si128(src.add(n - 32) as *const __m128i);
        let v3 = _mm_loadu_si128(src.add(n - 16) as *const __m128i);

        // SAFETY: Unaligned stores are valid for any alignment; caller guarantees
        // `dest` is writable for `n` bytes and non-overlapping with `src`.
        _mm_storeu_si128(dest as *mut __m128i, v0);
        _mm_storeu_si128(dest.add(16) as *mut __m128i, v1);
        _mm_storeu_si128(dest.add(n - 32) as *mut __m128i, v2);
        _mm_storeu_si128(dest.add(n - 16) as *mut __m128i, v3);
        return dest;
    }

    if n >= 16 {
        // 16-31 bytes: 2 × 16-byte loads/stores (overlapping)
        // SAFETY: Unaligned loads/stores are valid for any alignment; caller
        // guarantees `src`/`dest` are valid for `n` bytes and non-overlapping.
        let v0 = _mm_loadu_si128(src as *const __m128i);
        let v1 = _mm_loadu_si128(src.add(n - 16) as *const __m128i);
        _mm_storeu_si128(dest as *mut __m128i, v0);
        _mm_storeu_si128(dest.add(n - 16) as *mut __m128i, v1);
        return dest;
    }

    if n >= 8 {
        // 8-15 bytes: 2 × 8-byte loads/stores (overlapping)
        // SAFETY: read_unaligned/write_unaligned allow any alignment; caller
        // guarantees `src` is readable and `dest` writable for `n` bytes.
        let a = core::ptr::read_unaligned(src as *const u64);
        let b = core::ptr::read_unaligned(src.add(n - 8) as *const u64);
        core::ptr::write_unaligned(dest as *mut u64, a);
        core::ptr::write_unaligned(dest.add(n - 8) as *mut u64, b);
        return dest;
    }

    if n >= 4 {
        // 4-7 bytes: 2 × 4-byte loads/stores (overlapping)
        // SAFETY: read_unaligned/write_unaligned allow any alignment; caller
        // guarantees `src` is readable and `dest` writable for `n` bytes.
        let a = core::ptr::read_unaligned(src as *const u32);
        let b = core::ptr::read_unaligned(src.add(n - 4) as *const u32);
        core::ptr::write_unaligned(dest as *mut u32, a);
        core::ptr::write_unaligned(dest.add(n - 4) as *mut u32, b);
        return dest;
    }

    if n >= 2 {
        // 2-3 bytes: 2 × 2-byte loads/stores (overlapping)
        // SAFETY: read_unaligned/write_unaligned allow any alignment; caller
        // guarantees `src` is readable and `dest` writable for `n` bytes.
        let a = core::ptr::read_unaligned(src as *const u16);
        let b = core::ptr::read_unaligned(src.add(n - 2) as *const u16);
        core::ptr::write_unaligned(dest as *mut u16, a);
        core::ptr::write_unaligned(dest.add(n - 2) as *mut u16, b);
        return dest;
    }

    if n == 1 {
        *dest = *src;
    }

    dest
}

#[target_feature(enable = "avx2")]
unsafe fn copy_tail_avx2(mut d: *mut u8, mut s: *const u8, mut rem: usize) {
    while rem >= 32 {
        let v = _mm256_loadu_si256(s as *const __m256i);
        _mm256_storeu_si256(d as *mut __m256i, v);
        d = d.add(32);
        s = s.add(32);
        rem -= 32;
    }

    if rem >= 16 {
        let v = _mm_loadu_si128(s as *const __m128i);
        _mm_storeu_si128(d as *mut __m128i, v);
        d = d.add(16);
        s = s.add(16);
        rem -= 16;
    }

    if rem >= 8 {
        let v = core::ptr::read_unaligned(s as *const u64);
        core::ptr::write_unaligned(d as *mut u64, v);
        d = d.add(8);
        s = s.add(8);
        rem -= 8;
    }

    if rem >= 4 {
        let v = core::ptr::read_unaligned(s as *const u32);
        core::ptr::write_unaligned(d as *mut u32, v);
        d = d.add(4);
        s = s.add(4);
        rem -= 4;
    }

    if rem >= 2 {
        let v = core::ptr::read_unaligned(s as *const u16);
        core::ptr::write_unaligned(d as *mut u16, v);
        d = d.add(2);
        s = s.add(2);
        rem -= 2;
    }

    if rem == 1 {
        *d = *s;
    }
}

#[target_feature(enable = "avx2")]
unsafe fn copy_256_avx2(d: *mut u8, s: *const u8) {
    let v0 = _mm256_loadu_si256(s as *const __m256i);
    let v1 = _mm256_loadu_si256(s.add(32) as *const __m256i);
    let v2 = _mm256_loadu_si256(s.add(64) as *const __m256i);
    let v3 = _mm256_loadu_si256(s.add(96) as *const __m256i);
    let v4 = _mm256_loadu_si256(s.add(128) as *const __m256i);
    let v5 = _mm256_loadu_si256(s.add(160) as *const __m256i);
    let v6 = _mm256_loadu_si256(s.add(192) as *const __m256i);
    let v7 = _mm256_loadu_si256(s.add(224) as *const __m256i);
    _mm256_storeu_si256(d as *mut __m256i, v0);
    _mm256_storeu_si256(d.add(32) as *mut __m256i, v1);
    _mm256_storeu_si256(d.add(64) as *mut __m256i, v2);
    _mm256_storeu_si256(d.add(96) as *mut __m256i, v3);
    _mm256_storeu_si256(d.add(128) as *mut __m256i, v4);
    _mm256_storeu_si256(d.add(160) as *mut __m256i, v5);
    _mm256_storeu_si256(d.add(192) as *mut __m256i, v6);
    _mm256_storeu_si256(d.add(224) as *mut __m256i, v7);
}

// =============================================================================
// AVX DISPATCHER: Centralizes AVX state and manages VZEROUPPER
// =============================================================================

// Non-temporal stores regress around the multi-MiB transition on current
// targets; keep NT for very large copies only.
const NT_THRESHOLD: usize = 16 * 1024 * 1024;

#[target_feature(enable = "avx2")]
unsafe fn optimized_memcpy_avx_dispatch(dest: *mut u8, src: *const u8, n: usize) -> *mut u8 {
    if n < NT_THRESHOLD {
        optimized_memcpy_avx2_unaligned(dest, src, n);
    } else {
        optimized_memcpy_avx2_nt(dest, src, n);
    }

    dest
}

// =============================================================================
// MEDIUM PATH: AVX2 Unaligned Overlapping
// =============================================================================

#[target_feature(enable = "avx2")]
unsafe fn optimized_memcpy_avx2_unaligned(dest: *mut u8, src: *const u8, n: usize) -> *mut u8 {
    // 1. TINY/MEDIUM PATH: Branchless overlapping for 65B-128B.
    if n <= 128 {
        // SAFETY: Unaligned AVX loads/stores are valid for any alignment; caller
        // guarantees `src`/`dest` are valid for `n` bytes and non-overlapping.
        let v0 = _mm256_loadu_si256(src as *const __m256i);
        let v1 = _mm256_loadu_si256(src.add(32) as *const __m256i);
        let v2 = _mm256_loadu_si256(src.add(n - 64) as *const __m256i);
        let v3 = _mm256_loadu_si256(src.add(n - 32) as *const __m256i);

        _mm256_storeu_si256(dest as *mut __m256i, v0);
        _mm256_storeu_si256(dest.add(32) as *mut __m256i, v1);
        _mm256_storeu_si256(dest.add(n - 64) as *mut __m256i, v2);
        _mm256_storeu_si256(dest.add(n - 32) as *mut __m256i, v3);
        return dest;
    }

    // 2. INTERMEDIATE PATH: Branchless overlap for 129B-256B.
    if n <= 256 {
        // Head 128B
        let h0 = _mm256_loadu_si256(src as *const __m256i);
        let h1 = _mm256_loadu_si256(src.add(32) as *const __m256i);
        let h2 = _mm256_loadu_si256(src.add(64) as *const __m256i);
        let h3 = _mm256_loadu_si256(src.add(96) as *const __m256i);
        _mm256_storeu_si256(dest as *mut __m256i, h0);
        _mm256_storeu_si256(dest.add(32) as *mut __m256i, h1);
        _mm256_storeu_si256(dest.add(64) as *mut __m256i, h2);
        _mm256_storeu_si256(dest.add(96) as *mut __m256i, h3);

        // Tail 128B
        let ts = src.add(n - 128);
        let td = dest.add(n - 128);
        let t0 = _mm256_loadu_si256(ts as *const __m256i);
        let t1 = _mm256_loadu_si256(ts.add(32) as *const __m256i);
        let t2 = _mm256_loadu_si256(ts.add(64) as *const __m256i);
        let t3 = _mm256_loadu_si256(ts.add(96) as *const __m256i);
        _mm256_storeu_si256(td as *mut __m256i, t0);
        _mm256_storeu_si256(td.add(32) as *mut __m256i, t1);
        _mm256_storeu_si256(td.add(64) as *mut __m256i, t2);
        _mm256_storeu_si256(td.add(96) as *mut __m256i, t3);
        return dest;
    }

    // 3. INTERMEDIATE PATH: Branchless overlap for 257B-512B.
    if n <= 512 {
        if n == 512 {
            copy_256_avx2(dest, src);
            copy_256_avx2(dest.add(256), src.add(256));
            return dest;
        }

        // Head 256B
        copy_256_avx2(dest, src);

        // Remainder (1..=256B)
        let rem = n - 256;
        copy_tail_avx2(dest.add(256), src.add(256), rem);
        return dest;
    }

    // 4. LARGE-NEAR PATH: Unaligned loop for 513B-1024B.
    // Avoids alignment-prologue overhead in this transition zone.
    if n <= 1024 {
        let mut d = dest;
        let mut s = src;
        let mut rem = n;
        while rem >= 256 {
            copy_256_avx2(d, s);
            d = d.add(256);
            s = s.add(256);
            rem -= 256;
        }
        if rem > 0 {
            copy_tail_avx2(d, s, rem);
        }
        return dest;
    }

    // 5. LARGE PATH: Aligned loop for n > 1024.
    let mut d = dest;
    let mut s = src;
    let mut rem = n;

    // Alignment prologue
    let misalign = (d as usize) & 31;
    if misalign != 0 {
        let advance = 32 - misalign;
        let first_v = _mm256_loadu_si256(s as *const __m256i);
        _mm256_storeu_si256(d as *mut __m256i, first_v);
        d = d.add(advance);
        s = s.add(advance);
        rem -= advance;
    }

    // Main loop with aligned stores (256B unroll)
    while rem >= 256 {
        // SAFETY: Unaligned loads are valid for any alignment; caller guarantees
        // `src` is readable for the loop span.
        let v0 = _mm256_loadu_si256(s as *const __m256i);
        let v1 = _mm256_loadu_si256(s.add(32) as *const __m256i);
        let v2 = _mm256_loadu_si256(s.add(64) as *const __m256i);
        let v3 = _mm256_loadu_si256(s.add(96) as *const __m256i);
        let v4 = _mm256_loadu_si256(s.add(128) as *const __m256i);
        let v5 = _mm256_loadu_si256(s.add(160) as *const __m256i);
        let v6 = _mm256_loadu_si256(s.add(192) as *const __m256i);
        let v7 = _mm256_loadu_si256(s.add(224) as *const __m256i);

        // SAFETY: Aligned stores require 32-byte alignment; `d` is aligned by
        // the prologue and advances in 32-byte multiples.
        _mm256_store_si256(d as *mut __m256i, v0);
        _mm256_store_si256(d.add(32) as *mut __m256i, v1);
        _mm256_store_si256(d.add(64) as *mut __m256i, v2);
        _mm256_store_si256(d.add(96) as *mut __m256i, v3);
        _mm256_store_si256(d.add(128) as *mut __m256i, v4);
        _mm256_store_si256(d.add(160) as *mut __m256i, v5);
        _mm256_store_si256(d.add(192) as *mut __m256i, v6);
        _mm256_store_si256(d.add(224) as *mut __m256i, v7);

        d = d.add(256);
        s = s.add(256);
        rem -= 256;
    }

    // Sequential tail
    if rem > 0 {
        copy_tail_avx2(d, s, rem);
    }
    dest
}

#[target_feature(enable = "avx2")]
unsafe fn optimized_memcpy_avx2_nt(dest: *mut u8, src: *const u8, n: usize) -> *mut u8 {
    let mut d = dest;
    let mut s = src;
    let mut rem = n;

    // Alignment prologue (NT stores require 32-byte alignment)
    let misalign = (d as usize) & 31;
    if misalign != 0 {
        let advance = 32 - misalign;
        // SAFETY: Unaligned load/store are valid for any alignment; caller
        // guarantees `src`/`dest` are valid for `n` bytes.
        let v = _mm256_loadu_si256(s as *const __m256i);
        _mm256_storeu_si256(d as *mut __m256i, v);
        d = d.add(advance);
        s = s.add(advance);
        rem -= advance;
    }

    // Main loop: non-temporal stores (bypass cache)
    while rem >= 128 {
        // SAFETY: Unaligned loads are valid for any alignment; caller guarantees
        // `src` is readable for the loop span.
        let v0 = _mm256_loadu_si256(s as *const __m256i);
        let v1 = _mm256_loadu_si256(s.add(32) as *const __m256i);
        let v2 = _mm256_loadu_si256(s.add(64) as *const __m256i);
        let v3 = _mm256_loadu_si256(s.add(96) as *const __m256i);

        // SAFETY: Non-temporal stores require 32-byte alignment; `d` is aligned
        // by the prologue and advances in 32-byte multiples.
        _mm256_stream_si256(d as *mut __m256i, v0);
        _mm256_stream_si256(d.add(32) as *mut __m256i, v1);
        _mm256_stream_si256(d.add(64) as *mut __m256i, v2);
        _mm256_stream_si256(d.add(96) as *mut __m256i, v3);

        d = d.add(128);
        s = s.add(128);
        rem -= 128;
    }

    // Tail with regular stores (small, OK to cache)
    if rem > 0 {
        copy_tail_avx2(d, s, rem);
    }

    // REQUIRED: fence ensures NT stores are visible before function returns
    // SAFETY: SFENCE orders prior non-temporal stores; required for correctness
    // before returning to callers that may observe the memory.
    _mm_sfence();

    dest
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memcpy_optimized_0_to_1024() {
        let mut src = [0u8; 1024];
        let mut dst = [0u8; 1024];
        for (i, byte) in src.iter_mut().enumerate() {
            *byte = (i % 251) as u8; // prime to avoid patterns
        }

        for n in 0..=1024 {
            dst.fill(0);
            unsafe {
                optimized_memcpy_unified(dst.as_mut_ptr(), src.as_ptr(), n);
            }
            assert_eq!(&dst[..n], &src[..n], "Failed at size {}", n);
            if n < 1024 {
                assert_eq!(dst[n], 0, "Overwrote at size {} (index {})", n, n);
            }
        }
    }

    #[test]
    fn test_memcpy_optimized_alignment() {
        let mut src_full = [0u8; 1100];
        let mut dst_full = [0u8; 1100];
        for (i, byte) in src_full.iter_mut().enumerate() {
            *byte = (i % 251) as u8;
        }

        // Test various alignments for src and dst
        for src_off in 0..32 {
            for dst_off in 0..32 {
                for n in [
                    0, 1, 7, 8, 15, 16, 31, 32, 63, 64, 65, 127, 128, 129, 255, 256, 257, 511, 512,
                    1024,
                ] {
                    dst_full.fill(0);
                    unsafe {
                        optimized_memcpy_unified(
                            dst_full.as_mut_ptr().add(dst_off),
                            src_full.as_ptr().add(src_off),
                            n,
                        );
                    }
                    assert_eq!(
                        &dst_full[dst_off..dst_off + n],
                        &src_full[src_off..src_off + n],
                        "Failed at size {} with src_off {} dst_off {}",
                        n,
                        src_off,
                        dst_off
                    );
                }
            }
        }
    }
}
