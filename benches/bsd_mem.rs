use core::ffi::c_void;
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use faststrings::mem::{
    bcmp as fast_bcmp, bzero as fast_bzero, explicit_bzero as fast_explicit_bzero,
};
use std::time::Duration;

unsafe extern "C" {
    #[link_name = "bcmp"]
    fn libc_bcmp(s1: *const c_void, s2: *const c_void, n: usize) -> i32;
    #[link_name = "bzero"]
    fn libc_bzero(s: *mut c_void, n: usize);
    #[link_name = "explicit_bzero"]
    fn libc_explicit_bzero(s: *mut c_void, n: usize);
}

#[derive(Copy, Clone)]
enum CompareKind {
    Equal,
    DiffFirst,
    DiffLast,
    DiffMiddle,
}

#[derive(Clone)]
struct CompareCase {
    label: String,
    len: usize,
    kind: CompareKind,
}

#[derive(Clone)]
struct ZeroCase {
    label: String,
    len: usize,
    align: usize,
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

fn bcmp_benches(c: &mut Criterion) {
    let sizes = [31usize, 63, 256, 257, 1024, 4096, 65536];
    let mut cases = Vec::new();
    for len in sizes {
        cases.push(CompareCase {
            label: format!("size_{len}_equal"),
            len,
            kind: CompareKind::Equal,
        });
        cases.push(CompareCase {
            label: format!("size_{len}_diff_first"),
            len,
            kind: CompareKind::DiffFirst,
        });
        cases.push(CompareCase {
            label: format!("size_{len}_diff_mid"),
            len,
            kind: CompareKind::DiffMiddle,
        });
        cases.push(CompareCase {
            label: format!("size_{len}_diff_last"),
            len,
            kind: CompareKind::DiffLast,
        });
    }

    let mut group = c.benchmark_group("bcmp");
    for case in &cases {
        let len = case.len;
        let mut left = vec![0u8; len + 64];
        let mut right = vec![0u8; len + 64];
        for i in 0..(len + 64) {
            let v = ((i * 17 + len * 3) % 251) as u8;
            left[i] = v;
            right[i] = v;
        }

        let lhs = &mut left[32..32 + len];
        let rhs = &mut right[32..32 + len];
        match case.kind {
            CompareKind::Equal => {}
            CompareKind::DiffFirst => rhs[0] ^= 0x7F,
            CompareKind::DiffMiddle => rhs[len / 2] ^= 0x7F,
            CompareKind::DiffLast => rhs[len - 1] ^= 0x7F,
        }

        configure_group_for_len(&mut group, len);
        group.throughput(Throughput::Bytes(len as u64));

        group.bench_with_input(BenchmarkId::new("glibc", &case.label), &len, |b, &n| {
            b.iter(|| unsafe {
                let result = libc_bcmp(
                    black_box(lhs.as_ptr() as *const c_void),
                    black_box(rhs.as_ptr() as *const c_void),
                    black_box(n),
                );
                black_box(result)
            });
        });

        group.bench_with_input(
            BenchmarkId::new("faststrings", &case.label),
            &len,
            |b, &n| {
                b.iter(|| black_box(fast_bcmp(black_box(&lhs[..n]), black_box(&rhs[..n]))));
            },
        );
    }
    group.finish();
}

fn bzero_benches(c: &mut Criterion) {
    let sizes = [31usize, 63, 256, 257, 1024, 4096, 65536];
    let mut cases = Vec::new();
    for len in sizes {
        for align in [0usize, 1, 31] {
            cases.push(ZeroCase {
                label: format!("size_{len}_align_{align}"),
                len,
                align,
            });
        }
    }

    let mut group = c.benchmark_group("bzero");
    for case in &cases {
        let len = case.len;
        let mut buf = vec![0xA5u8; len + case.align + 64];
        let ptr = unsafe { buf.as_mut_ptr().add(case.align) };

        configure_group_for_len(&mut group, len);
        group.throughput(Throughput::Bytes(len as u64));

        group.bench_with_input(BenchmarkId::new("glibc", &case.label), &len, |b, &n| {
            b.iter(|| unsafe {
                libc_bzero(black_box(ptr as *mut c_void), black_box(n));
                black_box(core::ptr::read_volatile(ptr));
            });
        });

        group.bench_with_input(
            BenchmarkId::new("faststrings", &case.label),
            &len,
            |b, &n| {
                b.iter(|| unsafe {
                    let dst = core::slice::from_raw_parts_mut(black_box(ptr), n);
                    black_box(fast_bzero(dst));
                    black_box(core::ptr::read_volatile(ptr));
                });
            },
        );
    }
    group.finish();
}

fn explicit_bzero_benches(c: &mut Criterion) {
    let sizes = [31usize, 63, 256, 257, 1024, 4096, 65536];
    let mut cases = Vec::new();
    for len in sizes {
        for align in [0usize, 1, 31] {
            cases.push(ZeroCase {
                label: format!("size_{len}_align_{align}"),
                len,
                align,
            });
        }
    }

    let mut group = c.benchmark_group("explicit_bzero");
    for case in &cases {
        let len = case.len;
        let mut buf = vec![0x5Au8; len + case.align + 64];
        let ptr = unsafe { buf.as_mut_ptr().add(case.align) };

        configure_group_for_len(&mut group, len);
        group.throughput(Throughput::Bytes(len as u64));

        group.bench_with_input(BenchmarkId::new("glibc", &case.label), &len, |b, &n| {
            b.iter(|| unsafe {
                libc_explicit_bzero(black_box(ptr as *mut c_void), black_box(n));
                black_box(core::ptr::read_volatile(ptr));
            });
        });

        group.bench_with_input(
            BenchmarkId::new("faststrings", &case.label),
            &len,
            |b, &n| {
                b.iter(|| unsafe {
                    let dst = core::slice::from_raw_parts_mut(black_box(ptr), n);
                    fast_explicit_bzero(dst);
                    black_box(core::ptr::read_volatile(ptr));
                });
            },
        );
    }
    group.finish();
}

criterion_group!(benches, bcmp_benches, bzero_benches, explicit_bzero_benches);
criterion_main!(benches);
