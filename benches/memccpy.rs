use core::ffi::c_void;
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use faststrings::mem::memccpy as fast_memccpy;
use std::time::Duration;

unsafe extern "C" {
    #[link_name = "memccpy"]
    fn libc_memccpy(dest: *mut c_void, src: *const c_void, c: i32, n: usize) -> *mut c_void;
}

#[derive(Copy, Clone)]
enum StopKind {
    First,
    Middle,
    Last,
    Miss,
}

#[derive(Clone)]
struct CopyCase {
    label: String,
    len: usize,
    kind: StopKind,
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

fn memccpy_benches(c: &mut Criterion) {
    const MARK: u8 = 0;
    let sizes = [31usize, 63, 256, 257, 1024, 4096, 65536];
    let mut cases = Vec::new();
    for len in sizes {
        cases.push(CopyCase {
            label: format!("size_{len}_stop_first"),
            len,
            kind: StopKind::First,
        });
        cases.push(CopyCase {
            label: format!("size_{len}_stop_mid"),
            len,
            kind: StopKind::Middle,
        });
        cases.push(CopyCase {
            label: format!("size_{len}_stop_last"),
            len,
            kind: StopKind::Last,
        });
        cases.push(CopyCase {
            label: format!("size_{len}_stop_miss"),
            len,
            kind: StopKind::Miss,
        });
    }

    let mut group = c.benchmark_group("memccpy");
    for case in &cases {
        let len = case.len;
        let mut src_buf = vec![0u8; len + 64];
        let mut dst_buf = vec![0u8; len + 64];
        for (i, byte) in src_buf.iter_mut().enumerate() {
            let mut value = ((i * 17 + len * 5 + 3) % 251) as u8;
            if value == MARK {
                value = 1;
            }
            *byte = value;
        }
        for (i, byte) in dst_buf.iter_mut().enumerate() {
            *byte = (i.wrapping_mul(29) % 251) as u8;
        }

        let src = &mut src_buf[32..32 + len];
        let dst = &mut dst_buf[32..32 + len];
        match case.kind {
            StopKind::First => src[0] = MARK,
            StopKind::Middle => src[len / 2] = MARK,
            StopKind::Last => src[len - 1] = MARK,
            StopKind::Miss => {}
        }

        configure_group_for_len(&mut group, len);
        group.throughput(Throughput::Bytes(len as u64));

        group.bench_with_input(BenchmarkId::new("glibc", &case.label), &len, |b, &n| {
            b.iter(|| unsafe {
                let ptr = libc_memccpy(
                    black_box(dst.as_mut_ptr() as *mut c_void),
                    black_box(src.as_ptr() as *const c_void),
                    black_box(MARK as i32),
                    black_box(n),
                );
                let rv = if ptr.is_null() {
                    usize::MAX
                } else {
                    (ptr as usize) - (dst.as_ptr() as usize)
                };
                black_box(rv);
                black_box(core::ptr::read_volatile(dst.as_ptr()));
            });
        });

        group.bench_with_input(
            BenchmarkId::new("faststrings", &case.label),
            &len,
            |b, &n| {
                b.iter(|| {
                    let rv = fast_memccpy(black_box(&mut dst[..n]), black_box(&src[..n]), MARK)
                        .unwrap_or(usize::MAX);
                    black_box(rv);
                    black_box(dst[0]);
                });
            },
        );
    }
    group.finish();
}

criterion_group!(benches, memccpy_benches);
criterion_main!(benches);
