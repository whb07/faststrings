//! memcpy Criterion harness.
//!
//! ## Measurement model (glibc game)
//!
//! Both glibc and faststrings are invoked through the same opaque ABI:
//! `unsafe fn(*mut u8, *const u8, usize) -> *mut u8`, via a `black_box`'d
//! function pointer, with `#[inline(never)]` shims. Length is also
//! `black_box`'d so the compiler cannot specialize on size.
//!
//! That matches a dynamic `memcpy@plt` call with runtime `n` — not an
//! inlined Rust leaf vs a PLT call.
//!
//! ## Separate games
//!
//! - **Hot** (`memcpy_hot_fixed`, thresholds, alignment, pages): reused buffers,
//!   leaf / dispatch quality in cache.
//! - **Rotating**: working-set / cache-level traffic.
//! - **Cold**: cache-flushed; bandwidth and miss cost, not leaf latency.

use core::ffi::c_void;
use criterion::{
    BatchSize, BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main,
};
use faststrings::memcpy::optimized_memcpy_unified;

#[path = "bench_support.rs"]
mod support;
use support::{AlignedBuffer, flush_range};

/// Opaque memcpy ABI shared by both implementations under test.
type MemcpyFn = unsafe fn(*mut u8, *const u8, usize) -> *mut u8;

unsafe extern "C" {
    #[link_name = "memcpy"]
    fn libc_memcpy(dest: *mut c_void, src: *const c_void, n: usize) -> *mut c_void;
}

/// Thin glibc shim with the same Rust fn ABI as the faststrings entry.
/// Kept `inline(never)` so the call stays opaque like `memcpy@plt`.
#[inline(never)]
unsafe fn glibc_memcpy_abi(dest: *mut u8, src: *const u8, n: usize) -> *mut u8 {
    unsafe { libc_memcpy(dest.cast(), src.cast(), n) as *mut u8 }
}

/// Faststrings entry for ABI-fair benches. Must not inline into the Criterion
/// loop or we measure an inlined leaf against glibc's PLT call.
#[inline(never)]
unsafe fn faststrings_memcpy_abi(dest: *mut u8, src: *const u8, n: usize) -> *mut u8 {
    unsafe { optimized_memcpy_unified(dest, src, n) }
}

fn memcpy_fn(name: &str) -> MemcpyFn {
    match name {
        "glibc" => glibc_memcpy_abi,
        "faststrings" => faststrings_memcpy_abi,
        _ => unreachable!("unknown memcpy impl {name}"),
    }
}

/// Optional comma-separated size list, e.g. `127,128,129,255,256,257`.
/// When unset, the full default hot-fixed size matrix is used.
fn configured_hot_sizes() -> Vec<usize> {
    const DEFAULT: &[usize] = &[
        1, 7, 16, 31, 32, 62, 63, 64, 65, 127, 128, 129, 255, 256, 257, 511, 512, 513, 1023,
        1024, 1025, 4096, 65536, 262144, 16 * 1024 * 1024 - 1, 16 * 1024 * 1024,
        16 * 1024 * 1024 + 1,
    ];
    match std::env::var("FASTSTRINGS_MEMCPY_SIZES") {
        Ok(raw) if !raw.trim().is_empty() => raw
            .split(',')
            .filter_map(|s| s.trim().parse::<usize>().ok())
            .filter(|&n| n > 0)
            .collect(),
        _ => DEFAULT.to_vec(),
    }
}

/// Optional comma-separated group list:
/// `hot`, `thresholds`, `alignment`, `pages`, `rotating`, `cold`.
/// When unset, all groups run.
fn group_enabled(name: &str) -> bool {
    match std::env::var("FASTSTRINGS_MEMCPY_GROUPS") {
        Ok(raw) if !raw.trim().is_empty() => raw
            .split(',')
            .map(str::trim)
            .any(|g| g.eq_ignore_ascii_case(name)),
        _ => true,
    }
}

#[inline(always)]
unsafe fn call_memcpy(f: MemcpyFn, dst: *mut u8, src: *const u8, len: usize) {
    // black_box the fn pointer so rustc cannot devirtualize to a direct call.
    let f = black_box(f);
    unsafe {
        let _ = f(black_box(dst), black_box(src), black_box(len));
        black_box(core::ptr::read_volatile(dst));
    }
}

fn fixed_case(
    c: &mut Criterion,
    group_name: &str,
    label: &str,
    len: usize,
    src_off: usize,
    dst_off: usize,
) {
    let mut group = c.benchmark_group(group_name);
    group.throughput(Throughput::Bytes(len as u64));
    let mut src = AlignedBuffer::new(len + src_off + 4096, 4096);
    let mut dst = AlignedBuffer::new(len + dst_off + 4096, 4096);
    let src_ptr = src.ptr(src_off);
    let dst_ptr = dst.mut_ptr(dst_off);
    for name in ["glibc", "faststrings"] {
        let f = memcpy_fn(name);
        group.bench_function(BenchmarkId::new(name, label), |b| {
            b.iter(|| unsafe {
                call_memcpy(f, dst_ptr, src_ptr, len);
            })
        });
    }
    black_box((&mut src, &mut dst));
    group.finish();
}

fn hot_and_threshold_benches(c: &mut Criterion) {
    // Hot game: reused buffers, leaf/dispatch latency.
    if group_enabled("hot") {
        for len in configured_hot_sizes() {
            fixed_case(c, "memcpy_hot_fixed", &format!("size_{len}"), len, 0, 0);
        }
    }

    if !group_enabled("thresholds") {
        return;
    }

    // Deterministic randomized points around each implementation dispatch cliff.
    let mut state = 0x9e37_79b9_u32;
    for threshold in [63usize, 128, 256, 512, 1024, 16 * 1024 * 1024] {
        for sample in 0..3 {
            state ^= state << 13;
            state ^= state >> 17;
            state ^= state << 5;
            let radius = if threshold > 4096 { 4096 } else { 15 };
            let delta = (state as usize % (radius * 2 + 1)) as isize - radius as isize;
            let len = threshold.saturating_add_signed(delta).max(1);
            fixed_case(
                c,
                "memcpy_random_thresholds",
                &format!("t{threshold}_sample{sample}_size{len}"),
                len,
                0,
                0,
            );
        }
    }
}

fn alignment_and_page_benches(c: &mut Criterion) {
    // Still the hot game (buffers stay warm); varies pointer alignment / pages.
    if group_enabled("alignment") {
        for &len in &[64usize, 257, 4096] {
            for &(src_off, dst_off) in &[(0usize, 0usize), (0, 1), (1, 0), (7, 31), (31, 7), (63, 32)]
            {
                fixed_case(
                    c,
                    "memcpy_alignment",
                    &format!("len{len}_s{src_off}_d{dst_off}"),
                    len,
                    src_off,
                    dst_off,
                );
            }
        }
    }

    if !group_enabled("pages") {
        return;
    }

    for &(label, len, src_off, dst_off) in &[
        ("end_at_page", 256usize, 4096 - 256, 4096 - 256),
        ("cross_page_src", 256, 4096 - 127, 0),
        ("cross_page_dst", 256, 0, 4096 - 127),
        ("cross_both_different", 256, 4096 - 31, 4096 - 191),
    ] {
        fixed_case(c, "memcpy_page_boundaries", label, len, src_off, dst_off);
    }
}

fn rotating_working_sets(c: &mut Criterion) {
    // Rotating game: working-set / cache-level traffic (not leaf microbench).
    if !group_enabled("rotating") {
        return;
    }

    for &(label, working_set, chunk) in &[
        ("l1_32k", 32 * 1024usize, 256usize),
        ("l2_1m", 1024 * 1024, 4096),
        ("llc_32m", 32 * 1024 * 1024, 64 * 1024),
        ("stream_128m", 128 * 1024 * 1024, 1024 * 1024),
    ] {
        let mut group = c.benchmark_group("memcpy_rotating_working_set");
        group.throughput(Throughput::Bytes(chunk as u64));
        let src = AlignedBuffer::new(working_set + 64, 64);
        let mut dst = AlignedBuffer::new(working_set + 64, 64);
        let slots = working_set / chunk;
        for name in ["glibc", "faststrings"] {
            let f = memcpy_fn(name);
            let mut index = 0usize;
            group.bench_function(BenchmarkId::new(name, label), |b| {
                b.iter(|| unsafe {
                    let off = index * chunk;
                    call_memcpy(f, dst.mut_ptr(off), src.ptr(off), chunk);
                    index += 1;
                    if index == slots {
                        index = 0;
                    }
                })
            });
        }
        group.finish();
    }
}

fn cold_benches(c: &mut Criterion) {
    // Cold game: cache-flushed; miss / bandwidth dominated.
    if !group_enabled("cold") {
        return;
    }

    for &len in &[64usize, 4096, 256 * 1024] {
        let mut group = c.benchmark_group("memcpy_cold_flushed");
        group.throughput(Throughput::Bytes(len as u64));
        let src = AlignedBuffer::new(len, 64);
        let mut dst = AlignedBuffer::new(len, 64);
        let src_ptr = src.ptr(0);
        let dst_ptr = dst.mut_ptr(0);
        for name in ["glibc", "faststrings"] {
            let f = memcpy_fn(name);
            group.bench_function(BenchmarkId::new(name, format!("size_{len}")), |b| {
                b.iter_batched(
                    || unsafe {
                        flush_range(src_ptr, len);
                        flush_range(dst_ptr, len);
                    },
                    |_| unsafe {
                        call_memcpy(f, dst_ptr, src_ptr, len);
                    },
                    BatchSize::PerIteration,
                )
            });
        }
        group.finish();
    }
}

criterion_group!(
    benches,
    hot_and_threshold_benches,
    alignment_and_page_benches,
    rotating_working_sets,
    cold_benches
);
criterion_main!(benches);
