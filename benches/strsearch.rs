use core::ffi::c_char;
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use faststrings::search::{
    strcasestr as fast_strcasestr, strcspn as fast_strcspn, strpbrk as fast_strpbrk,
    strstr as fast_strstr, strspn as fast_strspn,
};
use std::time::Duration;

unsafe extern "C" {
    #[link_name = "strstr"]
    fn libc_strstr(haystack: *const c_char, needle: *const c_char) -> *mut c_char;
    #[link_name = "strcasestr"]
    fn libc_strcasestr(haystack: *const c_char, needle: *const c_char) -> *mut c_char;
    #[link_name = "strspn"]
    fn libc_strspn(s: *const c_char, accept: *const c_char) -> usize;
    #[link_name = "strcspn"]
    fn libc_strcspn(s: *const c_char, reject: *const c_char) -> usize;
    #[link_name = "strpbrk"]
    fn libc_strpbrk(s: *const c_char, accept: *const c_char) -> *mut c_char;
}

#[derive(Copy, Clone)]
enum SubCaseKind {
    HitHead,
    HitMid,
    HitTail,
    Miss,
    EmptyNeedle,
}

#[derive(Copy, Clone)]
enum SpanCaseKind {
    HitFirst,
    HitMid,
    HitTail,
    Miss,
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

fn make_ascii_haystack(len: usize) -> Vec<u8> {
    let mut out = vec![0u8; len + 1];
    for (i, b) in out[..len].iter_mut().enumerate() {
        *b = b'a' + ((i * 7 + 3) % 5) as u8;
    }
    out[len] = 0;
    out
}

fn make_sub_case(len: usize, kind: SubCaseKind, fold_case: bool) -> (Vec<u8>, Vec<u8>, u64) {
    let mut hay = make_ascii_haystack(len);
    let plain = [b'q', b'r', b's', b't'];
    let query = if fold_case {
        [b'Q', b'R', b'S', b'T']
    } else {
        plain
    };
    let mut needle = Vec::with_capacity(plain.len() + 1);

    let mut work = len.max(1) as u64;
    match kind {
        SubCaseKind::HitHead => {
            hay[..plain.len()].copy_from_slice(&plain);
            needle.extend_from_slice(&query);
            work = plain.len() as u64;
        }
        SubCaseKind::HitMid => {
            let pos = len / 2;
            hay[pos..pos + plain.len()].copy_from_slice(&plain);
            needle.extend_from_slice(&query);
            work = (pos + plain.len()) as u64;
        }
        SubCaseKind::HitTail => {
            let pos = len - plain.len();
            hay[pos..pos + plain.len()].copy_from_slice(&plain);
            needle.extend_from_slice(&query);
            work = len as u64;
        }
        SubCaseKind::Miss => {
            needle.extend_from_slice(&query);
        }
        SubCaseKind::EmptyNeedle => {
            work = 1;
        }
    }

    if fold_case {
        for (idx, b) in hay[..len].iter_mut().enumerate() {
            if idx % 2 == 0 {
                *b = b.to_ascii_uppercase();
            }
        }
    }

    needle.push(0);
    (hay, needle, work.max(1))
}

fn make_spn_case(len: usize, kind: SpanCaseKind) -> (Vec<u8>, Vec<u8>, u64) {
    let mut s = vec![0u8; len + 1];
    for (i, b) in s[..len].iter_mut().enumerate() {
        *b = [b'a', b'b', b'c'][i % 3];
    }
    s[len] = 0;

    let accept = b"abc\0".to_vec();
    let work = match kind {
        SpanCaseKind::HitFirst => {
            s[0] = b'x';
            1
        }
        SpanCaseKind::HitMid => {
            let pos = len / 2;
            s[pos] = b'x';
            (pos + 1) as u64
        }
        SpanCaseKind::HitTail => {
            s[len - 1] = b'x';
            len as u64
        }
        SpanCaseKind::Miss => len as u64,
    };

    (s, accept, work.max(1))
}

fn make_cspn_case(len: usize, kind: SpanCaseKind) -> (Vec<u8>, Vec<u8>, u64) {
    let mut s = vec![0u8; len + 1];
    for (i, b) in s[..len].iter_mut().enumerate() {
        *b = [b'a', b'b', b'c'][i % 3];
    }
    s[len] = 0;

    let reject = b"x\0".to_vec();
    let work = match kind {
        SpanCaseKind::HitFirst => {
            s[0] = b'x';
            1
        }
        SpanCaseKind::HitMid => {
            let pos = len / 2;
            s[pos] = b'x';
            (pos + 1) as u64
        }
        SpanCaseKind::HitTail => {
            s[len - 1] = b'x';
            len as u64
        }
        SpanCaseKind::Miss => len as u64,
    };

    (s, reject, work.max(1))
}

fn strstr_benches(c: &mut Criterion) {
    let sizes = [31usize, 256, 4096];
    let cases = [
        ("hit_head", SubCaseKind::HitHead),
        ("hit_mid", SubCaseKind::HitMid),
        ("hit_tail", SubCaseKind::HitTail),
        ("miss", SubCaseKind::Miss),
        ("empty_needle", SubCaseKind::EmptyNeedle),
    ];

    let mut group = c.benchmark_group("strstr");
    for len in sizes {
        for (label, kind) in cases {
            let (hay, needle, work) = make_sub_case(len, kind, false);
            let case_label = format!("size_{len}_{label}");

            configure_group_for_len(&mut group, len);
            group.throughput(Throughput::Bytes(work));

            group.bench_with_input(BenchmarkId::new("glibc", &case_label), &len, |b, _| {
                b.iter(|| unsafe {
                    let base = hay.as_ptr() as usize;
                    let ptr = libc_strstr(
                        black_box(hay.as_ptr() as *const c_char),
                        black_box(needle.as_ptr() as *const c_char),
                    );
                    let rv = if ptr.is_null() {
                        usize::MAX
                    } else {
                        (ptr as usize) - base
                    };
                    black_box(rv);
                });
            });

            group.bench_with_input(
                BenchmarkId::new("faststrings", &case_label),
                &len,
                |b, _| {
                    b.iter(|| {
                        black_box(
                            fast_strstr(black_box(&hay), black_box(&needle)).unwrap_or(usize::MAX),
                        );
                    });
                },
            );
        }
    }
    group.finish();
}

fn strcasestr_benches(c: &mut Criterion) {
    let sizes = [31usize, 256, 4096];
    let cases = [
        ("hit_head", SubCaseKind::HitHead),
        ("hit_mid", SubCaseKind::HitMid),
        ("hit_tail", SubCaseKind::HitTail),
        ("miss", SubCaseKind::Miss),
        ("empty_needle", SubCaseKind::EmptyNeedle),
    ];

    let mut group = c.benchmark_group("strcasestr");
    for len in sizes {
        for (label, kind) in cases {
            let (hay, needle, work) = make_sub_case(len, kind, true);
            let case_label = format!("size_{len}_{label}");

            configure_group_for_len(&mut group, len);
            group.throughput(Throughput::Bytes(work));

            group.bench_with_input(BenchmarkId::new("glibc", &case_label), &len, |b, _| {
                b.iter(|| unsafe {
                    let base = hay.as_ptr() as usize;
                    let ptr = libc_strcasestr(
                        black_box(hay.as_ptr() as *const c_char),
                        black_box(needle.as_ptr() as *const c_char),
                    );
                    let rv = if ptr.is_null() {
                        usize::MAX
                    } else {
                        (ptr as usize) - base
                    };
                    black_box(rv);
                });
            });

            group.bench_with_input(
                BenchmarkId::new("faststrings", &case_label),
                &len,
                |b, _| {
                    b.iter(|| {
                        black_box(
                            fast_strcasestr(black_box(&hay), black_box(&needle))
                                .unwrap_or(usize::MAX),
                        );
                    });
                },
            );
        }
    }
    group.finish();
}

fn strspn_benches(c: &mut Criterion) {
    let sizes = [31usize, 256, 4096];
    let cases = [
        ("reject_first", SpanCaseKind::HitFirst),
        ("reject_mid", SpanCaseKind::HitMid),
        ("reject_tail", SpanCaseKind::HitTail),
        ("full_match", SpanCaseKind::Miss),
    ];

    let mut group = c.benchmark_group("strspn");
    for len in sizes {
        for (label, kind) in cases {
            let (s, accept, work) = make_spn_case(len, kind);
            let case_label = format!("size_{len}_{label}");

            configure_group_for_len(&mut group, len);
            group.throughput(Throughput::Bytes(work));

            group.bench_with_input(BenchmarkId::new("glibc", &case_label), &len, |b, _| {
                b.iter(|| unsafe {
                    black_box(libc_strspn(
                        black_box(s.as_ptr() as *const c_char),
                        black_box(accept.as_ptr() as *const c_char),
                    ));
                });
            });

            group.bench_with_input(
                BenchmarkId::new("faststrings", &case_label),
                &len,
                |b, _| {
                    b.iter(|| {
                        black_box(fast_strspn(black_box(&s), black_box(&accept)));
                    });
                },
            );
        }
    }
    group.finish();
}

fn strcspn_benches(c: &mut Criterion) {
    let sizes = [31usize, 256, 4096];
    let cases = [
        ("hit_first", SpanCaseKind::HitFirst),
        ("hit_mid", SpanCaseKind::HitMid),
        ("hit_tail", SpanCaseKind::HitTail),
        ("miss", SpanCaseKind::Miss),
    ];

    let mut group = c.benchmark_group("strcspn");
    for len in sizes {
        for (label, kind) in cases {
            let (s, reject, work) = make_cspn_case(len, kind);
            let case_label = format!("size_{len}_{label}");

            configure_group_for_len(&mut group, len);
            group.throughput(Throughput::Bytes(work));

            group.bench_with_input(BenchmarkId::new("glibc", &case_label), &len, |b, _| {
                b.iter(|| unsafe {
                    black_box(libc_strcspn(
                        black_box(s.as_ptr() as *const c_char),
                        black_box(reject.as_ptr() as *const c_char),
                    ));
                });
            });

            group.bench_with_input(
                BenchmarkId::new("faststrings", &case_label),
                &len,
                |b, _| {
                    b.iter(|| {
                        black_box(fast_strcspn(black_box(&s), black_box(&reject)));
                    });
                },
            );
        }
    }
    group.finish();
}

fn strpbrk_benches(c: &mut Criterion) {
    let sizes = [31usize, 256, 4096];
    let cases = [
        ("hit_first", SpanCaseKind::HitFirst),
        ("hit_mid", SpanCaseKind::HitMid),
        ("hit_tail", SpanCaseKind::HitTail),
        ("miss", SpanCaseKind::Miss),
    ];

    let mut group = c.benchmark_group("strpbrk");
    for len in sizes {
        for (label, kind) in cases {
            let (s, accept, work) = make_cspn_case(len, kind);
            let case_label = format!("size_{len}_{label}");

            configure_group_for_len(&mut group, len);
            group.throughput(Throughput::Bytes(work));

            group.bench_with_input(BenchmarkId::new("glibc", &case_label), &len, |b, _| {
                b.iter(|| unsafe {
                    let base = s.as_ptr() as usize;
                    let ptr = libc_strpbrk(
                        black_box(s.as_ptr() as *const c_char),
                        black_box(accept.as_ptr() as *const c_char),
                    );
                    let rv = if ptr.is_null() {
                        usize::MAX
                    } else {
                        (ptr as usize) - base
                    };
                    black_box(rv);
                });
            });

            group.bench_with_input(
                BenchmarkId::new("faststrings", &case_label),
                &len,
                |b, _| {
                    b.iter(|| {
                        black_box(
                            fast_strpbrk(black_box(&s), black_box(&accept)).unwrap_or(usize::MAX),
                        );
                    });
                },
            );
        }
    }
    group.finish();
}

criterion_group!(
    benches,
    strstr_benches,
    strcasestr_benches,
    strspn_benches,
    strcspn_benches,
    strpbrk_benches
);
criterion_main!(benches);
