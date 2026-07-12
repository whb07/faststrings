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

// =============================================================================
// AVX DISPATCHER: Centralizes AVX state and manages VZEROUPPER
// =============================================================================

const NT_THRESHOLD: usize = 8 * 1024 * 1024;

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

    // 2. INTERMEDIATE PATH: Unaligned loop for 129B-512B.
    // In this range, the cost of the alignment prologue is higher than the benefit
    // of aligned stores. We use 128-byte unrolled chunks.
    if n <= 512 {
        let mut d = dest;
        let mut s = src;
        let mut rem = n;
        while rem >= 128 {
            // SAFETY: Unaligned loads/stores are valid for any alignment; caller
            // guarantees `src`/`dest` are valid for the loop span.
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
        if rem > 0 {
            let s_tail = src.add(n - 128);
            let d_tail = dest.add(n - 128);
            // SAFETY: Unaligned loads/stores are valid for any alignment; caller
            // guarantees tail ranges are within `n` bytes.
            let v0 = _mm256_loadu_si256(s_tail as *const __m256i);
            let v1 = _mm256_loadu_si256(s_tail.add(32) as *const __m256i);
            let v2 = _mm256_loadu_si256(s_tail.add(64) as *const __m256i);
            let v3 = _mm256_loadu_si256(s_tail.add(96) as *const __m256i);
            _mm256_storeu_si256(d_tail as *mut __m256i, v0);
            _mm256_storeu_si256(d_tail.add(32) as *mut __m256i, v1);
            _mm256_storeu_si256(d_tail.add(64) as *mut __m256i, v2);
            _mm256_storeu_si256(d_tail.add(96) as *mut __m256i, v3);
        }
        return dest;
    }

    // 3. LARGE PATH: Aligned loop for n > 512.
    let mut d = dest;
    let mut s = src;
    let mut rem = n;

    // Alignment prologue
    // SAFETY: Unaligned load/store are valid for any alignment; caller guarantees
    // `src`/`dest` are valid for `n` bytes.
    let first_v = _mm256_loadu_si256(s as *const __m256i);
    _mm256_storeu_si256(d as *mut __m256i, first_v);
    let advance = 32 - ((d as usize) & 31);
    d = d.add(advance);
    s = s.add(advance);
    rem -= advance;

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

    // Overlapping tail
    if rem > 0 {
        let s_tail = src.add(n - 256);
        let d_tail = dest.add(n - 256);
        // SAFETY: Unaligned loads/stores are valid for any alignment; caller
        // guarantees tail ranges are within `n` bytes.
        let v0 = _mm256_loadu_si256(s_tail as *const __m256i);
        let v1 = _mm256_loadu_si256(s_tail.add(32) as *const __m256i);
        let v2 = _mm256_loadu_si256(s_tail.add(64) as *const __m256i);
        let v3 = _mm256_loadu_si256(s_tail.add(96) as *const __m256i);
        let v4 = _mm256_loadu_si256(s_tail.add(128) as *const __m256i);
        let v5 = _mm256_loadu_si256(s_tail.add(160) as *const __m256i);
        let v6 = _mm256_loadu_si256(s_tail.add(192) as *const __m256i);
        let v7 = _mm256_loadu_si256(s_tail.add(224) as *const __m256i);

        _mm256_storeu_si256(d_tail as *mut __m256i, v0);
        _mm256_storeu_si256(d_tail.add(32) as *mut __m256i, v1);
        _mm256_storeu_si256(d_tail.add(64) as *mut __m256i, v2);
        _mm256_storeu_si256(d_tail.add(96) as *mut __m256i, v3);
        _mm256_storeu_si256(d_tail.add(128) as *mut __m256i, v4);
        _mm256_storeu_si256(d_tail.add(160) as *mut __m256i, v5);
        _mm256_storeu_si256(d_tail.add(192) as *mut __m256i, v6);
        _mm256_storeu_si256(d_tail.add(224) as *mut __m256i, v7);
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
        let s_tail = src.add(n - 128);
        let d_tail = dest.add(n - 128);

        // SAFETY: Unaligned loads/stores are valid for any alignment; caller
        // guarantees tail ranges are within `n` bytes.
        let v0 = _mm256_loadu_si256(s_tail as *const __m256i);
        let v1 = _mm256_loadu_si256(s_tail.add(32) as *const __m256i);
        let v2 = _mm256_loadu_si256(s_tail.add(64) as *const __m256i);
        let v3 = _mm256_loadu_si256(s_tail.add(96) as *const __m256i);

        _mm256_storeu_si256(d_tail as *mut __m256i, v0);
        _mm256_storeu_si256(d_tail.add(32) as *mut __m256i, v1);
        _mm256_storeu_si256(d_tail.add(64) as *mut __m256i, v2);
        _mm256_storeu_si256(d_tail.add(96) as *mut __m256i, v3);
    }

    // REQUIRED: fence ensures NT stores are visible before function returns
    // SAFETY: SFENCE orders prior non-temporal stores; required for correctness
    // before returning to callers that may observe the memory.
    _mm_sfence();

    dest
}
