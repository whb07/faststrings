use core::ffi::c_void;
use criterion::{BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main};
use faststrings::memmove::optimized_memmove_unified;

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

fn bench_pair(
    group: &mut criterion::BenchmarkGroup<'_, criterion::measurement::WallTime>,
    label: &str,
    len: usize,
    dst: *mut u8,
    src: *const u8,
) {
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

fn overlap_case(
    c: &mut Criterion,
    group_name: &str,
    label: &str,
    len: usize,
    delta: usize,
    direction: Direction,
    base_off: usize,
) {
    let mut group = c.benchmark_group(group_name);
    let mut buf = AlignedBuffer::new(len + delta + base_off + 4096, 4096);
    let base = buf.mut_ptr(base_off);
    let (src, dst) = match direction {
        Direction::Forward => (unsafe { base.add(delta) } as *const u8, base),
        Direction::Backward => (base as *const u8, unsafe { base.add(delta) }),
    };
    bench_pair(&mut group, label, len, dst, src);
    black_box(&mut buf);
    group.finish();
}

fn basic_and_large_benches(c: &mut Criterion) {
    for &len in &[
        1usize,
        16,
        31,
        32,
        63,
        64,
        65,
        256,
        1024,
        4096,
        256 * 1024,
        256 * 1024 + 1,
        1024 * 1024,
        2 * 1024 * 1024,
    ] {
        let delta = if len == 1 { 0 } else { 1 };
        overlap_case(
            c,
            "memmove_overlap_sizes",
            &format!("forward_len{len}_d{delta}"),
            len,
            delta,
            Direction::Forward,
            0,
        );
        overlap_case(
            c,
            "memmove_overlap_sizes",
            &format!("backward_len{len}_d{delta}"),
            len,
            delta,
            Direction::Backward,
            0,
        );
    }
}

fn overlap_delta_benches(c: &mut Criterion) {
    // Vector-width boundaries plus the small/large overlap cliffs.
    for &len in &[256usize, 4096, 256 * 1024, 1024 * 1024 + 1] {
        let candidates = [
            1usize,
            15,
            16,
            17,
            31,
            32,
            33,
            63,
            64,
            65,
            127,
            128,
            129,
            255,
            256,
            257,
            len - 1,
        ];
        for &delta in &candidates {
            if delta >= len {
                continue;
            }
            for &(direction, tag) in &[(Direction::Forward, "fwd"), (Direction::Backward, "bwd")] {
                overlap_case(
                    c,
                    "memmove_overlap_deltas",
                    &format!("{tag}_len{len}_d{delta}"),
                    len,
                    delta,
                    direction,
                    0,
                );
            }
        }
    }
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
    // Non-overlap permits source and destination alignment to vary independently.
    for &len in &[64usize, 257, 4096, 256 * 1024] {
        for &(src_off, dst_off) in &[(0usize, 0usize), (0, 1), (1, 0), (7, 31), (31, 7), (63, 32)] {
            let mut group = c.benchmark_group("memmove_nonoverlap_alignment");
            let mut src = AlignedBuffer::new(len + 64, 64);
            let mut dst = AlignedBuffer::new(len + 64, 64);
            bench_pair(
                &mut group,
                &format!("len{len}_s{src_off}_d{dst_off}"),
                len,
                dst.mut_ptr(dst_off),
                src.ptr(src_off),
            );
            black_box((&mut src, &mut dst));
            group.finish();
        }
    }

    // For overlap, delta constrains relative alignment; vary the absolute source alignment.
    for &(base_off, delta) in &[(0usize, 1usize), (1, 31), (7, 32), (31, 33), (63, 65)] {
        overlap_case(
            c,
            "memmove_overlap_alignment",
            &format!("len4096_base{base_off}_d{delta}"),
            4096,
            delta,
            Direction::Backward,
            base_off,
        );
    }
}

criterion_group!(
    benches,
    basic_and_large_benches,
    overlap_delta_benches,
    same_pointer_benches,
    alignment_benches
);
criterion_main!(benches);
