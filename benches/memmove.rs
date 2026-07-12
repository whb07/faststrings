use core::ffi::c_void;
use criterion::{BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main};
use faststrings::memmove::optimized_memmove_unified;
use std::time::Duration;

#[path = "bench_support.rs"]
mod support;
use support::AlignedBuffer;

unsafe extern "C" {
    #[link_name = "memmove"]
    fn libc_memmove(dest: *mut c_void, src: *const c_void, n: usize) -> *mut c_void;
}

#[derive(Clone, Copy)]
enum Implementation {
    Glibc,
    Faststrings,
}

#[derive(Clone, Copy)]
enum Direction {
    Forward,
    Backward,
}

#[inline(always)]
unsafe fn move_bytes(implementation: Implementation, dst: *mut u8, src: *const u8, len: usize) {
    match implementation {
        Implementation::Glibc => unsafe {
            libc_memmove(dst.cast(), src.cast(), len);
        },
        Implementation::Faststrings => unsafe {
            optimized_memmove_unified(dst, src, len);
        },
    }
    black_box(unsafe { core::ptr::read_volatile(dst) });
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
    for (name, implementation) in [
        ("glibc", Implementation::Glibc),
        ("faststrings", Implementation::Faststrings),
    ] {
        group.bench_function(BenchmarkId::new(name, label), |b| {
            b.iter(|| unsafe {
                move_bytes(
                    implementation,
                    black_box(dst),
                    black_box(src),
                    black_box(len),
                );
            })
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

criterion_group!(
    benches,
    basic_and_large_benches,
    overlap_delta_benches,
    same_pointer_benches,
    alignment_benches
);
criterion_main!(benches);
