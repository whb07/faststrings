use core::ffi::c_void;
use criterion::{
    BatchSize, BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main,
};
use faststrings::memcpy::optimized_memcpy_unified;

#[path = "bench_support.rs"]
mod support;
use support::{AlignedBuffer, flush_range};

unsafe extern "C" {
    #[link_name = "memcpy"]
    fn libc_memcpy(dest: *mut c_void, src: *const c_void, n: usize) -> *mut c_void;
}

#[derive(Clone, Copy)]
enum Implementation {
    Glibc,
    Faststrings,
}

#[inline(always)]
unsafe fn copy(implementation: Implementation, dst: *mut u8, src: *const u8, len: usize) {
    match implementation {
        Implementation::Glibc => unsafe {
            libc_memcpy(dst.cast(), src.cast(), len);
        },
        Implementation::Faststrings => unsafe {
            optimized_memcpy_unified(dst, src, len);
        },
    }
    black_box(unsafe { core::ptr::read_volatile(dst) });
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
    for (name, implementation) in [
        ("glibc", Implementation::Glibc),
        ("faststrings", Implementation::Faststrings),
    ] {
        group.bench_function(BenchmarkId::new(name, label), |b| {
            b.iter(|| unsafe {
                copy(
                    implementation,
                    black_box(dst_ptr),
                    black_box(src_ptr),
                    black_box(len),
                );
            })
        });
    }
    black_box((&mut src, &mut dst));
    group.finish();
}

fn hot_and_threshold_benches(c: &mut Criterion) {
    for &len in &[
        1usize,
        7,
        16,
        31,
        32,
        62,
        63,
        64,
        65,
        127,
        128,
        129,
        255,
        256,
        257,
        511,
        512,
        513,
        1023,
        1024,
        1025,
        4096,
        65536,
        262144,
        16 * 1024 * 1024 - 1,
        16 * 1024 * 1024,
        16 * 1024 * 1024 + 1,
    ] {
        fixed_case(c, "memcpy_hot_fixed", &format!("size_{len}"), len, 0, 0);
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
    for &len in &[64usize, 257, 4096] {
        for &(src_off, dst_off) in &[(0usize, 0usize), (0, 1), (1, 0), (7, 31), (31, 7), (63, 32)] {
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
        for (name, implementation) in [
            ("glibc", Implementation::Glibc),
            ("faststrings", Implementation::Faststrings),
        ] {
            let mut index = 0usize;
            group.bench_function(BenchmarkId::new(name, label), |b| {
                b.iter(|| unsafe {
                    let off = index * chunk;
                    copy(implementation, dst.mut_ptr(off), src.ptr(off), chunk);
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
    for &len in &[64usize, 4096, 256 * 1024] {
        let mut group = c.benchmark_group("memcpy_cold_flushed");
        group.throughput(Throughput::Bytes(len as u64));
        let src = AlignedBuffer::new(len, 64);
        let mut dst = AlignedBuffer::new(len, 64);
        let src_ptr = src.ptr(0);
        let dst_ptr = dst.mut_ptr(0);
        for (name, implementation) in [
            ("glibc", Implementation::Glibc),
            ("faststrings", Implementation::Faststrings),
        ] {
            group.bench_function(BenchmarkId::new(name, format!("size_{len}")), |b| {
                b.iter_batched(
                    || unsafe {
                        flush_range(src_ptr, len);
                        flush_range(dst_ptr, len);
                    },
                    |_| unsafe {
                        copy(implementation, dst_ptr, src_ptr, len);
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
