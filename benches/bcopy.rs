use core::ffi::c_void;
use criterion::{BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main};
use faststrings::bcopy::bcopy as fast_bcopy;
use std::time::Duration;

unsafe extern "C" {
    #[link_name = "bcopy"]
    fn libc_bcopy(src: *const c_void, dest: *mut c_void, n: usize);
}

#[derive(Copy, Clone)]
enum CaseKind {
    NonOverlap,
    OverlapForward,
    OverlapBackward,
}

#[derive(Clone)]
struct CopyCase {
    label: String,
    len: usize,
    kind: CaseKind,
    delta: usize,
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

fn bcopy_benches(c: &mut Criterion) {
    let mut cases = Vec::new();
    let sizes = [
        1usize, 8, 31, 32, 63, 64, 65, 128, 129, 256, 257, 1024, 4096, 65536,
    ];

    for len in sizes {
        cases.push(CopyCase {
            label: format!("nonoverlap_size_{len}"),
            len,
            kind: CaseKind::NonOverlap,
            delta: 17,
        });

        for delta in [1usize, 31] {
            cases.push(CopyCase {
                label: format!("overlap_fwd_size_{len}_d{delta}"),
                len,
                kind: CaseKind::OverlapForward,
                delta,
            });
            cases.push(CopyCase {
                label: format!("overlap_bwd_size_{len}_d{delta}"),
                len,
                kind: CaseKind::OverlapBackward,
                delta,
            });
        }
    }

    let mut group = c.benchmark_group("bcopy");

    for case in &cases {
        let len = case.len;
        let alloc_len = (len * 3) + 128;
        let mut buf = vec![0u8; alloc_len];
        for (i, byte) in buf.iter_mut().enumerate() {
            *byte = (i % 251) as u8;
        }

        let base = unsafe { buf.as_mut_ptr().add(32) };
        let (src_ptr, dst_ptr) = match case.kind {
            CaseKind::NonOverlap => unsafe { (base, base.add(len + case.delta)) },
            CaseKind::OverlapForward => unsafe { (base.add(case.delta), base) },
            CaseKind::OverlapBackward => unsafe { (base, base.add(case.delta)) },
        };

        configure_group_for_len(&mut group, len);
        group.throughput(Throughput::Bytes(len as u64));

        group.bench_with_input(BenchmarkId::new("glibc", &case.label), &len, |b, &n| {
            b.iter(|| unsafe {
                libc_bcopy(
                    black_box(src_ptr as *const c_void),
                    black_box(dst_ptr as *mut c_void),
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
                    let src = core::slice::from_raw_parts(black_box(src_ptr), n);
                    let dst = core::slice::from_raw_parts_mut(black_box(dst_ptr), n);
                    fast_bcopy(src, dst);
                    black_box(core::ptr::read_volatile(dst_ptr));
                });
            },
        );
    }

    group.finish();
}

criterion_group!(benches, bcopy_benches);
criterion_main!(benches);
