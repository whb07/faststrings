//! SIMD-optimized memory operations for x86_64
//!
//! This module provides high-performance implementations using SSE2 and AVX2
//! instructions. The main entry point is `memcpy_unified` which automatically
//! dispatches to the optimal code path based on buffer size.
//!
//! # Performance characteristics
//!
//! - 0-63 bytes: SSE2 with overlapping stores (no AVX entry fee)
//! - 64-1023 bytes: AVX2 branchless overlapping stores
//! - 1024+ bytes: AVX2 with aligned stores in a loop
//!
//! # Safety
//!
//! All functions in this module are unsafe as they use raw pointers and
//! SIMD intrinsics. Callers must ensure:
//! - Pointers are valid for the specified length
//! - Buffers do not overlap (use memmove for overlapping buffers)

#![allow(unsafe_code)]

use core::arch::x86_64::*;

// =============================================================================
// DISPATCH THRESHOLDS
// =============================================================================

/// Size threshold below which SSE2 is used (avoids AVX entry fee)
/// SSE small path correctly handles 0-64 bytes, so threshold is 65
pub const TINY_THRESHOLD: usize = 65;

/// Size threshold above which the aligned loop is used
pub const LARGE_THRESHOLD: usize = 256;

// =============================================================================
// UNIFIED DISPATCH: Entry point for all memcpy operations
// =============================================================================
//
// Design principles:
// 1. No AVX attribute on dispatch - stays in SSE/GPR land for small copies
// 2. Compiler can optimize away AVX paths for constant-size small copies
// 3. Clean I-cache utilization for the common small-copy case
// 4. No VEX/legacy transition penalties for tiny copies

/// High-performance memcpy with automatic dispatch based on size.
///
/// # Safety
///
/// - `dest` and `src` must be valid for reads/writes of `n` bytes
/// - The memory regions must not overlap
/// - AVX2 must be supported (check with `is_x86_feature_detected!("avx2")`)
// #[target_feature(enable = "avx2")]
// #[inline]
pub unsafe fn memcpy_unified(dest: *mut u8, src: *const u8, n: usize) -> *mut u8 {
    if n < TINY_THRESHOLD {
        // Tiny: SSE/scalar - avoid AVX entry fee entirely
        memcpy_sse_small(dest, src, n)
    } else if n < LARGE_THRESHOLD {
        // Medium: Branchless AVX2 overlapping stores (no loops)
        memcpy_avx2_medium(dest, src, n)
    } else {
        // Large: AVX2 with alignment prologue + main loop
        memcpy_avx2_large(dest, src, n)
    }
}

/// High-performance memcpy variant with refined AVX2 large path.
///
/// # Safety
///
/// - `dest` and `src` must be valid for reads/writes of `n` bytes
/// - The memory regions must not overlap
/// - AVX2 must be supported (check with `is_x86_feature_detected!("avx2")`)
// #[target_feature(enable = "avx2")]
// #[inline]
pub unsafe fn memcpy_unified_refined(dest: *mut u8, src: *const u8, n: usize) -> *mut u8 {
    if n < TINY_THRESHOLD {
        // Tiny: SSE/scalar - avoid AVX entry fee entirely
        memcpy_sse_small(dest, src, n)
    } else if n < LARGE_THRESHOLD {
        // Medium: Branchless AVX2 overlapping stores (no loops)
        memcpy_avx2_medium(dest, src, n)
    } else {
        // Large: AVX2 with alignment prologue + main loop
        memcpy_avx2_large_refined(dest, src, n)
    }
}

/// High-performance memcpy variant without medium-size AVX2 path.
///
/// # Safety
///
/// - `dest` and `src` must be valid for reads/writes of `n` bytes
/// - The memory regions must not overlap
/// - AVX2 must be supported (check with `is_x86_feature_detected!("avx2")`)
#[target_feature(enable = "avx2")]
// #[inline]
pub unsafe fn memcpy_unified_no_medium(dest: *mut u8, src: *const u8, n: usize) -> *mut u8 {
    if n < TINY_THRESHOLD {
        // Tiny: SSE/scalar - avoid AVX entry fee entirely
        memcpy_sse_small(dest, src, n)
    } else {
        // Large: AVX2 with alignment prologue + main loop
        memcpy_avx2_large(dest, src, n)
    }
}

/// High-performance memcpy variant without medium-size AVX2 path and refined large path.
///
/// # Safety
///
/// - `dest` and `src` must be valid for reads/writes of `n` bytes
/// - The memory regions must not overlap
/// - AVX2 must be supported (check with `is_x86_feature_detected!("avx2")`)
#[target_feature(enable = "avx2")]
// #[inline]
pub unsafe fn memcpy_unified_no_medium_refined(dest: *mut u8, src: *const u8, n: usize) -> *mut u8 {
    if n < TINY_THRESHOLD {
        // Tiny: SSE/scalar - avoid AVX entry fee entirely
        memcpy_sse_small(dest, src, n)
    } else {
        // Large: AVX2 with alignment prologue + main loop
        memcpy_avx2_large_refined(dest, src, n)
    }
}

// =============================================================================
// SMALL PATH: SSE2 Implementation (0-63 bytes)
// =============================================================================
//
// Key design decisions:
// - No AVX attribute: Uses legacy SSE encoding
// - No loops: Fully unrolled for each size class
// - Overlapping stores: Eliminates tail handling branches
// - All loads before stores: Maximizes ILP on Intel

/// SSE2 memcpy for small buffers (0-63 bytes)
///
/// # Safety
/// - `dest` and `src` must be valid for reads/writes of `n` bytes
/// - The memory regions must not overlap
#[inline(always)]
pub unsafe fn memcpy_sse_small(dest: *mut u8, src: *const u8, n: usize) -> *mut u8 {
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
// MEDIUM PATH: AVX2 Branchless Implementation (64-1023 bytes)
// =============================================================================
//
// Key design decisions:
// - Fully unrolled: No loops, fixed instruction count per size class
// - Overlapping stores: Front half + back half, meeting/overlapping in middle
// - All loads grouped before stores: Maximizes ILP

/// AVX2 memcpy for medium buffers (64-256 bytes)
///
/// # Safety
/// - `dest` and `src` must be valid for reads/writes of `n` bytes
/// - The memory regions must not overlap
/// - AVX2 must be supported
#[target_feature(enable = "avx2")]
#[inline]
pub unsafe fn memcpy_avx2_medium(dest: *mut u8, src: *const u8, n: usize) -> *mut u8 {
    // 128-256 bytes: 8x 32-byte YMM loads/stores (128 front + 128 back)
    // All loads before stores maximizes memory-level parallelism
    if n >= 128 {
        let v0 = _mm256_loadu_si256(src as *const __m256i);
        let v1 = _mm256_loadu_si256(src.add(32) as *const __m256i);
        let v2 = _mm256_loadu_si256(src.add(64) as *const __m256i);
        let v3 = _mm256_loadu_si256(src.add(96) as *const __m256i);
        let v4 = _mm256_loadu_si256(src.add(n - 128) as *const __m256i);
        let v5 = _mm256_loadu_si256(src.add(n - 96) as *const __m256i);
        let v6 = _mm256_loadu_si256(src.add(n - 64) as *const __m256i);
        let v7 = _mm256_loadu_si256(src.add(n - 32) as *const __m256i);

        // SAFETY: Unaligned AVX loads/stores are valid for any alignment; caller
        // guarantees `src`/`dest` are valid for `n` bytes and non-overlapping.
        _mm256_storeu_si256(dest as *mut __m256i, v0);
        _mm256_storeu_si256(dest.add(32) as *mut __m256i, v1);
        _mm256_storeu_si256(dest.add(64) as *mut __m256i, v2);
        _mm256_storeu_si256(dest.add(96) as *mut __m256i, v3);
        _mm256_storeu_si256(dest.add(n - 128) as *mut __m256i, v4);
        _mm256_storeu_si256(dest.add(n - 96) as *mut __m256i, v5);
        _mm256_storeu_si256(dest.add(n - 64) as *mut __m256i, v6);
        _mm256_storeu_si256(dest.add(n - 32) as *mut __m256i, v7);
        return dest;
    }

    // 64-127 bytes: 4x 32-byte YMM loads/stores (64 front + 64 back)
    let v0 = _mm256_loadu_si256(src as *const __m256i);
    let v1 = _mm256_loadu_si256(src.add(32) as *const __m256i);
    let v2 = _mm256_loadu_si256(src.add(n - 64) as *const __m256i);
    let v3 = _mm256_loadu_si256(src.add(n - 32) as *const __m256i);

    // SAFETY: Unaligned AVX stores are valid for any alignment; caller guarantees
    // `dest` is writable for `n` bytes and non-overlapping with `src`.
    _mm256_storeu_si256(dest as *mut __m256i, v0);
    _mm256_storeu_si256(dest.add(32) as *mut __m256i, v1);
    _mm256_storeu_si256(dest.add(n - 64) as *mut __m256i, v2);
    _mm256_storeu_si256(dest.add(n - 32) as *mut __m256i, v3);

    dest
}

// =============================================================================
// LARGE PATH: AVX2 Aligned Loop Implementation (256+ bytes)
// =============================================================================
//
// Key design decisions:
// - Alignment prologue: Aligns destination to 32-byte boundary
// - Aligned stores: Avoids cache-line splits in main loop
// - 128-byte unroll: 4 × 32-byte = one cache line pair per iteration
// - Overlapping tail: Eliminates complex tail handling

/// AVX2 memcpy for large buffers (256+ bytes)
///
/// Uses aligned stores in a loop for maximum throughput on large copies.
///
/// # Safety
/// - `dest` and `src` must be valid for reads/writes of `n` bytes
/// - The memory regions must not overlap
/// - AVX2 must be supported
/// - n must be >= 128 (for tail handling)
#[target_feature(enable = "avx2")]
#[inline]
pub unsafe fn memcpy_avx2_large(dest: *mut u8, src: *const u8, n: usize) -> *mut u8 {
    let mut d = dest;
    let mut s = src;
    let mut remaining = n;

    // n >= 256 guaranteed by dispatch (LARGE_THRESHOLD)

    // ALIGNMENT PROLOGUE: Copy first 32 bytes, then advance to aligned boundary
    // SAFETY: Unaligned load/store are valid for any alignment; caller guarantees
    // `src`/`dest` are valid for `n` bytes.
    let first_v = _mm256_loadu_si256(s as *const __m256i);
    _mm256_storeu_si256(d as *mut __m256i, first_v);

    let aligned_d = ((d as usize) + 32) & !31;
    let advance = aligned_d - (d as usize);
    d = aligned_d as *mut u8;
    s = s.add(advance);
    remaining -= advance;

    // MAIN LOOP: 128 bytes per iteration with aligned stores
    while remaining >= 128 {
        // SAFETY: Unaligned loads are valid for any alignment; caller guarantees
        // `src` is readable for the loop span.
        let v0 = _mm256_loadu_si256(s as *const __m256i);
        let v1 = _mm256_loadu_si256(s.add(32) as *const __m256i);
        let v2 = _mm256_loadu_si256(s.add(64) as *const __m256i);
        let v3 = _mm256_loadu_si256(s.add(96) as *const __m256i);

        // SAFETY: Aligned stores require 32-byte alignment; `d` is aligned by
        // the prologue and advances in 32-byte multiples.
        _mm256_store_si256(d as *mut __m256i, v0);
        _mm256_store_si256(d.add(32) as *mut __m256i, v1);
        _mm256_store_si256(d.add(64) as *mut __m256i, v2);
        _mm256_store_si256(d.add(96) as *mut __m256i, v3);

        d = d.add(128);
        s = s.add(128);
        remaining -= 128;
    }

    // TAIL: Copy last 128 bytes (overlapping with main loop is fine)
    if remaining > 0 {
        let src_tail = src.add(n - 128);
        let dst_tail = dest.add(n - 128);

        // SAFETY: Unaligned loads/stores are valid for any alignment; caller
        // guarantees tail ranges are within `n` bytes.
        let t0 = _mm256_loadu_si256(src_tail as *const __m256i);
        let t1 = _mm256_loadu_si256(src_tail.add(32) as *const __m256i);
        let t2 = _mm256_loadu_si256(src_tail.add(64) as *const __m256i);
        let t3 = _mm256_loadu_si256(src_tail.add(96) as *const __m256i);

        _mm256_storeu_si256(dst_tail as *mut __m256i, t0);
        _mm256_storeu_si256(dst_tail.add(32) as *mut __m256i, t1);
        _mm256_storeu_si256(dst_tail.add(64) as *mut __m256i, t2);
        _mm256_storeu_si256(dst_tail.add(96) as *mut __m256i, t3);
    }

    dest
}

// =============================================================================
// AVX2 Loop Small: Simple 128B loop without alignment for 128-512 bytes
// =============================================================================

/// AVX2 loop memcpy for 128-512 byte copies without alignment handling.
///
/// # Safety
///
/// - `dest` and `src` must be valid for reads/writes of `n` bytes
/// - The memory regions must not overlap
/// - AVX2 must be supported (check with `is_x86_feature_detected!("avx2")`)
#[target_feature(enable = "avx2")]
pub unsafe fn memcpy_avx2_loop_small(dest: *mut u8, src: *const u8, n: usize) -> *mut u8 {
    let mut offset = 0usize;

    // Main loop: 128 bytes per iteration, no alignment
    while offset + 128 <= n {
        // SAFETY: Unaligned loads/stores are valid for any alignment; caller
        // guarantees `src`/`dest` are valid for `n` bytes.
        let v0 = _mm256_loadu_si256(src.add(offset) as *const __m256i);
        let v1 = _mm256_loadu_si256(src.add(offset + 32) as *const __m256i);
        let v2 = _mm256_loadu_si256(src.add(offset + 64) as *const __m256i);
        let v3 = _mm256_loadu_si256(src.add(offset + 96) as *const __m256i);

        _mm256_storeu_si256(dest.add(offset) as *mut __m256i, v0);
        _mm256_storeu_si256(dest.add(offset + 32) as *mut __m256i, v1);
        _mm256_storeu_si256(dest.add(offset + 64) as *mut __m256i, v2);
        _mm256_storeu_si256(dest.add(offset + 96) as *mut __m256i, v3);

        offset += 128;
    }

    // Overlapping tail: last 128 bytes
    // SAFETY: Unaligned loads/stores are valid for any alignment; caller
    // guarantees tail ranges are within `n` bytes.
    let t0 = _mm256_loadu_si256(src.add(n - 128) as *const __m256i);
    let t1 = _mm256_loadu_si256(src.add(n - 96) as *const __m256i);
    let t2 = _mm256_loadu_si256(src.add(n - 64) as *const __m256i);
    let t3 = _mm256_loadu_si256(src.add(n - 32) as *const __m256i);

    _mm256_storeu_si256(dest.add(n - 128) as *mut __m256i, t0);
    _mm256_storeu_si256(dest.add(n - 96) as *mut __m256i, t1);
    _mm256_storeu_si256(dest.add(n - 64) as *mut __m256i, t2);
    _mm256_storeu_si256(dest.add(n - 32) as *mut __m256i, t3);

    dest
}

// =============================================================================
// LARGE PATH REFINED: AVX2 with 256B interleaved blocks
// =============================================================================

/// AVX2 memcpy with 256-byte interleaved blocks and medium path tail
///
/// Key differences from memcpy_avx2_large:
/// - 256-byte blocks with interleaved load/store pattern
/// - Reuses medium path for 0-255 byte tail
///
/// # Safety
/// - `dest` and `src` must be valid for reads/writes of `n` bytes
/// - The memory regions must not overlap
/// - AVX2 must be supported
#[target_feature(enable = "avx2")]
#[inline]
pub unsafe fn memcpy_avx2_large_refined(dest: *mut u8, src: *const u8, n: usize) -> *mut u8 {
    // 1. Prologue: Align destination to 32-byte boundary
    // SAFETY: Unaligned load/store are valid for any alignment; caller guarantees
    // `src`/`dest` are valid for `n` bytes.
    let first_v = _mm256_loadu_si256(src as *const __m256i);
    _mm256_storeu_si256(dest as *mut __m256i, first_v);

    let advance = 32 - ((dest as usize) & 31);
    let mut d = dest.add(advance);
    let mut s = src.add(advance);
    let mut remaining = n - advance;

    // 2. Main Loop: Interleaved 256B blocks (helps prevent 4K aliasing stalls)
    while remaining >= 256 {
        // SAFETY: Unaligned loads are valid for any alignment; caller guarantees
        // `src` is readable for the loop span.
        let v0 = _mm256_loadu_si256(s as *const __m256i);
        let v1 = _mm256_loadu_si256(s.add(32) as *const __m256i);
        let v2 = _mm256_loadu_si256(s.add(64) as *const __m256i);
        let v3 = _mm256_loadu_si256(s.add(96) as *const __m256i);

        // Interleave stores to keep ports busy
        // SAFETY: Aligned stores require 32-byte alignment; `d` is aligned by
        // the prologue and advances in 32-byte multiples.
        _mm256_store_si256(d as *mut __m256i, v0);
        _mm256_store_si256(d.add(32) as *mut __m256i, v1);
        _mm256_store_si256(d.add(64) as *mut __m256i, v2);
        _mm256_store_si256(d.add(96) as *mut __m256i, v3);

        // SAFETY: Unaligned loads are valid for any alignment; caller guarantees
        // `src` is readable for the loop span.
        let v4 = _mm256_loadu_si256(s.add(128) as *const __m256i);
        let v5 = _mm256_loadu_si256(s.add(160) as *const __m256i);
        let v6 = _mm256_loadu_si256(s.add(192) as *const __m256i);
        let v7 = _mm256_loadu_si256(s.add(224) as *const __m256i);

        // SAFETY: Aligned stores require 32-byte alignment; `d` is aligned by
        // the prologue and advances in 32-byte multiples.
        _mm256_store_si256(d.add(128) as *mut __m256i, v4);
        _mm256_store_si256(d.add(160) as *mut __m256i, v5);
        _mm256_store_si256(d.add(192) as *mut __m256i, v6);
        _mm256_store_si256(d.add(224) as *mut __m256i, v7);

        d = d.add(256);
        s = s.add(256);
        remaining -= 256;
    }

    // 3. Tail Dispatch: Use the Medium Path for the final 0-255 bytes
    if remaining > 0 {
        memcpy_avx2_medium(dest.add(n - remaining), src.add(n - remaining), remaining);
    }

    dest
}

// =============================================================================
// UNALIGNED PATH: No alignment prologue for 256-1024 byte range
// =============================================================================
//
// This variant skips the alignment prologue, trading potential cache-line
// split penalties for reduced setup overhead. Includes explicit VZEROUPPER
// to avoid SSE-AVX transition penalties.

/// AVX2 memcpy using unaligned stores for 256-1024 byte buffers
///
/// Key differences from memcpy_avx2_large:
/// - No alignment prologue (saves setup cycles)
/// - Uses unaligned stores throughout
///
/// # Safety
/// - `dest` and `src` must be valid for reads/writes of `n` bytes
/// - The memory regions must not overlap
/// - AVX2 must be supported
/// - n must be >= 256 and <= 1024
#[target_feature(enable = "avx2")]
pub unsafe fn memcpy_avx2_unaligned_256_1024(dest: *mut u8, src: *const u8, n: usize) -> *mut u8 {
    let mut d = dest;
    let mut s = src;
    let mut remaining = n;

    // 1. MAIN LOOP: 128 bytes per iteration (4 x YMM)
    // We stop when we have 128 bytes or fewer remaining.
    // Since n >= 256, this is guaranteed to run at least once.
    while remaining > 128 {
        // Load everything first to maximize pipeline throughput (ILP)
        // SAFETY: Unaligned loads/stores are valid for any alignment; caller
        // guarantees `src`/`dest` are valid for `n` bytes.
        let v0 = _mm256_loadu_si256(s as *const __m256i);
        let v1 = _mm256_loadu_si256(s.add(32) as *const __m256i);
        let v2 = _mm256_loadu_si256(s.add(64) as *const __m256i);
        let v3 = _mm256_loadu_si256(s.add(96) as *const __m256i);

        // Store after all loads are dispatched
        _mm256_storeu_si256(d as *mut __m256i, v0);
        _mm256_storeu_si256(d.add(32) as *mut __m256i, v1);
        _mm256_storeu_si256(d.add(64) as *mut __m256i, v2);
        _mm256_storeu_si256(d.add(96) as *mut __m256i, v3);

        d = d.add(128);
        s = s.add(128);
        remaining -= 128;
    }

    // 2. OVERLAPPING TAIL: Copy exactly the last 128 bytes.
    // This handles the final chunk (between 1 and 128 bytes).
    // Because it uses the original `n`, it "reaches back" into the
    // buffer to ensure the last byte is exactly at dest + n - 1.
    let src_tail = src.add(n - 128);
    let dst_tail = dest.add(n - 128);

    // SAFETY: Unaligned loads/stores are valid for any alignment; caller
    // guarantees tail ranges are within `n` bytes.
    let t0 = _mm256_loadu_si256(src_tail as *const __m256i);
    let t1 = _mm256_loadu_si256(src_tail.add(32) as *const __m256i);
    let t2 = _mm256_loadu_si256(src_tail.add(64) as *const __m256i);
    let t3 = _mm256_loadu_si256(src_tail.add(96) as *const __m256i);

    _mm256_storeu_si256(dst_tail as *mut __m256i, t0);
    _mm256_storeu_si256(dst_tail.add(32) as *mut __m256i, t1);
    _mm256_storeu_si256(dst_tail.add(64) as *mut __m256i, t2);
    _mm256_storeu_si256(dst_tail.add(96) as *mut __m256i, t3);

    dest
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    extern crate std;
    use super::*;
    use std::is_x86_feature_detected;
    use std::vec;
    use std::vec::Vec;

    /// Test memcpy_sse_small for all sizes from 0 to 64 bytes
    #[test]
    fn test_sse_small_0_to_64() {
        for size in 0..=64 {
            // Create source with recognizable pattern
            let src: Vec<u8> = (0..size).map(|i| (i % 256) as u8).collect();
            let mut dest = vec![0xFFu8; size];

            unsafe {
                memcpy_sse_small(dest.as_mut_ptr(), src.as_ptr(), size);
            }

            assert_eq!(
                src,
                dest,
                "memcpy_sse_small failed at size {}: expected {:?}, got {:?}",
                size,
                &src[..size.min(16)],
                &dest[..size.min(16)]
            );
        }
    }

    /// Test boundary bytes (first, middle, last) are correctly copied
    #[test]
    fn test_sse_small_boundary_bytes() {
        for size in [1, 2, 3, 4, 7, 8, 15, 16, 31, 32, 63, 64] {
            let mut src = vec![0u8; size];
            src[0] = 0xAA;
            if size > 1 {
                src[size - 1] = 0xBB;
            }
            // Only set middle if it doesn't overlap with first or last
            if size > 2 && size / 2 != size - 1 {
                src[size / 2] = 0xCC;
            }

            let mut dest = vec![0xFFu8; size];

            unsafe {
                memcpy_sse_small(dest.as_mut_ptr(), src.as_ptr(), size);
            }

            assert_eq!(dest[0], 0xAA, "First byte wrong at size {}", size);
            if size > 1 {
                assert_eq!(dest[size - 1], 0xBB, "Last byte wrong at size {}", size);
            }
            if size > 2 && size / 2 != size - 1 {
                assert_eq!(dest[size / 2], 0xCC, "Middle byte wrong at size {}", size);
            }
        }
    }

    /// Test that zero-length copy doesn't crash or modify anything
    #[test]
    fn test_sse_small_zero_length() {
        let src = [1u8, 2, 3, 4];
        let mut dest = vec![0xFFu8; 4];

        unsafe {
            memcpy_sse_small(dest.as_mut_ptr(), src.as_ptr(), 0);
        }

        assert_eq!(
            dest,
            vec![0xFFu8; 4],
            "zero-length copy modified destination"
        );
    }

    // =========================================================================
    // AVX2 Medium Path Tests
    // =========================================================================

    /// Test memcpy_avx2_medium for sizes 64 to 256 bytes (max coverage with 8×YMM)
    #[test]
    fn test_avx2_medium_64_to_256() {
        if !is_x86_feature_detected!("avx2") {
            return;
        }

        for size in 64..=256 {
            let src: Vec<u8> = (0..size).map(|i| (i % 256) as u8).collect();
            let mut dest = vec![0xFFu8; size];

            unsafe {
                memcpy_avx2_medium(dest.as_mut_ptr(), src.as_ptr(), size);
            }

            assert_eq!(
                src,
                dest,
                "memcpy_avx2_medium failed at size {}: first diff at byte {}",
                size,
                src.iter()
                    .zip(dest.iter())
                    .position(|(a, b)| a != b)
                    .unwrap_or(size)
            );
        }
    }

    /// Test AVX2 medium at specific boundary sizes
    #[test]
    fn test_avx2_medium_boundary_sizes() {
        if !is_x86_feature_detected!("avx2") {
            return;
        }

        for size in [64, 65, 127, 128, 129, 255, 256] {
            let mut src = vec![0u8; size];
            src[0] = 0xAA;
            if size > 1 {
                src[size - 1] = 0xBB;
            }
            if size > 2 {
                src[size / 2] = 0xCC;
            }

            let mut dest = vec![0xFFu8; size];

            unsafe {
                memcpy_avx2_medium(dest.as_mut_ptr(), src.as_ptr(), size);
            }

            assert_eq!(dest[0], 0xAA, "First byte wrong at size {}", size);
            if size > 1 {
                assert_eq!(dest[size - 1], 0xBB, "Last byte wrong at size {}", size);
            }
            if size > 2 {
                assert_eq!(dest[size / 2], 0xCC, "Middle byte wrong at size {}", size);
            }
        }
    }

    // =========================================================================
    // AVX2 Large Path Tests
    // =========================================================================

    /// Test memcpy_avx2_large for sizes 256B to 512KB
    #[test]
    fn test_avx2_large_256_to_512k() {
        if !is_x86_feature_detected!("avx2") {
            return;
        }

        // Test at key sizes: powers of 2 and boundaries
        let sizes = [
            256, 257, 511, 512, 513, 1023, 1024, 1025, 2048, 4096, 8192, 16384, 32768, 65536,
            131072, 262144, 524288, // 128KB, 256KB, 512KB
        ];

        for &size in &sizes {
            let src: Vec<u8> = (0..size).map(|i| (i % 256) as u8).collect();
            let mut dest = vec![0xFFu8; size];

            unsafe {
                memcpy_avx2_large(dest.as_mut_ptr(), src.as_ptr(), size);
            }

            assert_eq!(
                src,
                dest,
                "memcpy_avx2_large failed at size {}: first diff at byte {}",
                size,
                src.iter()
                    .zip(dest.iter())
                    .position(|(a, b)| a != b)
                    .unwrap_or(size)
            );
        }
    }

    /// Test AVX2 large at boundary bytes
    #[test]
    fn test_avx2_large_boundary_bytes() {
        if !is_x86_feature_detected!("avx2") {
            return;
        }

        for size in [256, 512, 1024, 4096, 65536, 524288] {
            let mut src = vec![0u8; size];
            src[0] = 0xAA;
            src[size - 1] = 0xBB;
            src[size / 2] = 0xCC;
            // Also test near alignment boundaries (avoid overwriting middle for small sizes)
            src[31] = 0xDD;
            src[32] = 0xEE;
            src[127] = 0x11;
            if size / 2 != 128 {
                src[128] = 0x22;
            }

            let mut dest = vec![0xFFu8; size];

            unsafe {
                memcpy_avx2_large(dest.as_mut_ptr(), src.as_ptr(), size);
            }

            assert_eq!(dest[0], 0xAA, "First byte wrong at size {}", size);
            assert_eq!(dest[size - 1], 0xBB, "Last byte wrong at size {}", size);
            assert_eq!(dest[size / 2], 0xCC, "Middle byte wrong at size {}", size);
            assert_eq!(dest[31], 0xDD, "Byte 31 wrong at size {}", size);
            assert_eq!(dest[32], 0xEE, "Byte 32 wrong at size {}", size);
            assert_eq!(dest[127], 0x11, "Byte 127 wrong at size {}", size);
            if size / 2 != 128 {
                assert_eq!(dest[128], 0x22, "Byte 128 wrong at size {}", size);
            }
        }
    }

    // =========================================================================
    // AVX2 Unaligned 256-1024 Path Tests
    // =========================================================================

    /// Test memcpy_avx2_unaligned_256_1024 for sizes 256 to 1024 bytes
    #[test]
    fn test_avx2_unaligned_256_to_1024() {
        if !is_x86_feature_detected!("avx2") {
            return;
        }

        // Test 25 sizes spread across the range
        for size in (256..=1024).step_by(32) {
            let src: Vec<u8> = (0..size).map(|i| (i % 256) as u8).collect();
            let mut dest = vec![0xFFu8; size];

            unsafe {
                memcpy_avx2_unaligned_256_1024(dest.as_mut_ptr(), src.as_ptr(), size);
            }

            assert_eq!(
                src,
                dest,
                "memcpy_avx2_unaligned_256_1024 failed at size {}: first diff at byte {}",
                size,
                src.iter()
                    .zip(dest.iter())
                    .position(|(a, b)| a != b)
                    .unwrap_or(size)
            );
        }
    }

    /// Test unaligned at boundary sizes
    #[test]
    fn test_avx2_unaligned_boundary_sizes() {
        if !is_x86_feature_detected!("avx2") {
            return;
        }

        for size in [256, 257, 384, 512, 640, 768, 896, 1023, 1024] {
            let mut src = vec![0u8; size];
            src[0] = 0xAA;
            src[size - 1] = 0xBB;
            src[size / 2] = 0xCC;

            let mut dest = vec![0xFFu8; size];

            unsafe {
                memcpy_avx2_unaligned_256_1024(dest.as_mut_ptr(), src.as_ptr(), size);
            }

            assert_eq!(dest[0], 0xAA, "First byte wrong at size {}", size);
            assert_eq!(dest[size - 1], 0xBB, "Last byte wrong at size {}", size);
            assert_eq!(dest[size / 2], 0xCC, "Middle byte wrong at size {}", size);
        }
    }
}
