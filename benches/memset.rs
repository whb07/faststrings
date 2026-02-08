use core::ffi::c_void;
use criterion::{BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main};
use faststrings::memset::optimized_memset_unified;
use std::time::Duration;

unsafe extern "C" {
    #[link_name = "memset"]
    fn libc_memset(dest: *mut c_void, c: i32, n: usize) -> *mut c_void;
}

#[derive(Clone)]
struct SetCase {
    label: String,
    len: usize,
    dst_off: usize,
    value: u8,
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

fn memset_benches(c: &mut Criterion) {
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
        1024 * 1024,
    ];

    for len in sizes {
        cases.push(SetCase {
            label: format!("size_{len}"),
            len,
            dst_off: 0,
            value: 0x5A,
        });
    }

    // Alignment sweep at representative cliff sizes.
    let align_sizes = [64usize, 65, 256, 257, 4096];
    for len in align_sizes {
        for dst_off in [0usize, 1, 15, 31] {
            cases.push(SetCase {
                label: format!("align_len{len}_d{dst_off}"),
                len,
                dst_off,
                value: 0xA5,
            });
        }
    }

    // Value sweep for likely fast paths/special values.
    for len in [64usize, 65, 4096, 262144] {
        for value in [0x00u8, 0x5A, 0xFF] {
            cases.push(SetCase {
                label: format!("value_len{len}_v{value:02x}"),
                len,
                dst_off: 0,
                value,
            });
        }
    }

    let mut group = c.benchmark_group("memset");

    for case in &cases {
        let len = case.len;
        let dst_off = case.dst_off;
        let value = case.value;

        let mut dst = vec![0u8; len + 64];
        let dst_ptr = unsafe { dst.as_mut_ptr().add(dst_off) };

        configure_group_for_len(&mut group, len);
        group.throughput(Throughput::Bytes(len as u64));

        group.bench_with_input(BenchmarkId::new("glibc", &case.label), &len, |b, &n| {
            b.iter(|| unsafe {
                libc_memset(
                    black_box(dst_ptr as *mut c_void),
                    black_box(value as i32),
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
                    optimized_memset_unified(black_box(dst_ptr), black_box(value), black_box(n));
                    black_box(core::ptr::read_volatile(dst_ptr));
                });
            },
        );
    }

    group.finish();
}

criterion_group!(benches, memset_benches);
criterion_main!(benches);
