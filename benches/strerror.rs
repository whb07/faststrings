use core::ffi::{c_char, c_int};
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use faststrings::strerror::strerror as fast_strerror;
use faststrings::strerror_r::strerror_r as fast_strerror_r;
use std::time::Duration;

unsafe extern "C" {
    #[link_name = "strerror"]
    fn libc_strerror(errnum: c_int) -> *mut c_char;
}

#[cfg(all(target_os = "linux", target_env = "gnu"))]
unsafe extern "C" {
    #[link_name = "__xpg_strerror_r"]
    fn libc_posix_strerror_r(errnum: c_int, buf: *mut c_char, buflen: usize) -> c_int;
}

#[cfg(not(all(target_os = "linux", target_env = "gnu")))]
unsafe extern "C" {
    #[link_name = "strerror_r"]
    fn libc_posix_strerror_r(errnum: c_int, buf: *mut c_char, buflen: usize) -> c_int;
}

fn configure_group(
    group: &mut criterion::BenchmarkGroup<'_, criterion::measurement::WallTime>,
    size_hint: usize,
) {
    if size_hint >= 256 {
        group.sample_size(50);
        group.warm_up_time(Duration::from_millis(200));
        group.measurement_time(Duration::from_millis(600));
    } else {
        group.sample_size(60);
        group.warm_up_time(Duration::from_millis(150));
        group.measurement_time(Duration::from_millis(450));
    }
}

fn strerror_benches(c: &mut Criterion) {
    let cases = [
        ("success", 0),
        ("enoent", 2),
        ("einval", 22),
        ("eacces", 13),
        ("etimedout", 110),
        ("unknown", 9_999),
    ];

    let mut group = c.benchmark_group("strerror");
    configure_group(&mut group, 64);

    for (label, errnum) in cases {
        group.bench_with_input(BenchmarkId::new("glibc", label), &errnum, |b, &err| {
            b.iter(|| unsafe {
                let ptr = libc_strerror(black_box(err));
                black_box(ptr);
                if !ptr.is_null() {
                    black_box(core::ptr::read_volatile(ptr as *const u8));
                }
            });
        });

        group.bench_with_input(BenchmarkId::new("faststrings", label), &errnum, |b, &err| {
            b.iter(|| {
                let msg = fast_strerror(black_box(err));
                black_box(msg.len());
                black_box(unsafe { core::ptr::read_volatile(msg.as_ptr()) });
            });
        });
    }

    group.finish();
}

fn strerror_r_benches(c: &mut Criterion) {
    let cases = [
        ("known_fit", 22, 64usize),
        ("known_tight", 2, 8usize),
        ("unknown_fit", 9_999, 64usize),
        ("unknown_tight", 9_999, 8usize),
        ("network_fit", 110, 64usize),
        ("network_tight", 111, 16usize),
    ];

    let mut group = c.benchmark_group("strerror_r");

    for (label, errnum, buflen) in cases {
        configure_group(&mut group, buflen);

        group.bench_with_input(BenchmarkId::new("glibc", label), &(errnum, buflen), |b, &(err, n)| {
            b.iter(|| unsafe {
                let mut buf = [0u8; 128];
                let rc = libc_posix_strerror_r(
                    black_box(err),
                    black_box(buf.as_mut_ptr() as *mut c_char),
                    black_box(n),
                );
                black_box(rc);
                black_box(core::ptr::read_volatile(buf.as_ptr()));
            });
        });

        group.bench_with_input(
            BenchmarkId::new("faststrings", label),
            &(errnum, buflen),
            |b, &(err, n)| {
                b.iter(|| {
                    let mut buf = [0u8; 128];
                    let rc = fast_strerror_r(black_box(err), black_box(&mut buf[..n]));
                    black_box(rc);
                    black_box(unsafe { core::ptr::read_volatile(buf.as_ptr()) });
                });
            },
        );
    }

    group.finish();
}

criterion_group!(benches, strerror_benches, strerror_r_benches);
criterion_main!(benches);
