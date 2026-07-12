//! ABI-fair memmove benchmarks against glibc.
//!
//! Both implementations use identical non-inlined shims called through an
//! opaque function pointer. Benchmark groups can be selected with
//! `FASTSTRINGS_MEMMOVE_GROUPS=overlap,nonoverlap,deltas,same,alignment,pages,rotating,cold`.

use core::ffi::c_void;
use criterion::{
    BatchSize, BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main,
};
use faststrings::memmove::optimized_memmove_unified;
use std::time::Duration;

#[path = "bench_support.rs"]
mod support;
use support::{AlignedBuffer, flush_range};

unsafe extern "C" {
    #[link_name = "memmove"]
    fn libc_memmove(dest: *mut c_void, src: *const c_void, n: usize) -> *mut c_void;
}

type MemmoveFn = unsafe fn(*mut u8, *const u8, usize) -> *mut u8;

#[inline(never)]
unsafe fn glibc_memmove_abi(dst: *mut u8, src: *const u8, len: usize) -> *mut u8 {
    unsafe { libc_memmove(dst.cast(), src.cast(), len) as *mut u8 }
}

#[inline(never)]
unsafe fn faststrings_memmove_abi(dst: *mut u8, src: *const u8, len: usize) -> *mut u8 {
    unsafe { optimized_memmove_unified(dst, src, len) }
}

fn memmove_fn(name: &str) -> MemmoveFn {
    match name {
        "glibc" => glibc_memmove_abi,
        "faststrings" => faststrings_memmove_abi,
        _ => unreachable!("unknown memmove implementation {name}"),
    }
}

fn group_enabled(name: &str) -> bool {
    match std::env::var("FASTSTRINGS_MEMMOVE_GROUPS") {
        Ok(raw) if !raw.trim().is_empty() => raw
            .split(',')
            .map(str::trim)
            .any(|group| group.eq_ignore_ascii_case(name)),
        _ => true,
    }
}

#[derive(Clone, Copy)]
enum Direction {
    Forward,
    Backward,
}

#[inline(always)]
unsafe fn call_memmove(f: MemmoveFn, dst: *mut u8, src: *const u8, len: usize) {
    let f = black_box(f);
    unsafe {
        let _ = f(black_box(dst), black_box(src), black_box(len));
        black_box(core::ptr::read_volatile(dst));
    }
}

fn configure_group_for_len(
    group: &mut criterion::BenchmarkGroup<'_, criterion::measurement::WallTime>,
    len: usize,
) {
    if len >= 1 << 20 {
        group.sample_size(20);
        group.warm_up_time(Duration::from_millis(300));
        group.measurement_time(Duration::from_millis(900));
    } else {
        group.sample_size(40);
        group.warm_up_time(Duration::from_millis(200));
        group.measurement_time(Duration::from_millis(500));
    }
}

fn bench_pair(
    group: &mut criterion::BenchmarkGroup<'_, criterion::measurement::WallTime>,
    label: &str,
    len: usize,
    dst: *mut u8,
    src: *const u8,
) {
    configure_group_for_len(group, len);
    group.throughput(Throughput::Bytes(len as u64));
    for name in ["glibc", "faststrings"] {
        let f = memmove_fn(name);
        group.bench_function(BenchmarkId::new(name, label), |b| {
            b.iter(|| unsafe { call_memmove(f, dst, src, len) });
        });
    }
}

fn bench_overlap_case(
    group: &mut criterion::BenchmarkGroup<'_, criterion::measurement::WallTime>,
    label: &str,
    len: usize,
    delta: usize,
    direction: Direction,
    base_off: usize,
) {
    let mut buf = AlignedBuffer::new(len + delta + base_off + 4096, 4096);
    let base = buf.mut_ptr(base_off);
    let (src, dst) = match direction {
        Direction::Forward => (unsafe { base.add(delta) } as *const u8, base),
        Direction::Backward => (base as *const u8, unsafe { base.add(delta) }),
    };
    bench_pair(group, label, len, dst, src);
    black_box(&mut buf);
}

fn basic_and_large_benches(c: &mut Criterion) {
    if !group_enabled("overlap") {
        return;
    }
    let mut group = c.benchmark_group("memmove_overlap_sizes");
    for &len in &[
        1usize,
        15,
        16,
        31,
        32,
        63,
        64,
        65,
        255,
        256,
        257,
        1023,
        1024,
        4095,
        4096,
        4097,
        256 * 1024,
        1024 * 1024,
        2 * 1024 * 1024,
    ] {
        let delta = if len == 1 { 0 } else { 1 };
        bench_overlap_case(
            &mut group,
            &format!("forward_len{len}_d{delta}"),
            len,
            delta,
            Direction::Forward,
            0,
        );
        bench_overlap_case(
            &mut group,
            &format!("backward_len{len}_d{delta}"),
            len,
            delta,
            Direction::Backward,
            0,
        );
    }
    group.finish();
}

fn overlap_delta_benches(c: &mut Criterion) {
    if !group_enabled("deltas") {
        return;
    }
    let mut group = c.benchmark_group("memmove_overlap_deltas");

    for &len in &[256usize, 4096] {
        for &delta in &[1usize, 15, 16, 31, 32, 33, 63, 64, 65, 127, 128, 129] {
            if delta >= len {
                continue;
            }
            for &(direction, tag) in &[(Direction::Forward, "fwd"), (Direction::Backward, "bwd")] {
                bench_overlap_case(
                    &mut group,
                    &format!("{tag}_len{len}_d{delta}"),
                    len,
                    delta,
                    direction,
                    0,
                );
            }
        }
    }

    for &len in &[256 * 1024usize, 1024 * 1024] {
        for &delta in &[1usize, 31, 32, 64, 128] {
            for &(direction, tag) in &[(Direction::Forward, "fwd"), (Direction::Backward, "bwd")] {
                bench_overlap_case(
                    &mut group,
                    &format!("{tag}_len{len}_d{delta}"),
                    len,
                    delta,
                    direction,
                    0,
                );
            }
        }
    }
    group.finish();
}

fn same_pointer_benches(c: &mut Criterion) {
    if !group_enabled("same") {
        return;
    }
    let mut group = c.benchmark_group("memmove_same_pointer");
    for &len in &[0usize, 1, 64, 4096, 1024 * 1024] {
        let mut buf = AlignedBuffer::new(len + 64, 64);
        let ptr = buf.mut_ptr(0);
        bench_pair(&mut group, &format!("size_{len}"), len, ptr, ptr);
        black_box(&mut buf);
    }
    group.finish();
}

fn alignment_benches(c: &mut Criterion) {
    if !group_enabled("alignment") {
        return;
    }
    let mut nonoverlap_group = c.benchmark_group("memmove_nonoverlap_alignment");
    for &len in &[64usize, 257, 4096] {
        for &(src_off, dst_off) in &[(0usize, 0usize), (1, 1), (0, 1), (31, 17)] {
            let mut src = AlignedBuffer::new(len + 64, 64);
            let mut dst = AlignedBuffer::new(len + 64, 64);
            bench_pair(
                &mut nonoverlap_group,
                &format!("len{len}_s{src_off}_d{dst_off}"),
                len,
                dst.mut_ptr(dst_off),
                src.ptr(src_off),
            );
            black_box((&mut src, &mut dst));
        }
    }
    nonoverlap_group.finish();

    let mut overlap_group = c.benchmark_group("memmove_overlap_alignment");
    for &(base_off, delta) in &[(0usize, 1usize), (1, 31), (7, 32), (31, 33)] {
        bench_overlap_case(
            &mut overlap_group,
            &format!("len4096_base{base_off}_d{delta}"),
            4096,
            delta,
            Direction::Backward,
            base_off,
        );
    }
    overlap_group.finish();
}

fn nonoverlap_benches(c: &mut Criterion) {
    if !group_enabled("nonoverlap") {
        return;
    }
    let mut group = c.benchmark_group("memmove_nonoverlap_sizes");
    for &len in &[
        1usize,
        15,
        16,
        31,
        32,
        63,
        64,
        65,
        255,
        256,
        257,
        1023,
        1024,
        4095,
        4096,
        4097,
        256 * 1024,
        1024 * 1024,
        2 * 1024 * 1024,
    ] {
        let src = AlignedBuffer::new(len + 64, 64);
        let mut dst = AlignedBuffer::new(len + 64, 64);
        bench_pair(
            &mut group,
            &format!("size_{len}"),
            len,
            dst.mut_ptr(0),
            src.ptr(0),
        );
        black_box((&src, &mut dst));
    }
    group.finish();
}

fn page_boundary_benches(c: &mut Criterion) {
    if !group_enabled("pages") {
        return;
    }
    let mut group = c.benchmark_group("memmove_page_boundaries");
    for &(label, len, src_off, dst_off) in &[
        ("end_at_page", 256usize, 4096 - 256, 4096 - 256),
        ("cross_page_src", 256, 4096 - 127, 0),
        ("cross_page_dst", 256, 0, 4096 - 127),
        ("cross_both_different", 256, 4096 - 31, 4096 - 191),
    ] {
        let src = AlignedBuffer::new(8192, 4096);
        let mut dst = AlignedBuffer::new(8192, 4096);
        bench_pair(
            &mut group,
            label,
            len,
            dst.mut_ptr(dst_off),
            src.ptr(src_off),
        );
        black_box((&src, &mut dst));
    }
    group.finish();
}

fn rotating_working_set_benches(c: &mut Criterion) {
    if !group_enabled("rotating") {
        return;
    }
    for &(label, working_set, chunk) in &[
        ("l1_32k", 32 * 1024usize, 256usize),
        ("l2_1m", 1024 * 1024, 4096),
        ("llc_32m", 32 * 1024 * 1024, 64 * 1024),
        ("stream_128m", 128 * 1024 * 1024, 1024 * 1024),
    ] {
        let mut group = c.benchmark_group("memmove_rotating_nonoverlap");
        configure_group_for_len(&mut group, chunk);
        group.throughput(Throughput::Bytes(chunk as u64));
        let src = AlignedBuffer::new(working_set + 64, 64);
        let mut dst = AlignedBuffer::new(working_set + 64, 64);
        let slots = working_set / chunk;
        for name in ["glibc", "faststrings"] {
            let f = memmove_fn(name);
            let mut index = 0usize;
            group.bench_function(BenchmarkId::new(name, label), |b| {
                b.iter(|| unsafe {
                    let offset = index * chunk;
                    call_memmove(f, dst.mut_ptr(offset), src.ptr(offset), chunk);
                    index += 1;
                    if index == slots {
                        index = 0;
                    }
                });
            });
        }
        black_box((&src, &mut dst));
        group.finish();
    }
}

fn cold_benches(c: &mut Criterion) {
    if !group_enabled("cold") {
        return;
    }
    for &len in &[64usize, 4096, 256 * 1024] {
        let mut group = c.benchmark_group("memmove_cold_nonoverlap");
        configure_group_for_len(&mut group, len);
        group.throughput(Throughput::Bytes(len as u64));
        let src = AlignedBuffer::new(len + 64, 64);
        let mut dst = AlignedBuffer::new(len + 64, 64);
        let src_ptr = src.ptr(0);
        let dst_ptr = dst.mut_ptr(0);
        for name in ["glibc", "faststrings"] {
            let f = memmove_fn(name);
            group.bench_function(BenchmarkId::new(name, format!("size_{len}")), |b| {
                b.iter_batched(
                    || unsafe {
                        flush_range(src_ptr, len);
                        flush_range(dst_ptr, len);
                    },
                    |_| unsafe { call_memmove(f, dst_ptr, src_ptr, len) },
                    BatchSize::PerIteration,
                );
            });
        }
        black_box((&src, &mut dst));
        group.finish();
    }
}

criterion_group!(
    benches,
    basic_and_large_benches,
    nonoverlap_benches,
    overlap_delta_benches,
    same_pointer_benches,
    alignment_benches,
    page_boundary_benches,
    rotating_working_set_benches,
    cold_benches
);
criterion_main!(benches);
