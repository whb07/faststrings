use core::ffi::c_void;
use criterion::{BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main};
use faststrings::memcpy::optimized_memcpy_unified;
use std::time::Duration;

unsafe extern "C" {
    #[link_name = "memcpy"]
    fn libc_memcpy(dest: *mut c_void, src: *const c_void, n: usize) -> *mut c_void;
}

#[derive(Clone)]
struct CopyCase {
    label: String,
    len: usize,
    src_off: usize,
    dst_off: usize,
}

fn configure_group_for_len(
    group: &mut criterion::BenchmarkGroup<'_, criterion::measurement::WallTime>,
    len: usize,
) {
    if len >= 1 << 20 {
        group.sample_size(20);
        group.warm_up_time(Duration::from_millis(300));
        group.measurement_time(Duration::from_millis(900));
    } else if len >= 1 << 16 {
        group.sample_size(30);
        group.warm_up_time(Duration::from_millis(250));
        group.measurement_time(Duration::from_millis(700));
    } else {
        group.sample_size(40);
        group.warm_up_time(Duration::from_millis(200));
        group.measurement_time(Duration::from_millis(500));
    }
}

fn memcpy_benches(c: &mut Criterion) {
    let mut cases = Vec::new();

    // Size sweep includes threshold boundaries and cliff zones.
    let sizes = [
        1usize,
        2,
        3,
        4,
        7,
        8,
        15,
        16,
        31,
        32,
        63,
        64,
        65,
        95,
        96,
        127,
        128,
        129,
        191,
        192,
        255,
        256,
        257,
        511,
        512,
        513,
        1023,
        1024,
        4095,
        4096,
        65535,
        65536,
        262143,
        262144,
        262145,
        (8 * 1024 * 1024) - 1,
        8 * 1024 * 1024,
        (8 * 1024 * 1024) + 1,
    ];

    for len in sizes {
        cases.push(CopyCase {
            label: format!("size_{len}"),
            len,
            src_off: 0,
            dst_off: 0,
        });
    }

    // Alignment sweep at representative cliff sizes.
    let align_sizes = [63usize, 64, 65, 256, 257, 4096];
    let align_pairs = [(0usize, 0usize), (1, 1), (15, 7), (31, 17)];
    for len in align_sizes {
        for (src_off, dst_off) in align_pairs {
            cases.push(CopyCase {
                label: format!("align_len{len}_s{src_off}_d{dst_off}"),
                len,
                src_off,
                dst_off,
            });
        }
    }

    let mut group = c.benchmark_group("memcpy");

    for case in &cases {
        let len = case.len;
        let src_off = case.src_off;
        let dst_off = case.dst_off;

        let alloc_len = len + 64;
        let mut src = vec![0u8; alloc_len];
        let mut dst = vec![0u8; alloc_len];
        for (i, byte) in src.iter_mut().enumerate() {
            *byte = (i % 251) as u8;
        }

        let src_ptr = unsafe { src.as_ptr().add(src_off) };
        let dst_ptr = unsafe { dst.as_mut_ptr().add(dst_off) };

        configure_group_for_len(&mut group, len);
        group.throughput(Throughput::Bytes(len as u64));

        group.bench_with_input(BenchmarkId::new("glibc", &case.label), &len, |b, &n| {
            b.iter(|| unsafe {
                libc_memcpy(
                    black_box(dst_ptr as *mut c_void),
                    black_box(src_ptr as *const c_void),
                    black_box(n),
                );
                black_box(core::ptr::read_volatile(dst_ptr));
            });
        });

        group.bench_with_input(
            BenchmarkId::new("faststrings", &case.label),
            &len,
            |b, &n| {
                b.iter(|| unsafe {
                    optimized_memcpy_unified(black_box(dst_ptr), black_box(src_ptr), black_box(n));
                    black_box(core::ptr::read_volatile(dst_ptr));
                });
            },
        );
    }

    group.finish();
}

criterion_group!(benches, memcpy_benches);
criterion_main!(benches);
