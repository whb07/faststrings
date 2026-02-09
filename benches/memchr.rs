use core::ffi::c_void;
use criterion::{BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main};
use faststrings::memchr::{optimized_memchr_unified, optimized_memrchr_unified};
use std::time::Duration;

unsafe extern "C" {
    #[link_name = "memchr"]
    fn libc_memchr(s: *const c_void, c: i32, n: usize) -> *mut c_void;

    #[link_name = "memrchr"]
    fn libc_memrchr(s: *const c_void, c: i32, n: usize) -> *mut c_void;
}

#[derive(Copy, Clone)]
enum HitKind {
    Miss,
    HitFirst,
    HitMid,
    HitLast,
}

#[derive(Clone)]
struct SearchCase {
    label: String,
    len: usize,
    off: usize,
    kind: HitKind,
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

fn build_cases() -> Vec<SearchCase> {
    let mut cases = Vec::new();

    let sizes = [
        1usize, 2, 3, 4, 7, 8, 15, 16, 31, 32, 63, 64, 65, 95, 96, 127, 128, 129, 191, 192, 255,
        256, 257, 511, 512, 513, 1023, 1024, 4096, 65536, 262144,
    ];

    for len in sizes {
        cases.push(SearchCase {
            label: format!("size_{len}_miss"),
            len,
            off: 0,
            kind: HitKind::Miss,
        });
        cases.push(SearchCase {
            label: format!("size_{len}_hit_last"),
            len,
            off: 0,
            kind: HitKind::HitLast,
        });
    }

    let position_sizes = [
        1usize, 8, 64, 65, 128, 129, 256, 257, 512, 513, 1024, 4096, 65536, 262144,
    ];
    for len in position_sizes {
        cases.push(SearchCase {
            label: format!("size_{len}_hit_first"),
            len,
            off: 0,
            kind: HitKind::HitFirst,
        });
        cases.push(SearchCase {
            label: format!("size_{len}_hit_mid"),
            len,
            off: 0,
            kind: HitKind::HitMid,
        });
    }

    let align_sizes = [64usize, 65, 256, 257, 4096];
    for len in align_sizes {
        for off in [0usize, 1, 15, 31] {
            cases.push(SearchCase {
                label: format!("align_len{len}_o{off}_miss"),
                len,
                off,
                kind: HitKind::Miss,
            });
            cases.push(SearchCase {
                label: format!("align_len{len}_o{off}_hit_last"),
                len,
                off,
                kind: HitKind::HitLast,
            });
        }
    }

    cases
}

fn make_buffer(case: &SearchCase, needle: u8) -> Vec<u8> {
    let mut buf = vec![0u8; case.len + 64];
    for (i, b) in buf.iter_mut().enumerate() {
        let mut v = (i % 251) as u8;
        if v == needle {
            v ^= 0x5C;
        }
        *b = v;
    }

    if case.len > 0 {
        let base = case.off;
        let idx = match case.kind {
            HitKind::Miss => None,
            HitKind::HitFirst => Some(0usize),
            HitKind::HitMid => Some(case.len / 2),
            HitKind::HitLast => Some(case.len - 1),
        };
        if let Some(i) = idx {
            buf[base + i] = needle;
        }
    }

    buf
}

fn memchr_benches(c: &mut Criterion) {
    let needle = 0xA5u8;
    let cases = build_cases();

    let mut fwd = c.benchmark_group("memchr");
    for case in &cases {
        let len = case.len;
        let buf = make_buffer(case, needle);
        let ptr = unsafe { buf.as_ptr().add(case.off) };

        configure_group_for_len(&mut fwd, len);
        fwd.throughput(Throughput::Bytes(len as u64));

        fwd.bench_with_input(BenchmarkId::new("glibc", &case.label), &len, |b, &n| {
            b.iter(|| unsafe {
                let r = libc_memchr(
                    black_box(ptr as *const c_void),
                    black_box(needle as i32),
                    black_box(n),
                );
                black_box(r);
            });
        });

        fwd.bench_with_input(
            BenchmarkId::new("faststrings", &case.label),
            &len,
            |b, &n| {
                b.iter(|| unsafe {
                    let r =
                        optimized_memchr_unified(black_box(ptr), black_box(n), black_box(needle));
                    black_box(r);
                });
            },
        );
    }
    fwd.finish();

    let mut rev = c.benchmark_group("memrchr");
    for case in &cases {
        let len = case.len;
        let buf = make_buffer(case, needle);
        let ptr = unsafe { buf.as_ptr().add(case.off) };

        configure_group_for_len(&mut rev, len);
        rev.throughput(Throughput::Bytes(len as u64));

        rev.bench_with_input(BenchmarkId::new("glibc", &case.label), &len, |b, &n| {
            b.iter(|| unsafe {
                let r = libc_memrchr(
                    black_box(ptr as *const c_void),
                    black_box(needle as i32),
                    black_box(n),
                );
                black_box(r);
            });
        });

        rev.bench_with_input(
            BenchmarkId::new("faststrings", &case.label),
            &len,
            |b, &n| {
                b.iter(|| unsafe {
                    let r =
                        optimized_memrchr_unified(black_box(ptr), black_box(n), black_box(needle));
                    black_box(r);
                });
            },
        );
    }
    rev.finish();
}

criterion_group!(benches, memchr_benches);
criterion_main!(benches);
