use core::ffi::{c_char, c_void};
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use faststrings::strdup::strdup as fast_strdup;
use faststrings::strndup::strndup as fast_strndup;
use faststrings::strtok::strtok as fast_strtok;
use faststrings::strtok_r::strtok_r as fast_strtok_r;
use std::time::Duration;

unsafe extern "C" {
    #[link_name = "strdup"]
    fn libc_strdup(s: *const c_char) -> *mut c_char;
    #[link_name = "strndup"]
    fn libc_strndup(s: *const c_char, n: usize) -> *mut c_char;
    #[link_name = "strtok"]
    fn libc_strtok(s: *mut c_char, delim: *const c_char) -> *mut c_char;
    #[link_name = "strtok_r"]
    fn libc_strtok_r(s: *mut c_char, delim: *const c_char, saveptr: *mut *mut c_char) -> *mut c_char;
    #[link_name = "free"]
    fn libc_free(p: *mut c_void);
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

fn make_token_input(len: usize) -> Vec<u8> {
    let mut out = vec![0u8; len + 1];
    for i in 0..len {
        out[i] = if i % 8 == 7 { b',' } else { b'a' + (i % 13) as u8 };
    }
    out[len] = 0;
    out
}

fn strdup_benches(c: &mut Criterion) {
    let sizes = [31usize, 256, 4096];
    let mut group = c.benchmark_group("strdup");
    for len in sizes {
        let src = make_c_string(len);
        let label = format!("size_{len}");

        configure_group_for_len(&mut group, len);
        group.throughput(Throughput::Bytes((len + 1) as u64));

        group.bench_with_input(BenchmarkId::new("glibc", &label), &len, |b, _| {
            b.iter(|| unsafe {
                let ptr = libc_strdup(black_box(src.as_ptr() as *const c_char));
                if !ptr.is_null() {
                    black_box(core::ptr::read_volatile(ptr as *const u8));
                    libc_free(ptr as *mut c_void);
                }
            });
        });

        group.bench_with_input(BenchmarkId::new("faststrings", &label), &len, |b, _| {
            b.iter(|| {
                let out = fast_strdup(black_box(&src));
                black_box(out.len());
                black_box(unsafe { core::ptr::read_volatile(out.as_ptr()) });
            });
        });
    }
    group.finish();
}

fn strndup_benches(c: &mut Criterion) {
    let sizes = [31usize, 256, 4096];
    let mut group = c.benchmark_group("strndup");
    for len in sizes {
        let src = make_c_string(len);
        let cases = [("n_trunc", (len / 2).max(1)), ("n_full", len + 16)];

        for (mode, n) in cases {
            let label = format!("size_{len}_{mode}");
            let work = len.min(n) as u64;
            configure_group_for_len(&mut group, len);
            group.throughput(Throughput::Bytes(work.max(1)));

            group.bench_with_input(BenchmarkId::new("glibc", &label), &n, |b, &n| {
                b.iter(|| unsafe {
                    let ptr = libc_strndup(
                        black_box(src.as_ptr() as *const c_char),
                        black_box(n),
                    );
                    if !ptr.is_null() {
                        black_box(core::ptr::read_volatile(ptr as *const u8));
                        libc_free(ptr as *mut c_void);
                    }
                });
            });

            group.bench_with_input(BenchmarkId::new("faststrings", &label), &n, |b, &n| {
                b.iter(|| {
                    let out = fast_strndup(black_box(&src), black_box(n));
                    black_box(out.len());
                    black_box(unsafe { core::ptr::read_volatile(out.as_ptr()) });
                });
            });
        }
    }
    group.finish();
}

fn strtok_benches(c: &mut Criterion) {
    let sizes = [31usize, 256, 4096];
    let delim = b",\0".to_vec();
    let mut group = c.benchmark_group("strtok");

    for len in sizes {
        let template = make_token_input(len);
        let mut work = template.clone();
        let label = format!("size_{len}_comma_split");

        configure_group_for_len(&mut group, len);
        group.throughput(Throughput::Bytes(len as u64));

        group.bench_with_input(BenchmarkId::new("glibc", &label), &len, |b, _| {
            b.iter(|| unsafe {
                work.copy_from_slice(&template);
                let mut count = 0usize;
                let mut tok = libc_strtok(
                    black_box(work.as_mut_ptr() as *mut c_char),
                    black_box(delim.as_ptr() as *const c_char),
                );
                while !tok.is_null() {
                    count += 1;
                    tok = libc_strtok(core::ptr::null_mut(), black_box(delim.as_ptr() as *const c_char));
                }
                black_box(count);
                black_box(core::ptr::read_volatile(work.as_ptr()));
            });
        });

        group.bench_with_input(BenchmarkId::new("faststrings", &label), &len, |b, _| {
            b.iter(|| {
                let mut state = 0usize;
                let mut count = 0usize;
                while let Some(tok) = fast_strtok(black_box(&template), black_box(&delim), &mut state) {
                    black_box(tok.len());
                    count += 1;
                }
                black_box(count);
            });
        });
    }

    group.finish();
}

fn strtok_r_benches(c: &mut Criterion) {
    let sizes = [31usize, 256, 4096];
    let delim = b",\0".to_vec();
    let mut group = c.benchmark_group("strtok_r");

    for len in sizes {
        let template = make_token_input(len);
        let mut work = template.clone();
        let label = format!("size_{len}_comma_split");

        configure_group_for_len(&mut group, len);
        group.throughput(Throughput::Bytes(len as u64));

        group.bench_with_input(BenchmarkId::new("glibc", &label), &len, |b, _| {
            b.iter(|| unsafe {
                work.copy_from_slice(&template);
                let mut count = 0usize;
                let mut save: *mut c_char = core::ptr::null_mut();
                let mut tok = libc_strtok_r(
                    black_box(work.as_mut_ptr() as *mut c_char),
                    black_box(delim.as_ptr() as *const c_char),
                    &mut save,
                );
                while !tok.is_null() {
                    count += 1;
                    tok = libc_strtok_r(
                        core::ptr::null_mut(),
                        black_box(delim.as_ptr() as *const c_char),
                        &mut save,
                    );
                }
                black_box(count);
                black_box(core::ptr::read_volatile(work.as_ptr()));
            });
        });

        group.bench_with_input(BenchmarkId::new("faststrings", &label), &len, |b, _| {
            b.iter(|| {
                let mut save = 0usize;
                let mut count = 0usize;
                while let Some(tok) =
                    fast_strtok_r(black_box(&template), black_box(&delim), &mut save)
                {
                    black_box(tok.len());
                    count += 1;
                }
                black_box(count);
            });
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    strdup_benches,
    strndup_benches,
    strtok_benches,
    strtok_r_benches
);
criterion_main!(benches);
