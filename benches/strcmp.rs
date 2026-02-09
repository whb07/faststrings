use core::ffi::c_char;
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use faststrings::str::{strcmp as fast_strcmp, strncmp as fast_strncmp};
use std::time::Duration;

unsafe extern "C" {
    #[link_name = "strcmp"]
    fn libc_strcmp(s1: *const c_char, s2: *const c_char) -> i32;
    #[link_name = "strncmp"]
    fn libc_strncmp(s1: *const c_char, s2: *const c_char, n: usize) -> i32;
}

#[derive(Copy, Clone)]
enum StrcmpKind {
    Equal,
    DiffFirst,
    DiffMid,
    DiffLast,
    LhsShorter,
    RhsShorter,
}

#[derive(Clone)]
struct StrcmpCase {
    label: String,
    len: usize,
    kind: StrcmpKind,
}

#[derive(Copy, Clone)]
enum StrncmpKind {
    EqualNSmall,
    EqualNExact,
    DiffAfterNSmall,
    DiffBeforeNSmall,
    LhsShorterNOver,
    RhsShorterNOver,
}

#[derive(Clone)]
struct StrncmpCase {
    label: String,
    len: usize,
    kind: StrncmpKind,
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
        out[i] = b'a' + ((i % 23) as u8);
    }
    out[len] = 0;
    out
}

fn build_strcmp_inputs(case: &StrcmpCase) -> (Vec<u8>, Vec<u8>, u64) {
    let len = case.len;
    let mut lhs = make_base(len);
    let mut rhs = make_base(len);

    let work = match case.kind {
        StrcmpKind::Equal => len.max(1) as u64,
        StrcmpKind::DiffFirst => {
            rhs[0] = b'z';
            1
        }
        StrcmpKind::DiffMid => {
            rhs[len / 2] = b'z';
            (len / 2 + 1) as u64
        }
        StrcmpKind::DiffLast => {
            rhs[len - 1] = b'z';
            len as u64
        }
        StrcmpKind::LhsShorter => {
            let short = (len / 2).max(1);
            lhs[short] = 0;
            (short + 1) as u64
        }
        StrcmpKind::RhsShorter => {
            let short = (len / 2).max(1);
            rhs[short] = 0;
            (short + 1) as u64
        }
    };

    (lhs, rhs, work.max(1))
}

fn strcmp_benches(c: &mut Criterion) {
    let sizes = [31usize, 63, 256, 1024, 4096];
    let mut cases = Vec::new();
    for len in sizes {
        cases.push(StrcmpCase {
            label: format!("size_{len}_equal"),
            len,
            kind: StrcmpKind::Equal,
        });
        cases.push(StrcmpCase {
            label: format!("size_{len}_diff_first"),
            len,
            kind: StrcmpKind::DiffFirst,
        });
        cases.push(StrcmpCase {
            label: format!("size_{len}_diff_mid"),
            len,
            kind: StrcmpKind::DiffMid,
        });
        cases.push(StrcmpCase {
            label: format!("size_{len}_diff_last"),
            len,
            kind: StrcmpKind::DiffLast,
        });
        cases.push(StrcmpCase {
            label: format!("size_{len}_lhs_shorter"),
            len,
            kind: StrcmpKind::LhsShorter,
        });
        cases.push(StrcmpCase {
            label: format!("size_{len}_rhs_shorter"),
            len,
            kind: StrcmpKind::RhsShorter,
        });
    }

    let mut group = c.benchmark_group("strcmp");
    for case in &cases {
        let (lhs, rhs, work) = build_strcmp_inputs(case);

        configure_group_for_len(&mut group, case.len);
        group.throughput(Throughput::Bytes(work));

        group.bench_with_input(BenchmarkId::new("glibc", &case.label), &case.len, |b, _| {
            b.iter(|| unsafe {
                black_box(libc_strcmp(
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
                    black_box(fast_strcmp(black_box(&lhs), black_box(&rhs)));
                });
            },
        );
    }
    group.finish();
}

fn build_strncmp_inputs(case: &StrncmpCase) -> (Vec<u8>, Vec<u8>, usize, u64) {
    let len = case.len;
    let n_small = (len / 2).max(1);
    let n_exact = len + 1;
    let n_over = len + 32;

    let lhs = make_base(len);
    let mut rhs = make_base(len);

    match case.kind {
        StrncmpKind::EqualNSmall => (lhs, rhs, n_small, n_small as u64),
        StrncmpKind::EqualNExact => (lhs, rhs, n_exact, len as u64),
        StrncmpKind::DiffAfterNSmall => {
            rhs[len - 1] = b'z';
            (lhs, rhs, n_small, n_small as u64)
        }
        StrncmpKind::DiffBeforeNSmall => {
            rhs[0] = b'z';
            (lhs, rhs, n_small, 1)
        }
        StrncmpKind::LhsShorterNOver => {
            let mut lhs_short = make_base(len);
            let short = (len / 2).max(1);
            lhs_short[short] = 0;
            (lhs_short, rhs, n_over, (short + 1) as u64)
        }
        StrncmpKind::RhsShorterNOver => {
            let short = (len / 2).max(1);
            rhs[short] = 0;
            (lhs, rhs, n_over, (short + 1) as u64)
        }
    }
}

fn strncmp_benches(c: &mut Criterion) {
    let sizes = [31usize, 63, 256, 1024, 4096];
    let mut cases = Vec::new();
    for len in sizes {
        cases.push(StrncmpCase {
            label: format!("size_{len}_equal_n_small"),
            len,
            kind: StrncmpKind::EqualNSmall,
        });
        cases.push(StrncmpCase {
            label: format!("size_{len}_equal_n_exact"),
            len,
            kind: StrncmpKind::EqualNExact,
        });
        cases.push(StrncmpCase {
            label: format!("size_{len}_diff_after_n_small"),
            len,
            kind: StrncmpKind::DiffAfterNSmall,
        });
        cases.push(StrncmpCase {
            label: format!("size_{len}_diff_before_n_small"),
            len,
            kind: StrncmpKind::DiffBeforeNSmall,
        });
        cases.push(StrncmpCase {
            label: format!("size_{len}_lhs_shorter_n_over"),
            len,
            kind: StrncmpKind::LhsShorterNOver,
        });
        cases.push(StrncmpCase {
            label: format!("size_{len}_rhs_shorter_n_over"),
            len,
            kind: StrncmpKind::RhsShorterNOver,
        });
    }

    let mut group = c.benchmark_group("strncmp");
    for case in &cases {
        let (lhs, rhs, n, work) = build_strncmp_inputs(case);

        configure_group_for_len(&mut group, case.len);
        group.throughput(Throughput::Bytes(work));

        group.bench_with_input(
            BenchmarkId::new("glibc", &case.label),
            &(case.len, n),
            |b, &(_, n)| {
                b.iter(|| unsafe {
                    black_box(libc_strncmp(
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
                    black_box(fast_strncmp(black_box(&lhs), black_box(&rhs), black_box(n)));
                });
            },
        );
    }
    group.finish();
}

criterion_group!(benches, strcmp_benches, strncmp_benches);
criterion_main!(benches);
