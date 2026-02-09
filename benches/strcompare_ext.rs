use core::ffi::c_char;
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use faststrings::str::{
    strcasecmp as fast_strcasecmp, strcoll as fast_strcoll, strncasecmp as fast_strncasecmp,
    strverscmp as fast_strverscmp,
};
use std::time::Duration;

unsafe extern "C" {
    #[link_name = "strverscmp"]
    fn libc_strverscmp(s1: *const c_char, s2: *const c_char) -> i32;
    #[link_name = "strcoll"]
    fn libc_strcoll(s1: *const c_char, s2: *const c_char) -> i32;
    #[link_name = "strcasecmp"]
    fn libc_strcasecmp(s1: *const c_char, s2: *const c_char) -> i32;
    #[link_name = "strncasecmp"]
    fn libc_strncasecmp(s1: *const c_char, s2: *const c_char, n: usize) -> i32;
}

#[derive(Copy, Clone)]
enum CmpKind {
    Equal,
    DiffFirst,
    DiffMid,
    DiffLast,
    LhsShorter,
    RhsShorter,
}

#[derive(Clone)]
struct CmpCase {
    label: String,
    len: usize,
    kind: CmpKind,
}

#[derive(Copy, Clone)]
enum NCaseKind {
    EqualNSmall,
    EqualNExact,
    DiffAfterNSmall,
    DiffBeforeNSmall,
    LhsShorterNOver,
    RhsShorterNOver,
}

#[derive(Clone)]
struct NCase {
    label: String,
    len: usize,
    kind: NCaseKind,
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

fn make_base(len: usize) -> Vec<u8> {
    let mut out = vec![0u8; len + 1];
    for i in 0..len {
        out[i] = b'a' + ((i * 7 + len * 11 + 3) % 23) as u8;
    }
    out[len] = 0;
    out
}

fn to_upper_ascii_in_place(buf: &mut [u8]) {
    for b in buf {
        *b = b.to_ascii_uppercase();
    }
}

fn build_cmp_inputs(case: &CmpCase, ignore_case: bool) -> (Vec<u8>, Vec<u8>, u64) {
    let len = case.len;
    let mut lhs = make_base(len);
    let mut rhs = make_base(len);
    if ignore_case {
        to_upper_ascii_in_place(&mut rhs[..len]);
    }

    let work = match case.kind {
        CmpKind::Equal => len.max(1) as u64,
        CmpKind::DiffFirst => {
            rhs[0] = b'z';
            1
        }
        CmpKind::DiffMid => {
            rhs[len / 2] = b'z';
            (len / 2 + 1) as u64
        }
        CmpKind::DiffLast => {
            rhs[len - 1] = b'z';
            len as u64
        }
        CmpKind::LhsShorter => {
            let short = (len / 2).max(1);
            lhs[short] = 0;
            (short + 1) as u64
        }
        CmpKind::RhsShorter => {
            let short = (len / 2).max(1);
            rhs[short] = 0;
            (short + 1) as u64
        }
    };

    (lhs, rhs, work.max(1))
}

fn build_ncasecmp_inputs(case: &NCase) -> (Vec<u8>, Vec<u8>, usize, u64) {
    let len = case.len;
    let n_small = (len / 2).max(1);
    let n_exact = len + 1;
    let n_over = len + 32;

    let lhs = make_base(len);
    let mut rhs = make_base(len);
    to_upper_ascii_in_place(&mut rhs[..len]);

    match case.kind {
        NCaseKind::EqualNSmall => (lhs, rhs, n_small, n_small as u64),
        NCaseKind::EqualNExact => (lhs, rhs, n_exact, len as u64),
        NCaseKind::DiffAfterNSmall => {
            rhs[len - 1] = b'z';
            (lhs, rhs, n_small, n_small as u64)
        }
        NCaseKind::DiffBeforeNSmall => {
            rhs[0] = b'z';
            (lhs, rhs, n_small, 1)
        }
        NCaseKind::LhsShorterNOver => {
            let mut lhs_short = make_base(len);
            let short = (len / 2).max(1);
            lhs_short[short] = 0;
            (lhs_short, rhs, n_over, (short + 1) as u64)
        }
        NCaseKind::RhsShorterNOver => {
            let short = (len / 2).max(1);
            rhs[short] = 0;
            (lhs, rhs, n_over, (short + 1) as u64)
        }
    }
}

fn strverscmp_case(left: &str, right: &str) -> (Vec<u8>, Vec<u8>, u64) {
    let mut lhs = left.as_bytes().to_vec();
    let mut rhs = right.as_bytes().to_vec();
    lhs.push(0);
    rhs.push(0);
    (lhs, rhs, left.len().max(right.len()) as u64)
}

fn strverscmp_benches(c: &mut Criterion) {
    let cases = [
        ("equal_plain", "v1.2.3", "v1.2.3"),
        ("numeric_less", "v1.2.3", "v1.2.10"),
        ("numeric_greater", "v2.0", "v1.99"),
        ("leading_zero_vs_plain", "v01", "v1"),
        ("plain_vs_leading_zero", "v1", "v01"),
        ("prefix_shorter", "release-1", "release-1a"),
        ("prefix_longer", "release-1a", "release-1"),
        ("long_numeric_run", "build-000123", "build-123"),
        ("long_tail_diff", "pkg-1.0.0-alpha2", "pkg-1.0.0-alpha10"),
        ("equal_long", "module-2026.02.09", "module-2026.02.09"),
    ];

    let mut group = c.benchmark_group("strverscmp");
    for (label, left, right) in cases {
        let (lhs, rhs, work) = strverscmp_case(left, right);
        group.sample_size(40);
        group.warm_up_time(Duration::from_millis(200));
        group.measurement_time(Duration::from_millis(500));
        group.throughput(Throughput::Bytes(work.max(1)));

        group.bench_with_input(BenchmarkId::new("glibc", label), &work, |b, _| {
            b.iter(|| unsafe {
                black_box(libc_strverscmp(
                    black_box(lhs.as_ptr() as *const c_char),
                    black_box(rhs.as_ptr() as *const c_char),
                ));
            });
        });

        group.bench_with_input(BenchmarkId::new("faststrings", label), &work, |b, _| {
            b.iter(|| {
                black_box(fast_strverscmp(black_box(&lhs), black_box(&rhs)));
            });
        });
    }
    group.finish();
}

fn strcoll_benches(c: &mut Criterion) {
    let sizes = [31usize, 256, 4096];
    let mut cases = Vec::new();
    for len in sizes {
        cases.push(CmpCase {
            label: format!("size_{len}_equal"),
            len,
            kind: CmpKind::Equal,
        });
        cases.push(CmpCase {
            label: format!("size_{len}_diff_first"),
            len,
            kind: CmpKind::DiffFirst,
        });
        cases.push(CmpCase {
            label: format!("size_{len}_diff_mid"),
            len,
            kind: CmpKind::DiffMid,
        });
        cases.push(CmpCase {
            label: format!("size_{len}_diff_last"),
            len,
            kind: CmpKind::DiffLast,
        });
        cases.push(CmpCase {
            label: format!("size_{len}_lhs_shorter"),
            len,
            kind: CmpKind::LhsShorter,
        });
        cases.push(CmpCase {
            label: format!("size_{len}_rhs_shorter"),
            len,
            kind: CmpKind::RhsShorter,
        });
    }

    let mut group = c.benchmark_group("strcoll");
    for case in &cases {
        let (lhs, rhs, work) = build_cmp_inputs(case, false);

        configure_group_for_len(&mut group, case.len);
        group.throughput(Throughput::Bytes(work));

        group.bench_with_input(BenchmarkId::new("glibc", &case.label), &case.len, |b, _| {
            b.iter(|| unsafe {
                black_box(libc_strcoll(
                    black_box(lhs.as_ptr() as *const c_char),
                    black_box(rhs.as_ptr() as *const c_char),
                ));
            });
        });

        group.bench_with_input(
            BenchmarkId::new("faststrings", &case.label),
            &case.len,
            |b, _| {
                b.iter(|| {
                    black_box(fast_strcoll(black_box(&lhs), black_box(&rhs)));
                });
            },
        );
    }
    group.finish();
}

fn strcasecmp_benches(c: &mut Criterion) {
    let sizes = [31usize, 256, 4096];
    let mut cases = Vec::new();
    for len in sizes {
        cases.push(CmpCase {
            label: format!("size_{len}_equal"),
            len,
            kind: CmpKind::Equal,
        });
        cases.push(CmpCase {
            label: format!("size_{len}_diff_first"),
            len,
            kind: CmpKind::DiffFirst,
        });
        cases.push(CmpCase {
            label: format!("size_{len}_diff_mid"),
            len,
            kind: CmpKind::DiffMid,
        });
        cases.push(CmpCase {
            label: format!("size_{len}_diff_last"),
            len,
            kind: CmpKind::DiffLast,
        });
        cases.push(CmpCase {
            label: format!("size_{len}_lhs_shorter"),
            len,
            kind: CmpKind::LhsShorter,
        });
        cases.push(CmpCase {
            label: format!("size_{len}_rhs_shorter"),
            len,
            kind: CmpKind::RhsShorter,
        });
    }

    let mut group = c.benchmark_group("strcasecmp");
    for case in &cases {
        let (lhs, rhs, work) = build_cmp_inputs(case, true);

        configure_group_for_len(&mut group, case.len);
        group.throughput(Throughput::Bytes(work));

        group.bench_with_input(BenchmarkId::new("glibc", &case.label), &case.len, |b, _| {
            b.iter(|| unsafe {
                black_box(libc_strcasecmp(
                    black_box(lhs.as_ptr() as *const c_char),
                    black_box(rhs.as_ptr() as *const c_char),
                ));
            });
        });

        group.bench_with_input(
            BenchmarkId::new("faststrings", &case.label),
            &case.len,
            |b, _| {
                b.iter(|| {
                    black_box(fast_strcasecmp(black_box(&lhs), black_box(&rhs)));
                });
            },
        );
    }
    group.finish();
}

fn strncasecmp_benches(c: &mut Criterion) {
    let sizes = [31usize, 256, 4096];
    let mut cases = Vec::new();
    for len in sizes {
        cases.push(NCase {
            label: format!("size_{len}_equal_n_small"),
            len,
            kind: NCaseKind::EqualNSmall,
        });
        cases.push(NCase {
            label: format!("size_{len}_equal_n_exact"),
            len,
            kind: NCaseKind::EqualNExact,
        });
        cases.push(NCase {
            label: format!("size_{len}_diff_after_n_small"),
            len,
            kind: NCaseKind::DiffAfterNSmall,
        });
        cases.push(NCase {
            label: format!("size_{len}_diff_before_n_small"),
            len,
            kind: NCaseKind::DiffBeforeNSmall,
        });
        cases.push(NCase {
            label: format!("size_{len}_lhs_shorter_n_over"),
            len,
            kind: NCaseKind::LhsShorterNOver,
        });
        cases.push(NCase {
            label: format!("size_{len}_rhs_shorter_n_over"),
            len,
            kind: NCaseKind::RhsShorterNOver,
        });
    }

    let mut group = c.benchmark_group("strncasecmp");
    for case in &cases {
        let (lhs, rhs, n, work) = build_ncasecmp_inputs(case);

        configure_group_for_len(&mut group, case.len);
        group.throughput(Throughput::Bytes(work.max(1)));

        group.bench_with_input(
            BenchmarkId::new("glibc", &case.label),
            &(case.len, n),
            |b, &(_, n)| {
                b.iter(|| unsafe {
                    black_box(libc_strncasecmp(
                        black_box(lhs.as_ptr() as *const c_char),
                        black_box(rhs.as_ptr() as *const c_char),
                        black_box(n),
                    ));
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("faststrings", &case.label),
            &(case.len, n),
            |b, &(_, n)| {
                b.iter(|| {
                    black_box(fast_strncasecmp(black_box(&lhs), black_box(&rhs), black_box(n)));
                });
            },
        );
    }
    group.finish();
}

criterion_group!(
    benches,
    strverscmp_benches,
    strcoll_benches,
    strcasecmp_benches,
    strncasecmp_benches
);
criterion_main!(benches);
