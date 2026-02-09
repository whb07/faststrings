use core::ffi::c_char;
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use faststrings::str::{strlen as fast_strlen, strnlen as fast_strnlen};
use std::time::Duration;

unsafe extern "C" {
    #[link_name = "strlen"]
    fn libc_strlen(s: *const c_char) -> usize;
    #[link_name = "strnlen"]
    fn libc_strnlen(s: *const c_char, maxlen: usize) -> usize;
}

#[derive(Clone)]
struct StrlenCase {
    label: String,
    len: usize,
    nul_pos: usize,
}

#[derive(Clone)]
struct StrnlenCase {
    label: String,
    base_len: usize,
    nul_pos: usize,
    maxlen: usize,
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

fn strlen_benches(c: &mut Criterion) {
    let sizes = [31usize, 63, 256, 257, 1024, 4096, 65536];
    let mut cases = Vec::new();
    for len in sizes {
        cases.push(StrlenCase {
            label: format!("size_{len}_nul_head"),
            len,
            nul_pos: 0,
        });
        cases.push(StrlenCase {
            label: format!("size_{len}_nul_mid"),
            len,
            nul_pos: len / 2,
        });
        cases.push(StrlenCase {
            label: format!("size_{len}_nul_tail"),
            len,
            nul_pos: len,
        });
    }

    let mut group = c.benchmark_group("strlen");
    for case in &cases {
        let len = case.len;
        let mut buf = vec![0u8; len + 1 + 64];
        for i in 0..(len + 1) {
            let mut v = ((i * 37 + len * 11 + 3) % 251) as u8;
            if v == 0 {
                v = 1;
            }
            buf[i] = v;
        }
        buf[case.nul_pos] = 0;
        let s = &buf[..len + 1];

        configure_group_for_len(&mut group, len);
        group.throughput(Throughput::Bytes((case.nul_pos + 1) as u64));

        group.bench_with_input(BenchmarkId::new("glibc", &case.label), &len, |b, _| {
            b.iter(|| unsafe {
                black_box(libc_strlen(black_box(s.as_ptr() as *const c_char)));
            });
        });

        group.bench_with_input(
            BenchmarkId::new("faststrings", &case.label),
            &len,
            |b, _| {
                b.iter(|| {
                    black_box(fast_strlen(black_box(s)));
                });
            },
        );
    }
    group.finish();
}

fn strnlen_benches(c: &mut Criterion) {
    let sizes = [31usize, 63, 256, 257, 1024, 4096, 65536];
    let mut cases = Vec::new();
    for len in sizes {
        cases.push(StrnlenCase {
            label: format!("size_{len}_max_before_endnul"),
            base_len: len,
            nul_pos: len,
            maxlen: (len / 2).max(1),
        });
        cases.push(StrnlenCase {
            label: format!("size_{len}_max_at_endnul"),
            base_len: len,
            nul_pos: len,
            maxlen: len + 1,
        });
        cases.push(StrnlenCase {
            label: format!("size_{len}_max_after_endnul"),
            base_len: len,
            nul_pos: len,
            maxlen: len + 32,
        });
        cases.push(StrnlenCase {
            label: format!("size_{len}_midnul_max_after"),
            base_len: len,
            nul_pos: len / 2,
            maxlen: len + 32,
        });
    }

    let mut group = c.benchmark_group("strnlen");
    for case in &cases {
        let scan_len = case.maxlen.max(case.nul_pos + 1);
        let mut buf = vec![0u8; scan_len + 64];
        for i in 0..scan_len {
            let mut v = ((i * 17 + case.base_len * 7 + 5) % 251) as u8;
            if v == 0 {
                v = 1;
            }
            buf[i] = v;
        }
        buf[case.nul_pos] = 0;
        let s = &buf[..scan_len];

        configure_group_for_len(&mut group, scan_len);
        group.throughput(Throughput::Bytes(
            (case.maxlen.min(case.nul_pos + 1)) as u64,
        ));

        group.bench_with_input(
            BenchmarkId::new("glibc", &case.label),
            &(scan_len, case.maxlen),
            |b, &(_, maxlen)| {
                b.iter(|| unsafe {
                    black_box(libc_strnlen(
                        black_box(s.as_ptr() as *const c_char),
                        black_box(maxlen),
                    ));
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("faststrings", &case.label),
            &(scan_len, case.maxlen),
            |b, &(_, maxlen)| {
                b.iter(|| {
                    black_box(fast_strnlen(black_box(s), black_box(maxlen)));
                });
            },
        );
    }
    group.finish();
}

criterion_group!(benches, strlen_benches, strnlen_benches);
criterion_main!(benches);
