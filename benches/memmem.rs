use core::ffi::c_void;
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use faststrings::mem::memmem as fast_memmem;
use std::time::Duration;

unsafe extern "C" {
    #[link_name = "memmem"]
    fn libc_memmem(
        haystack: *const c_void,
        haystack_len: usize,
        needle: *const c_void,
        needle_len: usize,
    ) -> *mut c_void;
}

#[derive(Copy, Clone)]
enum Placement {
    Head,
    Middle,
    Tail,
    Miss,
}

#[derive(Clone)]
struct SearchCase {
    label: String,
    hay_len: usize,
    needle_len: usize,
    placement: Placement,
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

fn memmem_benches(c: &mut Criterion) {
    let mut cases = Vec::new();
    let hay_sizes = [256usize, 4096, 65536];
    let needle_sizes = [1usize, 4, 16, 31];

    for hay_len in hay_sizes {
        for needle_len in needle_sizes {
            if needle_len > hay_len {
                continue;
            }
            cases.push(SearchCase {
                label: format!("h{hay_len}_n{needle_len}_head"),
                hay_len,
                needle_len,
                placement: Placement::Head,
            });
            cases.push(SearchCase {
                label: format!("h{hay_len}_n{needle_len}_mid"),
                hay_len,
                needle_len,
                placement: Placement::Middle,
            });
            cases.push(SearchCase {
                label: format!("h{hay_len}_n{needle_len}_tail"),
                hay_len,
                needle_len,
                placement: Placement::Tail,
            });
            cases.push(SearchCase {
                label: format!("h{hay_len}_n{needle_len}_miss"),
                hay_len,
                needle_len,
                placement: Placement::Miss,
            });
        }
    }

    let mut group = c.benchmark_group("memmem");
    for case in &cases {
        let hay_len = case.hay_len;
        let needle_len = case.needle_len;

        let mut haystack_buf = vec![0u8; hay_len + 64];
        for i in 0..hay_len {
            haystack_buf[32 + i] = ((i * 37 + 11) % 113) as u8;
        }
        let haystack = &mut haystack_buf[32..32 + hay_len];

        let mut needle = vec![0u8; needle_len];
        for (i, byte) in needle.iter_mut().enumerate() {
            *byte = 200u8.wrapping_add(i as u8);
        }

        let pos = match case.placement {
            Placement::Head => Some(0usize),
            Placement::Middle => Some((hay_len - needle_len) / 2),
            Placement::Tail => Some(hay_len - needle_len),
            Placement::Miss => None,
        };
        if let Some(p) = pos {
            haystack[p..p + needle_len].copy_from_slice(&needle);
        }

        configure_group_for_len(&mut group, hay_len);
        group.throughput(Throughput::Bytes(hay_len as u64));

        group.bench_with_input(
            BenchmarkId::new("glibc", &case.label),
            &(hay_len, needle_len),
            |b, &(h, n)| {
                b.iter(|| unsafe {
                    let base = haystack.as_ptr() as usize;
                    let ptr = libc_memmem(
                        black_box(haystack.as_ptr() as *const c_void),
                        black_box(h),
                        black_box(needle.as_ptr() as *const c_void),
                        black_box(n),
                    );
                    let rv = if ptr.is_null() {
                        usize::MAX
                    } else {
                        (ptr as usize) - base
                    };
                    black_box(rv);
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("faststrings", &case.label),
            &(hay_len, needle_len),
            |b, &(_, _)| {
                b.iter(|| {
                    let rv =
                        fast_memmem(black_box(haystack), black_box(&needle)).unwrap_or(usize::MAX);
                    black_box(rv);
                });
            },
        );
    }

    group.finish();
}

criterion_group!(benches, memmem_benches);
criterion_main!(benches);
