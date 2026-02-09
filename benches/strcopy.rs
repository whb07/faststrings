use core::ffi::c_char;
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use faststrings::stpncpy::stpncpy as fast_stpncpy;
use faststrings::str::{
    strcat as fast_strcat, strcpy as fast_strcpy, strlcat as fast_strlcat, strlcpy as fast_strlcpy,
    strncat as fast_strncat, stpcpy as fast_stpcpy, strncpy as fast_strncpy,
};
use faststrings::strxfrm::strxfrm as fast_strxfrm;
use std::time::Duration;

unsafe extern "C" {
    #[link_name = "strcpy"]
    fn libc_strcpy(dest: *mut c_char, src: *const c_char) -> *mut c_char;
    #[link_name = "strncpy"]
    fn libc_strncpy(dest: *mut c_char, src: *const c_char, n: usize) -> *mut c_char;
    #[link_name = "stpcpy"]
    fn libc_stpcpy(dest: *mut c_char, src: *const c_char) -> *mut c_char;
    #[link_name = "stpncpy"]
    fn libc_stpncpy(dest: *mut c_char, src: *const c_char, n: usize) -> *mut c_char;
    #[link_name = "strcat"]
    fn libc_strcat(dest: *mut c_char, src: *const c_char) -> *mut c_char;
    #[link_name = "strncat"]
    fn libc_strncat(dest: *mut c_char, src: *const c_char, n: usize) -> *mut c_char;
    #[link_name = "strlcpy"]
    fn libc_strlcpy(dest: *mut c_char, src: *const c_char, size: usize) -> usize;
    #[link_name = "strlcat"]
    fn libc_strlcat(dest: *mut c_char, src: *const c_char, size: usize) -> usize;
    #[link_name = "strxfrm"]
    fn libc_strxfrm(dest: *mut c_char, src: *const c_char, n: usize) -> usize;
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

fn make_c_string(len: usize) -> Vec<u8> {
    let mut out = vec![0u8; len + 1];
    for i in 0..len {
        out[i] = b'a' + ((i * 7 + len * 11 + 3) % 23) as u8;
    }
    out[len] = 0;
    out
}

fn make_prefixed_dest(total_len: usize, prefix_len: usize) -> Vec<u8> {
    let mut dest = vec![0u8; total_len];
    for i in 0..prefix_len {
        dest[i] = b'k' + (i % 7) as u8;
    }
    dest[prefix_len] = 0;
    dest
}

fn strcpy_benches(c: &mut Criterion) {
    let sizes = [31usize, 256, 4096];
    let mut group = c.benchmark_group("strcpy");

    for len in sizes {
        let src = make_c_string(len);
        let template = vec![0xAAu8; len + 64];
        let mut dst = template.clone();
        let label = format!("size_{len}_fit");

        configure_group_for_len(&mut group, len);
        group.throughput(Throughput::Bytes((len + 1) as u64));

        group.bench_with_input(BenchmarkId::new("glibc", &label), &len, |b, _| {
            b.iter(|| unsafe {
                dst.copy_from_slice(&template);
                let ret = libc_strcpy(
                    black_box(dst.as_mut_ptr() as *mut c_char),
                    black_box(src.as_ptr() as *const c_char),
                );
                black_box((ret as usize) - (dst.as_ptr() as usize));
                black_box(unsafe { core::ptr::read_volatile(dst.as_ptr()) });
            });
        });

        group.bench_with_input(BenchmarkId::new("faststrings", &label), &len, |b, _| {
            b.iter(|| {
                dst.copy_from_slice(&template);
                black_box(fast_strcpy(black_box(&mut dst), black_box(&src)));
                black_box(unsafe { core::ptr::read_volatile(dst.as_ptr()) });
            });
        });
    }

    group.finish();
}

fn strncpy_benches(c: &mut Criterion) {
    let sizes = [31usize, 256, 4096];
    let mut group = c.benchmark_group("strncpy");

    for len in sizes {
        let cases = [
            ("truncate", len, (len / 2).max(1)),
            ("pad", (len / 2).max(1), len),
        ];
        for (mode, src_len, n) in cases {
            let src = make_c_string(src_len);
            let template = vec![0xAAu8; n + 64];
            let mut dst = template.clone();
            let label = format!("size_{len}_{mode}");

            configure_group_for_len(&mut group, len);
            group.throughput(Throughput::Bytes(n as u64));

            group.bench_with_input(BenchmarkId::new("glibc", &label), &n, |b, &n| {
                b.iter(|| unsafe {
                    dst.copy_from_slice(&template);
                    let ret = libc_strncpy(
                        black_box(dst.as_mut_ptr() as *mut c_char),
                        black_box(src.as_ptr() as *const c_char),
                        black_box(n),
                    );
                    black_box((ret as usize) - (dst.as_ptr() as usize));
                    black_box(unsafe { core::ptr::read_volatile(dst.as_ptr()) });
                });
            });

            group.bench_with_input(BenchmarkId::new("faststrings", &label), &n, |b, &n| {
                b.iter(|| {
                    dst.copy_from_slice(&template);
                    black_box(fast_strncpy(black_box(&mut dst), black_box(&src), black_box(n)));
                    black_box(unsafe { core::ptr::read_volatile(dst.as_ptr()) });
                });
            });
        }
    }

    group.finish();
}

fn stpcpy_benches(c: &mut Criterion) {
    let sizes = [31usize, 256, 4096];
    let mut group = c.benchmark_group("stpcpy");

    for len in sizes {
        let src = make_c_string(len);
        let template = vec![0xAAu8; len + 64];
        let mut dst = template.clone();
        let label = format!("size_{len}_fit");

        configure_group_for_len(&mut group, len);
        group.throughput(Throughput::Bytes((len + 1) as u64));

        group.bench_with_input(BenchmarkId::new("glibc", &label), &len, |b, _| {
            b.iter(|| unsafe {
                dst.copy_from_slice(&template);
                let end = libc_stpcpy(
                    black_box(dst.as_mut_ptr() as *mut c_char),
                    black_box(src.as_ptr() as *const c_char),
                );
                black_box((end as usize) - (dst.as_ptr() as usize));
                black_box(unsafe { core::ptr::read_volatile(dst.as_ptr()) });
            });
        });

        group.bench_with_input(BenchmarkId::new("faststrings", &label), &len, |b, _| {
            b.iter(|| {
                dst.copy_from_slice(&template);
                black_box(fast_stpcpy(black_box(&mut dst), black_box(&src)));
                black_box(unsafe { core::ptr::read_volatile(dst.as_ptr()) });
            });
        });
    }

    group.finish();
}

fn stpncpy_benches(c: &mut Criterion) {
    let sizes = [31usize, 256, 4096];
    let mut group = c.benchmark_group("stpncpy");

    for len in sizes {
        let cases = [
            ("truncate", len, (len / 2).max(1)),
            ("pad", (len / 2).max(1), len),
        ];
        for (mode, src_len, n) in cases {
            let src = make_c_string(src_len);
            let template = vec![0xAAu8; n + 64];
            let mut dst = template.clone();
            let label = format!("size_{len}_{mode}");

            configure_group_for_len(&mut group, len);
            group.throughput(Throughput::Bytes(n as u64));

            group.bench_with_input(BenchmarkId::new("glibc", &label), &n, |b, &n| {
                b.iter(|| unsafe {
                    dst.copy_from_slice(&template);
                    let end = libc_stpncpy(
                        black_box(dst.as_mut_ptr() as *mut c_char),
                        black_box(src.as_ptr() as *const c_char),
                        black_box(n),
                    );
                    black_box((end as usize) - (dst.as_ptr() as usize));
                    black_box(unsafe { core::ptr::read_volatile(dst.as_ptr()) });
                });
            });

            group.bench_with_input(BenchmarkId::new("faststrings", &label), &n, |b, &n| {
                b.iter(|| {
                    dst.copy_from_slice(&template);
                    black_box(fast_stpncpy(black_box(&mut dst), black_box(&src), black_box(n)));
                    black_box(unsafe { core::ptr::read_volatile(dst.as_ptr()) });
                });
            });
        }
    }

    group.finish();
}

fn strcat_benches(c: &mut Criterion) {
    let sizes = [31usize, 256, 4096];
    let mut group = c.benchmark_group("strcat");

    for len in sizes {
        let prefix_len = (len / 2).max(1);
        let src_len = len - prefix_len;
        let src = make_c_string(src_len);
        let template = make_prefixed_dest(len + 64, prefix_len);
        let mut dst = template.clone();
        let label = format!("size_{len}_append");

        configure_group_for_len(&mut group, len);
        group.throughput(Throughput::Bytes(src_len as u64));

        group.bench_with_input(BenchmarkId::new("glibc", &label), &len, |b, _| {
            b.iter(|| unsafe {
                dst.copy_from_slice(&template);
                let ret = libc_strcat(
                    black_box(dst.as_mut_ptr() as *mut c_char),
                    black_box(src.as_ptr() as *const c_char),
                );
                black_box((ret as usize) - (dst.as_ptr() as usize));
                black_box(unsafe { core::ptr::read_volatile(dst.as_ptr()) });
            });
        });

        group.bench_with_input(BenchmarkId::new("faststrings", &label), &len, |b, _| {
            b.iter(|| {
                dst.copy_from_slice(&template);
                black_box(fast_strcat(black_box(&mut dst), black_box(&src)));
                black_box(unsafe { core::ptr::read_volatile(dst.as_ptr()) });
            });
        });
    }

    group.finish();
}

fn strncat_benches(c: &mut Criterion) {
    let sizes = [31usize, 256, 4096];
    let mut group = c.benchmark_group("strncat");

    for len in sizes {
        let prefix_len = (len / 2).max(1);
        let src_len = len - prefix_len;
        let src = make_c_string(src_len);
        let template = make_prefixed_dest(len + 64, prefix_len);
        let mut dst = template.clone();
        let cases = [("n_small", (src_len / 2).max(1)), ("n_full", src_len + 16)];

        for (mode, n) in cases {
            let label = format!("size_{len}_{mode}");
            let work = src_len.min(n) as u64;
            configure_group_for_len(&mut group, len);
            group.throughput(Throughput::Bytes(work.max(1)));

            group.bench_with_input(BenchmarkId::new("glibc", &label), &n, |b, &n| {
                b.iter(|| unsafe {
                    dst.copy_from_slice(&template);
                    let ret = libc_strncat(
                        black_box(dst.as_mut_ptr() as *mut c_char),
                        black_box(src.as_ptr() as *const c_char),
                        black_box(n),
                    );
                    black_box((ret as usize) - (dst.as_ptr() as usize));
                    black_box(unsafe { core::ptr::read_volatile(dst.as_ptr()) });
                });
            });

            group.bench_with_input(BenchmarkId::new("faststrings", &label), &n, |b, &n| {
                b.iter(|| {
                    dst.copy_from_slice(&template);
                    black_box(fast_strncat(black_box(&mut dst), black_box(&src), black_box(n)));
                    black_box(unsafe { core::ptr::read_volatile(dst.as_ptr()) });
                });
            });
        }
    }

    group.finish();
}

fn strlcpy_benches(c: &mut Criterion) {
    let sizes = [31usize, 256, 4096];
    let mut group = c.benchmark_group("strlcpy");

    for len in sizes {
        let src = make_c_string(len);
        let cases = [("fit", len + 64), ("truncate", (len / 2).max(2))];
        for (mode, dst_len) in cases {
            let template = vec![0xAAu8; dst_len];
            let mut dst = template.clone();
            let label = format!("size_{len}_{mode}");
            let work = len.min(dst_len.saturating_sub(1)).max(1) as u64;

            configure_group_for_len(&mut group, len);
            group.throughput(Throughput::Bytes(work));

            group.bench_with_input(BenchmarkId::new("glibc", &label), &dst_len, |b, &n| {
                b.iter(|| unsafe {
                    dst.copy_from_slice(&template);
                    black_box(libc_strlcpy(
                        black_box(dst.as_mut_ptr() as *mut c_char),
                        black_box(src.as_ptr() as *const c_char),
                        black_box(n),
                    ));
                    black_box(unsafe { core::ptr::read_volatile(dst.as_ptr()) });
                });
            });

            group.bench_with_input(BenchmarkId::new("faststrings", &label), &dst_len, |b, _| {
                b.iter(|| {
                    dst.copy_from_slice(&template);
                    black_box(fast_strlcpy(black_box(&mut dst), black_box(&src)));
                    black_box(unsafe { core::ptr::read_volatile(dst.as_ptr()) });
                });
            });
        }
    }

    group.finish();
}

fn strlcat_benches(c: &mut Criterion) {
    let sizes = [31usize, 256, 4096];
    let mut group = c.benchmark_group("strlcat");

    for len in sizes {
        let prefix_len = (len / 2).max(1);
        let src = make_c_string(len);
        let cases = [
            ("fit", prefix_len + len + 64),
            ("truncate", prefix_len + (len / 2).max(1) + 1),
        ];
        for (mode, dst_len) in cases {
            let template = make_prefixed_dest(dst_len, prefix_len.min(dst_len.saturating_sub(1)));
            let mut dst = template.clone();
            let label = format!("size_{len}_{mode}");

            configure_group_for_len(&mut group, len);
            group.throughput(Throughput::Bytes(len as u64));

            group.bench_with_input(BenchmarkId::new("glibc", &label), &dst_len, |b, &n| {
                b.iter(|| unsafe {
                    dst.copy_from_slice(&template);
                    black_box(libc_strlcat(
                        black_box(dst.as_mut_ptr() as *mut c_char),
                        black_box(src.as_ptr() as *const c_char),
                        black_box(n),
                    ));
                    black_box(unsafe { core::ptr::read_volatile(dst.as_ptr()) });
                });
            });

            group.bench_with_input(BenchmarkId::new("faststrings", &label), &dst_len, |b, _| {
                b.iter(|| {
                    dst.copy_from_slice(&template);
                    black_box(fast_strlcat(black_box(&mut dst), black_box(&src)));
                    black_box(unsafe { core::ptr::read_volatile(dst.as_ptr()) });
                });
            });
        }
    }

    group.finish();
}

fn strxfrm_benches(c: &mut Criterion) {
    let sizes = [31usize, 256, 4096];
    let mut group = c.benchmark_group("strxfrm");

    for len in sizes {
        let src = make_c_string(len);
        let cases = [("fit", len + 64), ("truncate", (len / 2).max(2))];
        for (mode, dst_len) in cases {
            let template = vec![0xAAu8; dst_len];
            let mut dst = template.clone();
            let label = format!("size_{len}_{mode}");
            let work = len.min(dst_len.saturating_sub(1)).max(1) as u64;

            configure_group_for_len(&mut group, len);
            group.throughput(Throughput::Bytes(work));

            group.bench_with_input(BenchmarkId::new("glibc", &label), &dst_len, |b, &n| {
                b.iter(|| unsafe {
                    dst.copy_from_slice(&template);
                    black_box(libc_strxfrm(
                        black_box(dst.as_mut_ptr() as *mut c_char),
                        black_box(src.as_ptr() as *const c_char),
                        black_box(n),
                    ));
                    black_box(unsafe { core::ptr::read_volatile(dst.as_ptr()) });
                });
            });

            group.bench_with_input(BenchmarkId::new("faststrings", &label), &dst_len, |b, _| {
                b.iter(|| {
                    dst.copy_from_slice(&template);
                    black_box(fast_strxfrm(black_box(&mut dst), black_box(&src)));
                    black_box(unsafe { core::ptr::read_volatile(dst.as_ptr()) });
                });
            });
        }
    }

    group.finish();
}

criterion_group!(
    benches,
    strcpy_benches,
    strncpy_benches,
    stpcpy_benches,
    stpncpy_benches,
    strcat_benches,
    strncat_benches,
    strlcpy_benches,
    strlcat_benches,
    strxfrm_benches
);
criterion_main!(benches);
