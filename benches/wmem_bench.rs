use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use faststrings::types::wchar_t;
use faststrings::wmem::{
    wmemchr as fast_wmemchr, wmemcmp as fast_wmemcmp, wmemcpy as fast_wmemcpy,
    wmemmove as fast_wmemmove, wmempcpy as fast_wmempcpy, wmemrchr as fast_wmemrchr,
    wmemset as fast_wmemset,
};
use std::time::Duration;

unsafe extern "C" {
    #[link_name = "wmemcpy"]
    fn libc_wmemcpy(dest: *mut wchar_t, src: *const wchar_t, n: usize) -> *mut wchar_t;
    #[link_name = "wmempcpy"]
    fn libc_wmempcpy(dest: *mut wchar_t, src: *const wchar_t, n: usize) -> *mut wchar_t;
    #[link_name = "wmemmove"]
    fn libc_wmemmove(dest: *mut wchar_t, src: *const wchar_t, n: usize) -> *mut wchar_t;
    #[link_name = "wmemset"]
    fn libc_wmemset(dest: *mut wchar_t, c: wchar_t, n: usize) -> *mut wchar_t;
    #[link_name = "wmemcmp"]
    fn libc_wmemcmp(s1: *const wchar_t, s2: *const wchar_t, n: usize) -> i32;
    #[link_name = "wmemchr"]
    fn libc_wmemchr(s: *const wchar_t, c: wchar_t, n: usize) -> *mut wchar_t;
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

fn make_wide_buf(len: usize) -> Vec<wchar_t> {
    let mut out = vec![0 as wchar_t; len];
    for (i, v) in out.iter_mut().enumerate() {
        *v = (i as wchar_t % 251) + 1;
    }
    out
}

fn scalar_wmemrchr(s: &[wchar_t], c: wchar_t) -> Option<usize> {
    for i in (0..s.len()).rev() {
        if s[i] == c {
            return Some(i);
        }
    }
    None
}

fn wmemcpy_benches(c: &mut Criterion) {
    let sizes = [31usize, 256, 4096];
    let mut group = c.benchmark_group("wmemcpy");

    for len in sizes {
        let src = make_wide_buf(len);
        let template = vec![0x55 as wchar_t; len];
        let mut dst = template.clone();
        let bytes = (len * core::mem::size_of::<wchar_t>()) as u64;
        let label = format!("size_{len}");

        configure_group_for_len(&mut group, len);
        group.throughput(Throughput::Bytes(bytes));

        group.bench_with_input(BenchmarkId::new("glibc", &label), &len, |b, &n| {
            b.iter(|| unsafe {
                dst.copy_from_slice(&template);
                let ret = libc_wmemcpy(
                    black_box(dst.as_mut_ptr()),
                    black_box(src.as_ptr()),
                    black_box(n),
                );
                black_box((ret as usize) - (dst.as_ptr() as usize));
                black_box(core::ptr::read_volatile(dst.as_ptr()));
            });
        });

        group.bench_with_input(BenchmarkId::new("faststrings", &label), &len, |b, _| {
            b.iter(|| {
                dst.copy_from_slice(&template);
                black_box(fast_wmemcpy(black_box(&mut dst), black_box(&src)));
                black_box(unsafe { core::ptr::read_volatile(dst.as_ptr()) });
            });
        });
    }
    group.finish();
}

fn wmempcpy_benches(c: &mut Criterion) {
    let sizes = [31usize, 256, 4096];
    let mut group = c.benchmark_group("wmempcpy");

    for len in sizes {
        let src = make_wide_buf(len);
        let template = vec![0x55 as wchar_t; len];
        let mut dst = template.clone();
        let bytes = (len * core::mem::size_of::<wchar_t>()) as u64;
        let label = format!("size_{len}");

        configure_group_for_len(&mut group, len);
        group.throughput(Throughput::Bytes(bytes));

        group.bench_with_input(BenchmarkId::new("glibc", &label), &len, |b, &n| {
            b.iter(|| unsafe {
                dst.copy_from_slice(&template);
                let ret = libc_wmempcpy(
                    black_box(dst.as_mut_ptr()),
                    black_box(src.as_ptr()),
                    black_box(n),
                );
                black_box((ret as usize) - (dst.as_ptr() as usize));
                black_box(core::ptr::read_volatile(dst.as_ptr()));
            });
        });

        group.bench_with_input(BenchmarkId::new("faststrings", &label), &len, |b, _| {
            b.iter(|| {
                dst.copy_from_slice(&template);
                black_box(fast_wmempcpy(black_box(&mut dst), black_box(&src)));
                black_box(unsafe { core::ptr::read_volatile(dst.as_ptr()) });
            });
        });
    }
    group.finish();
}

fn wmemmove_benches(c: &mut Criterion) {
    let sizes = [31usize, 256, 4096];
    let mut group = c.benchmark_group("wmemmove");

    for len in sizes {
        let src = make_wide_buf(len);
        let template = vec![0x33 as wchar_t; len];
        let mut dst = template.clone();
        let bytes = (len * core::mem::size_of::<wchar_t>()) as u64;
        let label = format!("size_{len}_nonoverlap");

        configure_group_for_len(&mut group, len);
        group.throughput(Throughput::Bytes(bytes));

        group.bench_with_input(BenchmarkId::new("glibc", &label), &len, |b, &n| {
            b.iter(|| unsafe {
                dst.copy_from_slice(&template);
                let ret = libc_wmemmove(
                    black_box(dst.as_mut_ptr()),
                    black_box(src.as_ptr()),
                    black_box(n),
                );
                black_box((ret as usize) - (dst.as_ptr() as usize));
                black_box(core::ptr::read_volatile(dst.as_ptr()));
            });
        });

        group.bench_with_input(BenchmarkId::new("faststrings", &label), &len, |b, _| {
            b.iter(|| {
                dst.copy_from_slice(&template);
                black_box(fast_wmemmove(black_box(&mut dst), black_box(&src)));
                black_box(unsafe { core::ptr::read_volatile(dst.as_ptr()) });
            });
        });
    }
    group.finish();
}

fn wmemset_benches(c: &mut Criterion) {
    let sizes = [31usize, 256, 4096];
    let mut group = c.benchmark_group("wmemset");

    for len in sizes {
        let fill = 0x1234 as wchar_t;
        let template = vec![0x11 as wchar_t; len];
        let mut dst = template.clone();
        let bytes = (len * core::mem::size_of::<wchar_t>()) as u64;
        let label = format!("size_{len}");

        configure_group_for_len(&mut group, len);
        group.throughput(Throughput::Bytes(bytes));

        group.bench_with_input(BenchmarkId::new("glibc", &label), &len, |b, &n| {
            b.iter(|| unsafe {
                dst.copy_from_slice(&template);
                let ret = libc_wmemset(black_box(dst.as_mut_ptr()), black_box(fill), black_box(n));
                black_box((ret as usize) - (dst.as_ptr() as usize));
                black_box(core::ptr::read_volatile(dst.as_ptr()));
            });
        });

        group.bench_with_input(BenchmarkId::new("faststrings", &label), &len, |b, _| {
            b.iter(|| {
                dst.copy_from_slice(&template);
                black_box(fast_wmemset(black_box(&mut dst), black_box(fill)));
                black_box(unsafe { core::ptr::read_volatile(dst.as_ptr()) });
            });
        });
    }
    group.finish();
}

fn wmemcmp_benches(c: &mut Criterion) {
    let sizes = [31usize, 256, 4096];
    let mut group = c.benchmark_group("wmemcmp");

    for len in sizes {
        let lhs = make_wide_buf(len);
        let rhs = make_wide_buf(len);
        let bytes = (len * core::mem::size_of::<wchar_t>()) as u64;
        let label = format!("size_{len}_equal");

        configure_group_for_len(&mut group, len);
        group.throughput(Throughput::Bytes(bytes));

        group.bench_with_input(BenchmarkId::new("glibc", &label), &len, |b, &n| {
            b.iter(|| unsafe {
                black_box(libc_wmemcmp(
                    black_box(lhs.as_ptr()),
                    black_box(rhs.as_ptr()),
                    black_box(n),
                ));
            });
        });

        group.bench_with_input(BenchmarkId::new("faststrings", &label), &len, |b, _| {
            b.iter(|| {
                black_box(fast_wmemcmp(black_box(&lhs), black_box(&rhs)));
            });
        });
    }
    group.finish();
}

fn wmemchr_benches(c: &mut Criterion) {
    let sizes = [31usize, 256, 4096];
    let mut group = c.benchmark_group("wmemchr");

    for len in sizes {
        let mut hay = make_wide_buf(len);
        let target = 0x7A7A as wchar_t;
        let pos = len / 2;
        hay[pos] = target;
        let bytes = ((pos + 1) * core::mem::size_of::<wchar_t>()) as u64;
        let label = format!("size_{len}_hit_mid");

        configure_group_for_len(&mut group, len);
        group.throughput(Throughput::Bytes(bytes.max(1)));

        group.bench_with_input(BenchmarkId::new("glibc", &label), &len, |b, &n| {
            b.iter(|| unsafe {
                let base = hay.as_ptr() as usize;
                let ptr = libc_wmemchr(black_box(hay.as_ptr()), black_box(target), black_box(n));
                let rv = if ptr.is_null() {
                    usize::MAX
                } else {
                    (ptr as usize) - base
                };
                black_box(rv);
            });
        });

        group.bench_with_input(BenchmarkId::new("faststrings", &label), &len, |b, _| {
            b.iter(|| {
                black_box(fast_wmemchr(black_box(&hay), black_box(target)).unwrap_or(usize::MAX));
            });
        });
    }
    group.finish();
}

fn wmemrchr_benches(c: &mut Criterion) {
    let sizes = [31usize, 256, 4096];
    let mut group = c.benchmark_group("wmemrchr");

    for len in sizes {
        let mut hay = make_wide_buf(len);
        let target = 0x6B6B as wchar_t;
        let pos = len - 1;
        hay[pos] = target;
        let bytes = (len * core::mem::size_of::<wchar_t>()) as u64;
        let label = format!("size_{len}_hit_tail");

        configure_group_for_len(&mut group, len);
        group.throughput(Throughput::Bytes(bytes));

        group.bench_with_input(BenchmarkId::new("scalar", &label), &len, |b, _| {
            b.iter(|| {
                black_box(scalar_wmemrchr(black_box(&hay), black_box(target)).unwrap_or(usize::MAX));
            });
        });

        group.bench_with_input(BenchmarkId::new("faststrings", &label), &len, |b, _| {
            b.iter(|| {
                black_box(fast_wmemrchr(black_box(&hay), black_box(target)).unwrap_or(usize::MAX));
            });
        });
    }
    group.finish();
}

criterion_group!(
    benches,
    wmemcpy_benches,
    wmempcpy_benches,
    wmemmove_benches,
    wmemset_benches,
    wmemcmp_benches,
    wmemchr_benches,
    wmemrchr_benches
);
criterion_main!(benches);
