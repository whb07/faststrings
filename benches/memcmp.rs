use core::ffi::c_void;
use criterion::{BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main};
use faststrings::memcmp::optimized_memcmp_unified;
use std::time::Duration;

unsafe extern "C" {
    #[link_name = "memcmp"]
    fn libc_memcmp(s1: *const c_void, s2: *const c_void, n: usize) -> i32;
}

#[derive(Copy, Clone)]
enum CompareKind {
    Equal,
    DiffFirst,
    DiffMid,
    DiffLast,
}

#[derive(Clone)]
struct CompareCase {
    label: String,
    len: usize,
    s1_off: usize,
    s2_off: usize,
    kind: CompareKind,
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

fn memcmp_benches(c: &mut Criterion) {
    let mut cases = Vec::new();

    let sizes = [
        1usize, 2, 3, 4, 7, 8, 15, 16, 31, 32, 63, 64, 65, 95, 96, 127, 128, 129, 191, 192, 255,
        256, 257, 511, 512, 513, 1023, 1024, 4096, 65536, 262144, 1048576,
    ];

    for len in sizes {
        cases.push(CompareCase {
            label: format!("size_{len}_equal"),
            len,
            s1_off: 0,
            s2_off: 0,
            kind: CompareKind::Equal,
        });
        cases.push(CompareCase {
            label: format!("size_{len}_diff_last"),
            len,
            s1_off: 0,
            s2_off: 0,
            kind: CompareKind::DiffLast,
        });
    }

    let diff_position_sizes = [1usize, 8, 64, 65, 128, 129, 256, 257, 512, 513, 1024, 4096, 65536, 262144];
    for len in diff_position_sizes {
        cases.push(CompareCase {
            label: format!("size_{len}_diff_first"),
            len,
            s1_off: 0,
            s2_off: 0,
            kind: CompareKind::DiffFirst,
        });
        cases.push(CompareCase {
            label: format!("size_{len}_diff_mid"),
            len,
            s1_off: 0,
            s2_off: 0,
            kind: CompareKind::DiffMid,
        });
    }

    let align_sizes = [64usize, 65, 256, 257, 4096];
    let align_pairs = [(0usize, 0usize), (1, 1), (15, 7), (31, 17)];
    for len in align_sizes {
        for (s1_off, s2_off) in align_pairs {
            cases.push(CompareCase {
                label: format!("align_len{len}_s{s1_off}_d{s2_off}_equal"),
                len,
                s1_off,
                s2_off,
                kind: CompareKind::Equal,
            });
            cases.push(CompareCase {
                label: format!("align_len{len}_s{s1_off}_d{s2_off}_diff_last"),
                len,
                s1_off,
                s2_off,
                kind: CompareKind::DiffLast,
            });
        }
    }

    let mut group = c.benchmark_group("memcmp");

    for case in &cases {
        let len = case.len;
        let alloc_len = len + 64;
        let mut s1 = vec![0u8; alloc_len];
        let mut s2 = vec![0u8; alloc_len];

        for (i, byte) in s1.iter_mut().enumerate() {
            *byte = (i % 251) as u8;
        }
        s2.copy_from_slice(&s1);

        let s1_ptr = unsafe { s1.as_ptr().add(case.s1_off) };
        let s2_mut = unsafe { s2.as_mut_ptr().add(case.s2_off) };
        let s2_ptr = s2_mut as *const u8;

        if len > 0 {
            let diff_idx = match case.kind {
                CompareKind::Equal => None,
                CompareKind::DiffFirst => Some(0usize),
                CompareKind::DiffMid => Some(len / 2),
                CompareKind::DiffLast => Some(len - 1),
            };

            if let Some(idx) = diff_idx {
                unsafe {
                    let src = *s1_ptr.add(idx);
                    *s2_mut.add(idx) = src.wrapping_add(1);
                }
            }
        }

        configure_group_for_len(&mut group, len);
        group.throughput(Throughput::Bytes(len as u64));

        group.bench_with_input(BenchmarkId::new("glibc", &case.label), &len, |b, &n| {
            b.iter(|| unsafe {
                let r = libc_memcmp(
                    black_box(s1_ptr as *const c_void),
                    black_box(s2_ptr as *const c_void),
                    black_box(n),
                );
                black_box(r);
            });
        });

        group.bench_with_input(BenchmarkId::new("faststrings", &case.label), &len, |b, &n| {
            b.iter(|| unsafe {
                let r = optimized_memcmp_unified(black_box(s1_ptr), black_box(s2_ptr), black_box(n));
                black_box(r);
            });
        });
    }

    group.finish();
}

criterion_group!(benches, memcmp_benches);
criterion_main!(benches);
