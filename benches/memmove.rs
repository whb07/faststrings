use core::ffi::c_void;
use criterion::{BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main};
use faststrings::memmove::optimized_memmove_unified;
use std::time::Duration;

unsafe extern "C" {
    #[link_name = "memmove"]
    fn libc_memmove(dest: *mut c_void, src: *const c_void, n: usize) -> *mut c_void;
}

#[derive(Copy, Clone)]
enum CaseKind {
    NonOverlap,
    OverlapForward,
    OverlapBackward,
}

#[derive(Clone)]
struct MoveCase {
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

fn memmove_benches(c: &mut Criterion) {
    let mut cases = Vec::new();

    let sizes = [
        1usize, 2, 3, 4, 7, 8, 15, 16, 31, 32, 63, 64, 65, 127, 128, 129, 255, 256, 257, 511, 512,
        513, 1023, 1024, 4096, 65536, 262144,
    ];

    for len in sizes {
        cases.push(MoveCase {
            label: format!("nonoverlap_size_{len}"),
            len,
            kind: CaseKind::NonOverlap,
            delta: 17,
        });

        for delta in [1usize, 15, 31] {
            cases.push(MoveCase {
                label: format!("overlap_fwd_size_{len}_d{delta}"),
                len,
                kind: CaseKind::OverlapForward,
                delta,
            });
            cases.push(MoveCase {
                label: format!("overlap_bwd_size_{len}_d{delta}"),
                len,
                kind: CaseKind::OverlapBackward,
                delta,
            });
        }
    }

    let mut group = c.benchmark_group("memmove");

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
                libc_memmove(
                    black_box(dst_ptr as *mut c_void),
                    black_box(src_ptr as *const c_void),
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
                    optimized_memmove_unified(black_box(dst_ptr), black_box(src_ptr), black_box(n));
                    black_box(core::ptr::read_volatile(dst_ptr));
                });
            },
        );
    }

    group.finish();
}

criterion_group!(benches, memmove_benches);
criterion_main!(benches);
