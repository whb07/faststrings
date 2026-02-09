use core::ffi::c_char;
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use faststrings::search::{
    strchr as fast_strchr, strchrnul as fast_strchrnul, strrchr as fast_strrchr,
};
use std::time::Duration;

unsafe extern "C" {
    #[link_name = "strchr"]
    fn libc_strchr(s: *const c_char, c: i32) -> *mut c_char;
    #[link_name = "strchrnul"]
    fn libc_strchrnul(s: *const c_char, c: i32) -> *mut c_char;
    #[link_name = "strrchr"]
    fn libc_strrchr(s: *const c_char, c: i32) -> *mut c_char;
}

#[derive(Copy, Clone)]
enum CaseKind {
    HitHead,
    HitMiddle,
    HitTail,
    Miss,
    FindNul,
}

#[derive(Clone)]
struct SearchCase {
    label: String,
    len: usize,
    kind: CaseKind,
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

fn make_cases() -> Vec<SearchCase> {
    let sizes = [31usize, 63, 256, 1024, 4096, 65536];
    let mut cases = Vec::new();
    for len in sizes {
        cases.push(SearchCase {
            label: format!("size_{len}_hit_head"),
            len,
            kind: CaseKind::HitHead,
        });
        cases.push(SearchCase {
            label: format!("size_{len}_hit_mid"),
            len,
            kind: CaseKind::HitMiddle,
        });
        cases.push(SearchCase {
            label: format!("size_{len}_hit_tail"),
            len,
            kind: CaseKind::HitTail,
        });
        cases.push(SearchCase {
            label: format!("size_{len}_miss"),
            len,
            kind: CaseKind::Miss,
        });
        cases.push(SearchCase {
            label: format!("size_{len}_find_nul"),
            len,
            kind: CaseKind::FindNul,
        });
    }
    cases
}

fn prepare_input(case: &SearchCase) -> (Vec<u8>, u8, u64) {
    const TARGET: u8 = 233;
    let len = case.len;
    let mut buf = vec![0u8; len + 1 + 64];
    for i in 0..len {
        buf[i] = ((i * 13 + len * 7 + 1) % 199) as u8 + 1;
    }
    buf[len] = 0;

    let (needle, work) = match case.kind {
        CaseKind::HitHead => {
            buf[0] = TARGET;
            (TARGET, 1)
        }
        CaseKind::HitMiddle => {
            let p = len / 2;
            buf[p] = TARGET;
            (TARGET, (p + 1) as u64)
        }
        CaseKind::HitTail => {
            let p = len.saturating_sub(1);
            buf[p] = TARGET;
            (TARGET, len as u64)
        }
        CaseKind::Miss => (TARGET, len as u64),
        CaseKind::FindNul => (0, (len + 1) as u64),
    };

    (buf, needle, work.max(1))
}

fn strchr_benches(c: &mut Criterion) {
    let cases = make_cases();
    let mut group = c.benchmark_group("strchr");
    for case in &cases {
        let (buf, needle, work) = prepare_input(case);
        let s = &buf[..case.len + 1];

        configure_group_for_len(&mut group, case.len);
        group.throughput(Throughput::Bytes(work));

        group.bench_with_input(BenchmarkId::new("glibc", &case.label), &case.len, |b, _| {
            b.iter(|| unsafe {
                let base = s.as_ptr() as usize;
                let ptr = libc_strchr(
                    black_box(s.as_ptr() as *const c_char),
                    black_box(needle as i32),
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
            BenchmarkId::new("faststrings", &case.label),
            &case.len,
            |b, _| {
                b.iter(|| {
                    black_box(fast_strchr(black_box(s), black_box(needle)).unwrap_or(usize::MAX));
                });
            },
        );
    }
    group.finish();
}

fn strchrnul_benches(c: &mut Criterion) {
    let cases = make_cases();
    let mut group = c.benchmark_group("strchrnul");
    for case in &cases {
        let (buf, needle, work) = prepare_input(case);
        let s = &buf[..case.len + 1];

        configure_group_for_len(&mut group, case.len);
        group.throughput(Throughput::Bytes(work));

        group.bench_with_input(BenchmarkId::new("glibc", &case.label), &case.len, |b, _| {
            b.iter(|| unsafe {
                let base = s.as_ptr() as usize;
                let ptr = libc_strchrnul(
                    black_box(s.as_ptr() as *const c_char),
                    black_box(needle as i32),
                );
                black_box((ptr as usize) - base);
            });
        });

        group.bench_with_input(
            BenchmarkId::new("faststrings", &case.label),
            &case.len,
            |b, _| {
                b.iter(|| {
                    black_box(fast_strchrnul(black_box(s), black_box(needle)));
                });
            },
        );
    }
    group.finish();
}

fn strrchr_benches(c: &mut Criterion) {
    let cases = make_cases();
    let mut group = c.benchmark_group("strrchr");
    for case in &cases {
        let (buf, needle, work) = prepare_input(case);
        let s = &buf[..case.len + 1];

        configure_group_for_len(&mut group, case.len);
        group.throughput(Throughput::Bytes(work));

        group.bench_with_input(BenchmarkId::new("glibc", &case.label), &case.len, |b, _| {
            b.iter(|| unsafe {
                let base = s.as_ptr() as usize;
                let ptr = libc_strrchr(
                    black_box(s.as_ptr() as *const c_char),
                    black_box(needle as i32),
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
            BenchmarkId::new("faststrings", &case.label),
            &case.len,
            |b, _| {
                b.iter(|| {
                    black_box(fast_strrchr(black_box(s), black_box(needle)).unwrap_or(usize::MAX));
                });
            },
        );
    }
    group.finish();
}

criterion_group!(benches, strchr_benches, strchrnul_benches, strrchr_benches);
criterion_main!(benches);
