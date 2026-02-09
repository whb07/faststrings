use core::ffi::c_void;
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use faststrings::types::wchar_t;
use faststrings::wcstok::wcstok as fast_wcstok;
use faststrings::wcsdup::wcsdup as fast_wcsdup;
use faststrings::wcsxfrm::wcsxfrm as fast_wcsxfrm;
use faststrings::wide::{
    wcpcpy as fast_wcpcpy, wcpncpy as fast_wcpncpy, wcscasecmp as fast_wcscasecmp,
    wcscat as fast_wcscat, wcschr as fast_wcschr, wcschrnul as fast_wcschrnul,
    wcscmp as fast_wcscmp, wcscoll as fast_wcscoll, wcscpy as fast_wcscpy,
    wcscspn as fast_wcscspn, wcslen as fast_wcslen, wcslcat as fast_wcslcat,
    wcslcpy as fast_wcslcpy, wcsncasecmp as fast_wcsncasecmp, wcsncat as fast_wcsncat,
    wcsncmp as fast_wcsncmp, wcsncpy as fast_wcsncpy, wcsnlen as fast_wcsnlen,
    wcspbrk as fast_wcspbrk, wcsrchr as fast_wcsrchr, wcsspn as fast_wcsspn,
    wcsstr as fast_wcsstr,
};
use std::time::Duration;

unsafe extern "C" {
    #[link_name = "wcslen"]
    fn libc_wcslen(s: *const wchar_t) -> usize;
    #[link_name = "wcsnlen"]
    fn libc_wcsnlen(s: *const wchar_t, maxlen: usize) -> usize;
    #[link_name = "wcscpy"]
    fn libc_wcscpy(dest: *mut wchar_t, src: *const wchar_t) -> *mut wchar_t;
    #[link_name = "wcsncpy"]
    fn libc_wcsncpy(dest: *mut wchar_t, src: *const wchar_t, n: usize) -> *mut wchar_t;
    #[link_name = "wcpcpy"]
    fn libc_wcpcpy(dest: *mut wchar_t, src: *const wchar_t) -> *mut wchar_t;
    #[link_name = "wcpncpy"]
    fn libc_wcpncpy(dest: *mut wchar_t, src: *const wchar_t, n: usize) -> *mut wchar_t;
    #[link_name = "wcscat"]
    fn libc_wcscat(dest: *mut wchar_t, src: *const wchar_t) -> *mut wchar_t;
    #[link_name = "wcsncat"]
    fn libc_wcsncat(dest: *mut wchar_t, src: *const wchar_t, n: usize) -> *mut wchar_t;
    #[link_name = "wcscmp"]
    fn libc_wcscmp(s1: *const wchar_t, s2: *const wchar_t) -> i32;
    #[link_name = "wcsncmp"]
    fn libc_wcsncmp(s1: *const wchar_t, s2: *const wchar_t, n: usize) -> i32;
    #[link_name = "wcscoll"]
    fn libc_wcscoll(s1: *const wchar_t, s2: *const wchar_t) -> i32;
    #[link_name = "wcschr"]
    fn libc_wcschr(s: *const wchar_t, c: wchar_t) -> *mut wchar_t;
    #[link_name = "wcsrchr"]
    fn libc_wcsrchr(s: *const wchar_t, c: wchar_t) -> *mut wchar_t;
    #[link_name = "wcsstr"]
    fn libc_wcsstr(haystack: *const wchar_t, needle: *const wchar_t) -> *mut wchar_t;
    #[link_name = "wcsspn"]
    fn libc_wcsspn(s: *const wchar_t, accept: *const wchar_t) -> usize;
    #[link_name = "wcscspn"]
    fn libc_wcscspn(s: *const wchar_t, reject: *const wchar_t) -> usize;
    #[link_name = "wcspbrk"]
    fn libc_wcspbrk(s: *const wchar_t, accept: *const wchar_t) -> *mut wchar_t;
    #[link_name = "wcscasecmp"]
    fn libc_wcscasecmp(s1: *const wchar_t, s2: *const wchar_t) -> i32;
    #[link_name = "wcsncasecmp"]
    fn libc_wcsncasecmp(s1: *const wchar_t, s2: *const wchar_t, n: usize) -> i32;
    #[link_name = "wcschrnul"]
    fn libc_wcschrnul(s: *const wchar_t, c: wchar_t) -> *mut wchar_t;
    #[link_name = "wcslcpy"]
    fn libc_wcslcpy(dest: *mut wchar_t, src: *const wchar_t, size: usize) -> usize;
    #[link_name = "wcslcat"]
    fn libc_wcslcat(dest: *mut wchar_t, src: *const wchar_t, size: usize) -> usize;
    #[link_name = "wcstok"]
    fn libc_wcstok(s: *mut wchar_t, delim: *const wchar_t, saveptr: *mut *mut wchar_t)
    -> *mut wchar_t;
    #[link_name = "wcsxfrm"]
    fn libc_wcsxfrm(dest: *mut wchar_t, src: *const wchar_t, n: usize) -> usize;
    #[link_name = "wcsdup"]
    fn libc_wcsdup(src: *const wchar_t) -> *mut wchar_t;
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

fn bytes_for(elems: usize) -> u64 {
    (elems * core::mem::size_of::<wchar_t>()) as u64
}

fn make_wide_c_string(len: usize) -> Vec<wchar_t> {
    let mut out = vec![0 as wchar_t; len + 1];
    for i in 0..len {
        out[i] = (b'a' + ((i * 7 + len * 11 + 3) % 23) as u8) as wchar_t;
    }
    out[len] = 0;
    out
}

fn make_upper_copy(src: &[wchar_t]) -> Vec<wchar_t> {
    let mut out = src.to_vec();
    for v in &mut out {
        if (b'a' as wchar_t..=b'z' as wchar_t).contains(v) {
            *v -= 32;
        }
    }
    out
}

fn make_prefixed_wide(total_len: usize, prefix_len: usize) -> Vec<wchar_t> {
    let mut dest = vec![0 as wchar_t; total_len];
    for i in 0..prefix_len {
        dest[i] = (b'k' + (i % 7) as u8) as wchar_t;
    }
    dest[prefix_len] = 0;
    dest
}

fn len_benches(c: &mut Criterion) {
    let sizes = [31usize, 256, 4096];

    let mut len_group = c.benchmark_group("wcslen");
    for len in sizes {
        let s = make_wide_c_string(len);
        let label = format!("size_{len}");
        configure_group_for_len(&mut len_group, len);
        len_group.throughput(Throughput::Bytes(bytes_for(len)));
        len_group.bench_with_input(BenchmarkId::new("glibc", &label), &len, |b, _| {
            b.iter(|| unsafe {
                black_box(libc_wcslen(black_box(s.as_ptr())));
            });
        });
        len_group.bench_with_input(BenchmarkId::new("faststrings", &label), &len, |b, _| {
            b.iter(|| {
                black_box(fast_wcslen(black_box(&s)));
            });
        });
    }
    len_group.finish();

    let mut nlen_group = c.benchmark_group("wcsnlen");
    for len in sizes {
        let s = make_wide_c_string(len);
        let maxlen = (len / 2).max(1);
        let label = format!("size_{len}_max_half");
        configure_group_for_len(&mut nlen_group, len);
        nlen_group.throughput(Throughput::Bytes(bytes_for(maxlen)));
        nlen_group.bench_with_input(BenchmarkId::new("glibc", &label), &maxlen, |b, &n| {
            b.iter(|| unsafe {
                black_box(libc_wcsnlen(black_box(s.as_ptr()), black_box(n)));
            });
        });
        nlen_group.bench_with_input(BenchmarkId::new("faststrings", &label), &maxlen, |b, &n| {
            b.iter(|| {
                black_box(fast_wcsnlen(black_box(&s), black_box(n)));
            });
        });
    }
    nlen_group.finish();
}

fn copy_cat_benches(c: &mut Criterion) {
    let sizes = [31usize, 256, 4096];

    let mut wcscpy_group = c.benchmark_group("wcscpy");
    for len in sizes {
        let src = make_wide_c_string(len);
        let template = vec![0x11 as wchar_t; len + 64];
        let mut dst = template.clone();
        let label = format!("size_{len}");
        configure_group_for_len(&mut wcscpy_group, len);
        wcscpy_group.throughput(Throughput::Bytes(bytes_for(len)));
        wcscpy_group.bench_with_input(BenchmarkId::new("glibc", &label), &len, |b, _| {
            b.iter(|| unsafe {
                dst.copy_from_slice(&template);
                let ret = libc_wcscpy(black_box(dst.as_mut_ptr()), black_box(src.as_ptr()));
                black_box((ret as usize) - (dst.as_ptr() as usize));
                black_box(core::ptr::read_volatile(dst.as_ptr()));
            });
        });
        wcscpy_group.bench_with_input(BenchmarkId::new("faststrings", &label), &len, |b, _| {
            b.iter(|| {
                dst.copy_from_slice(&template);
                black_box(fast_wcscpy(black_box(&mut dst), black_box(&src)));
                black_box(unsafe { core::ptr::read_volatile(dst.as_ptr()) });
            });
        });
    }
    wcscpy_group.finish();

    let mut wcsncpy_group = c.benchmark_group("wcsncpy");
    for len in sizes {
        let src = make_wide_c_string(len);
        let n = (len / 2).max(1);
        let template = vec![0x22 as wchar_t; n + 64];
        let mut dst = template.clone();
        let label = format!("size_{len}_n_half");
        configure_group_for_len(&mut wcsncpy_group, len);
        wcsncpy_group.throughput(Throughput::Bytes(bytes_for(n)));
        wcsncpy_group.bench_with_input(BenchmarkId::new("glibc", &label), &n, |b, &n| {
            b.iter(|| unsafe {
                dst.copy_from_slice(&template);
                let ret = libc_wcsncpy(
                    black_box(dst.as_mut_ptr()),
                    black_box(src.as_ptr()),
                    black_box(n),
                );
                black_box((ret as usize) - (dst.as_ptr() as usize));
                black_box(core::ptr::read_volatile(dst.as_ptr()));
            });
        });
        wcsncpy_group.bench_with_input(BenchmarkId::new("faststrings", &label), &n, |b, &n| {
            b.iter(|| {
                dst.copy_from_slice(&template);
                black_box(fast_wcsncpy(black_box(&mut dst), black_box(&src), black_box(n)));
                black_box(unsafe { core::ptr::read_volatile(dst.as_ptr()) });
            });
        });
    }
    wcsncpy_group.finish();

    let mut wcpcpy_group = c.benchmark_group("wcpcpy");
    for len in sizes {
        let src = make_wide_c_string(len);
        let template = vec![0x33 as wchar_t; len + 64];
        let mut dst = template.clone();
        let label = format!("size_{len}");
        configure_group_for_len(&mut wcpcpy_group, len);
        wcpcpy_group.throughput(Throughput::Bytes(bytes_for(len)));
        wcpcpy_group.bench_with_input(BenchmarkId::new("glibc", &label), &len, |b, _| {
            b.iter(|| unsafe {
                dst.copy_from_slice(&template);
                let end = libc_wcpcpy(black_box(dst.as_mut_ptr()), black_box(src.as_ptr()));
                black_box((end as usize) - (dst.as_ptr() as usize));
                black_box(core::ptr::read_volatile(dst.as_ptr()));
            });
        });
        wcpcpy_group.bench_with_input(BenchmarkId::new("faststrings", &label), &len, |b, _| {
            b.iter(|| {
                dst.copy_from_slice(&template);
                black_box(fast_wcpcpy(black_box(&mut dst), black_box(&src)));
                black_box(unsafe { core::ptr::read_volatile(dst.as_ptr()) });
            });
        });
    }
    wcpcpy_group.finish();

    let mut wcpncpy_group = c.benchmark_group("wcpncpy");
    for len in sizes {
        let src = make_wide_c_string(len);
        let n = (len / 2).max(1);
        let template = vec![0x44 as wchar_t; n + 64];
        let mut dst = template.clone();
        let label = format!("size_{len}_n_half");
        configure_group_for_len(&mut wcpncpy_group, len);
        wcpncpy_group.throughput(Throughput::Bytes(bytes_for(n)));
        wcpncpy_group.bench_with_input(BenchmarkId::new("glibc", &label), &n, |b, &n| {
            b.iter(|| unsafe {
                dst.copy_from_slice(&template);
                let end = libc_wcpncpy(
                    black_box(dst.as_mut_ptr()),
                    black_box(src.as_ptr()),
                    black_box(n),
                );
                black_box((end as usize) - (dst.as_ptr() as usize));
                black_box(core::ptr::read_volatile(dst.as_ptr()));
            });
        });
        wcpncpy_group.bench_with_input(BenchmarkId::new("faststrings", &label), &n, |b, &n| {
            b.iter(|| {
                dst.copy_from_slice(&template);
                black_box(fast_wcpncpy(black_box(&mut dst), black_box(&src), black_box(n)));
                black_box(unsafe { core::ptr::read_volatile(dst.as_ptr()) });
            });
        });
    }
    wcpncpy_group.finish();

    let mut wcscat_group = c.benchmark_group("wcscat");
    for len in sizes {
        let prefix = (len / 2).max(1);
        let src_len = len - prefix;
        let src = make_wide_c_string(src_len);
        let template = make_prefixed_wide(len + 64, prefix);
        let mut dst = template.clone();
        let label = format!("size_{len}_append");
        configure_group_for_len(&mut wcscat_group, len);
        wcscat_group.throughput(Throughput::Bytes(bytes_for(src_len)));
        wcscat_group.bench_with_input(BenchmarkId::new("glibc", &label), &len, |b, _| {
            b.iter(|| unsafe {
                dst.copy_from_slice(&template);
                let ret = libc_wcscat(black_box(dst.as_mut_ptr()), black_box(src.as_ptr()));
                black_box((ret as usize) - (dst.as_ptr() as usize));
                black_box(core::ptr::read_volatile(dst.as_ptr()));
            });
        });
        wcscat_group.bench_with_input(BenchmarkId::new("faststrings", &label), &len, |b, _| {
            b.iter(|| {
                dst.copy_from_slice(&template);
                black_box(fast_wcscat(black_box(&mut dst), black_box(&src)));
                black_box(unsafe { core::ptr::read_volatile(dst.as_ptr()) });
            });
        });
    }
    wcscat_group.finish();

    let mut wcsncat_group = c.benchmark_group("wcsncat");
    for len in sizes {
        let prefix = (len / 2).max(1);
        let src_len = len - prefix;
        let src = make_wide_c_string(src_len);
        let n = (src_len / 2).max(1);
        let template = make_prefixed_wide(len + 64, prefix);
        let mut dst = template.clone();
        let label = format!("size_{len}_n_half");
        configure_group_for_len(&mut wcsncat_group, len);
        wcsncat_group.throughput(Throughput::Bytes(bytes_for(n)));
        wcsncat_group.bench_with_input(BenchmarkId::new("glibc", &label), &n, |b, &n| {
            b.iter(|| unsafe {
                dst.copy_from_slice(&template);
                let ret = libc_wcsncat(
                    black_box(dst.as_mut_ptr()),
                    black_box(src.as_ptr()),
                    black_box(n),
                );
                black_box((ret as usize) - (dst.as_ptr() as usize));
                black_box(core::ptr::read_volatile(dst.as_ptr()));
            });
        });
        wcsncat_group.bench_with_input(BenchmarkId::new("faststrings", &label), &n, |b, &n| {
            b.iter(|| {
                dst.copy_from_slice(&template);
                black_box(fast_wcsncat(black_box(&mut dst), black_box(&src), black_box(n)));
                black_box(unsafe { core::ptr::read_volatile(dst.as_ptr()) });
            });
        });
    }
    wcsncat_group.finish();

    let mut wcslcpy_group = c.benchmark_group("wcslcpy");
    for len in sizes {
        let src = make_wide_c_string(len);
        let dst_len = len + 64;
        let template = vec![0x55 as wchar_t; dst_len];
        let mut dst = template.clone();
        let label = format!("size_{len}_fit");
        configure_group_for_len(&mut wcslcpy_group, len);
        wcslcpy_group.throughput(Throughput::Bytes(bytes_for(len)));
        wcslcpy_group.bench_with_input(BenchmarkId::new("glibc", &label), &dst_len, |b, &n| {
            b.iter(|| unsafe {
                dst.copy_from_slice(&template);
                black_box(libc_wcslcpy(
                    black_box(dst.as_mut_ptr()),
                    black_box(src.as_ptr()),
                    black_box(n),
                ));
                black_box(core::ptr::read_volatile(dst.as_ptr()));
            });
        });
        wcslcpy_group.bench_with_input(
            BenchmarkId::new("faststrings", &label),
            &dst_len,
            |b, _| {
                b.iter(|| {
                    dst.copy_from_slice(&template);
                    black_box(fast_wcslcpy(black_box(&mut dst), black_box(&src)));
                    black_box(unsafe { core::ptr::read_volatile(dst.as_ptr()) });
                });
            },
        );
    }
    wcslcpy_group.finish();

    let mut wcslcat_group = c.benchmark_group("wcslcat");
    for len in sizes {
        let prefix = (len / 2).max(1);
        let src = make_wide_c_string(len);
        let dst_len = prefix + len + 64;
        let template = make_prefixed_wide(dst_len, prefix);
        let mut dst = template.clone();
        let label = format!("size_{len}_fit");
        configure_group_for_len(&mut wcslcat_group, len);
        wcslcat_group.throughput(Throughput::Bytes(bytes_for(len)));
        wcslcat_group.bench_with_input(BenchmarkId::new("glibc", &label), &dst_len, |b, &n| {
            b.iter(|| unsafe {
                dst.copy_from_slice(&template);
                black_box(libc_wcslcat(
                    black_box(dst.as_mut_ptr()),
                    black_box(src.as_ptr()),
                    black_box(n),
                ));
                black_box(core::ptr::read_volatile(dst.as_ptr()));
            });
        });
        wcslcat_group.bench_with_input(
            BenchmarkId::new("faststrings", &label),
            &dst_len,
            |b, _| {
                b.iter(|| {
                    dst.copy_from_slice(&template);
                    black_box(fast_wcslcat(black_box(&mut dst), black_box(&src)));
                    black_box(unsafe { core::ptr::read_volatile(dst.as_ptr()) });
                });
            },
        );
    }
    wcslcat_group.finish();
}

fn cmp_benches(c: &mut Criterion) {
    let sizes = [31usize, 256, 4096];

    let mut wcscmp_group = c.benchmark_group("wcscmp");
    for len in sizes {
        let lhs = make_wide_c_string(len);
        let mut rhs = make_wide_c_string(len);
        rhs[len / 2] = b'z' as wchar_t;
        let label = format!("size_{len}_diff_mid");
        configure_group_for_len(&mut wcscmp_group, len);
        wcscmp_group.throughput(Throughput::Bytes(bytes_for(len / 2 + 1)));
        wcscmp_group.bench_with_input(BenchmarkId::new("glibc", &label), &len, |b, _| {
            b.iter(|| unsafe {
                black_box(libc_wcscmp(black_box(lhs.as_ptr()), black_box(rhs.as_ptr())));
            });
        });
        wcscmp_group.bench_with_input(BenchmarkId::new("faststrings", &label), &len, |b, _| {
            b.iter(|| {
                black_box(fast_wcscmp(black_box(&lhs), black_box(&rhs)));
            });
        });
    }
    wcscmp_group.finish();

    let mut wcsncmp_group = c.benchmark_group("wcsncmp");
    for len in sizes {
        let lhs = make_wide_c_string(len);
        let mut rhs = make_wide_c_string(len);
        let n = (len / 2).max(1);
        rhs[0] = b'z' as wchar_t;
        let label = format!("size_{len}_n_half_diff_first");
        configure_group_for_len(&mut wcsncmp_group, len);
        wcsncmp_group.throughput(Throughput::Bytes(bytes_for(1)));
        wcsncmp_group.bench_with_input(BenchmarkId::new("glibc", &label), &n, |b, &n| {
            b.iter(|| unsafe {
                black_box(libc_wcsncmp(
                    black_box(lhs.as_ptr()),
                    black_box(rhs.as_ptr()),
                    black_box(n),
                ));
            });
        });
        wcsncmp_group.bench_with_input(BenchmarkId::new("faststrings", &label), &n, |b, &n| {
            b.iter(|| {
                black_box(fast_wcsncmp(black_box(&lhs), black_box(&rhs), black_box(n)));
            });
        });
    }
    wcsncmp_group.finish();

    let mut wcscoll_group = c.benchmark_group("wcscoll");
    for len in sizes {
        let lhs = make_wide_c_string(len);
        let mut rhs = make_wide_c_string(len);
        rhs[len / 2] = b'z' as wchar_t;
        let label = format!("size_{len}_diff_mid");
        configure_group_for_len(&mut wcscoll_group, len);
        wcscoll_group.throughput(Throughput::Bytes(bytes_for(len / 2 + 1)));
        wcscoll_group.bench_with_input(BenchmarkId::new("glibc", &label), &len, |b, _| {
            b.iter(|| unsafe {
                black_box(libc_wcscoll(black_box(lhs.as_ptr()), black_box(rhs.as_ptr())));
            });
        });
        wcscoll_group.bench_with_input(BenchmarkId::new("faststrings", &label), &len, |b, _| {
            b.iter(|| {
                black_box(fast_wcscoll(black_box(&lhs), black_box(&rhs)));
            });
        });
    }
    wcscoll_group.finish();

    let mut wcscasecmp_group = c.benchmark_group("wcscasecmp");
    for len in sizes {
        let lhs = make_wide_c_string(len);
        let rhs = make_upper_copy(&lhs);
        let label = format!("size_{len}_equal_casefold");
        configure_group_for_len(&mut wcscasecmp_group, len);
        wcscasecmp_group.throughput(Throughput::Bytes(bytes_for(len)));
        wcscasecmp_group.bench_with_input(BenchmarkId::new("glibc", &label), &len, |b, _| {
            b.iter(|| unsafe {
                black_box(libc_wcscasecmp(
                    black_box(lhs.as_ptr()),
                    black_box(rhs.as_ptr()),
                ));
            });
        });
        wcscasecmp_group.bench_with_input(
            BenchmarkId::new("faststrings", &label),
            &len,
            |b, _| {
                b.iter(|| {
                    black_box(fast_wcscasecmp(black_box(&lhs), black_box(&rhs)));
                });
            },
        );
    }
    wcscasecmp_group.finish();

    let mut wcsncasecmp_group = c.benchmark_group("wcsncasecmp");
    for len in sizes {
        let lhs = make_wide_c_string(len);
        let rhs = make_upper_copy(&lhs);
        let n = (len / 2).max(1);
        let label = format!("size_{len}_n_half_equal");
        configure_group_for_len(&mut wcsncasecmp_group, len);
        wcsncasecmp_group.throughput(Throughput::Bytes(bytes_for(n)));
        wcsncasecmp_group.bench_with_input(BenchmarkId::new("glibc", &label), &n, |b, &n| {
            b.iter(|| unsafe {
                black_box(libc_wcsncasecmp(
                    black_box(lhs.as_ptr()),
                    black_box(rhs.as_ptr()),
                    black_box(n),
                ));
            });
        });
        wcsncasecmp_group.bench_with_input(
            BenchmarkId::new("faststrings", &label),
            &n,
            |b, &n| {
                b.iter(|| {
                    black_box(fast_wcsncasecmp(black_box(&lhs), black_box(&rhs), black_box(n)));
                });
            },
        );
    }
    wcsncasecmp_group.finish();
}

fn search_benches(c: &mut Criterion) {
    let sizes = [31usize, 256, 4096];

    let mut wcschr_group = c.benchmark_group("wcschr");
    for len in sizes {
        let mut s = make_wide_c_string(len);
        let target = b'x' as wchar_t;
        let pos = len / 2;
        s[pos] = target;
        let label = format!("size_{len}_hit_mid");
        configure_group_for_len(&mut wcschr_group, len);
        wcschr_group.throughput(Throughput::Bytes(bytes_for(pos + 1)));
        wcschr_group.bench_with_input(BenchmarkId::new("glibc", &label), &len, |b, _| {
            b.iter(|| unsafe {
                let base = s.as_ptr() as usize;
                let p = libc_wcschr(black_box(s.as_ptr()), black_box(target));
                let rv = if p.is_null() { usize::MAX } else { (p as usize) - base };
                black_box(rv);
            });
        });
        wcschr_group.bench_with_input(BenchmarkId::new("faststrings", &label), &len, |b, _| {
            b.iter(|| {
                black_box(fast_wcschr(black_box(&s), black_box(target)).unwrap_or(usize::MAX));
            });
        });
    }
    wcschr_group.finish();

    let mut wcsrchr_group = c.benchmark_group("wcsrchr");
    for len in sizes {
        let mut s = make_wide_c_string(len);
        let target = b'x' as wchar_t;
        s[len / 2] = target;
        s[len - 1] = target;
        let label = format!("size_{len}_hit_tail");
        configure_group_for_len(&mut wcsrchr_group, len);
        wcsrchr_group.throughput(Throughput::Bytes(bytes_for(len)));
        wcsrchr_group.bench_with_input(BenchmarkId::new("glibc", &label), &len, |b, _| {
            b.iter(|| unsafe {
                let base = s.as_ptr() as usize;
                let p = libc_wcsrchr(black_box(s.as_ptr()), black_box(target));
                let rv = if p.is_null() { usize::MAX } else { (p as usize) - base };
                black_box(rv);
            });
        });
        wcsrchr_group.bench_with_input(BenchmarkId::new("faststrings", &label), &len, |b, _| {
            b.iter(|| {
                black_box(fast_wcsrchr(black_box(&s), black_box(target)).unwrap_or(usize::MAX));
            });
        });
    }
    wcsrchr_group.finish();

    let mut wcschrnul_group = c.benchmark_group("wcschrnul");
    for len in sizes {
        let s = make_wide_c_string(len);
        let target = b'Z' as wchar_t;
        let label = format!("size_{len}_miss");
        configure_group_for_len(&mut wcschrnul_group, len);
        wcschrnul_group.throughput(Throughput::Bytes(bytes_for(len)));
        wcschrnul_group.bench_with_input(BenchmarkId::new("glibc", &label), &len, |b, _| {
            b.iter(|| unsafe {
                let base = s.as_ptr() as usize;
                let p = libc_wcschrnul(black_box(s.as_ptr()), black_box(target));
                black_box((p as usize) - base);
            });
        });
        wcschrnul_group.bench_with_input(
            BenchmarkId::new("faststrings", &label),
            &len,
            |b, _| {
                b.iter(|| {
                    black_box(fast_wcschrnul(black_box(&s), black_box(target)));
                });
            },
        );
    }
    wcschrnul_group.finish();

    let mut wcsstr_group = c.benchmark_group("wcsstr");
    for len in sizes {
        let mut hay = make_wide_c_string(len);
        let needle = vec![b'q' as wchar_t, b'r' as wchar_t, b's' as wchar_t, b't' as wchar_t, 0];
        let pos = len / 2;
        hay[pos] = b'q' as wchar_t;
        hay[pos + 1] = b'r' as wchar_t;
        hay[pos + 2] = b's' as wchar_t;
        hay[pos + 3] = b't' as wchar_t;
        let label = format!("size_{len}_hit_mid");
        configure_group_for_len(&mut wcsstr_group, len);
        wcsstr_group.throughput(Throughput::Bytes(bytes_for(pos + 4)));
        wcsstr_group.bench_with_input(BenchmarkId::new("glibc", &label), &len, |b, _| {
            b.iter(|| unsafe {
                let base = hay.as_ptr() as usize;
                let p = libc_wcsstr(black_box(hay.as_ptr()), black_box(needle.as_ptr()));
                let rv = if p.is_null() { usize::MAX } else { (p as usize) - base };
                black_box(rv);
            });
        });
        wcsstr_group.bench_with_input(BenchmarkId::new("faststrings", &label), &len, |b, _| {
            b.iter(|| {
                black_box(fast_wcsstr(black_box(&hay), black_box(&needle)).unwrap_or(usize::MAX));
            });
        });
    }
    wcsstr_group.finish();

    let mut wcsspn_group = c.benchmark_group("wcsspn");
    for len in sizes {
        let mut s = vec![0 as wchar_t; len + 1];
        for (i, v) in s[..len].iter_mut().enumerate() {
            *v = [b'a', b'b', b'c'][i % 3] as wchar_t;
        }
        s[len] = 0;
        let accept = vec![b'a' as wchar_t, b'b' as wchar_t, b'c' as wchar_t, 0];
        let label = format!("size_{len}_full");
        configure_group_for_len(&mut wcsspn_group, len);
        wcsspn_group.throughput(Throughput::Bytes(bytes_for(len)));
        wcsspn_group.bench_with_input(BenchmarkId::new("glibc", &label), &len, |b, _| {
            b.iter(|| unsafe {
                black_box(libc_wcsspn(
                    black_box(s.as_ptr()),
                    black_box(accept.as_ptr()),
                ));
            });
        });
        wcsspn_group.bench_with_input(BenchmarkId::new("faststrings", &label), &len, |b, _| {
            b.iter(|| {
                black_box(fast_wcsspn(black_box(&s), black_box(&accept)));
            });
        });
    }
    wcsspn_group.finish();

    let mut wcscspn_group = c.benchmark_group("wcscspn");
    for len in sizes {
        let mut s = vec![0 as wchar_t; len + 1];
        for (i, v) in s[..len].iter_mut().enumerate() {
            *v = [b'a', b'b', b'c'][i % 3] as wchar_t;
        }
        let pos = len / 2;
        s[pos] = b'x' as wchar_t;
        s[len] = 0;
        let reject = vec![b'x' as wchar_t, 0];
        let label = format!("size_{len}_hit_mid");
        configure_group_for_len(&mut wcscspn_group, len);
        wcscspn_group.throughput(Throughput::Bytes(bytes_for(pos + 1)));
        wcscspn_group.bench_with_input(BenchmarkId::new("glibc", &label), &len, |b, _| {
            b.iter(|| unsafe {
                black_box(libc_wcscspn(
                    black_box(s.as_ptr()),
                    black_box(reject.as_ptr()),
                ));
            });
        });
        wcscspn_group.bench_with_input(BenchmarkId::new("faststrings", &label), &len, |b, _| {
            b.iter(|| {
                black_box(fast_wcscspn(black_box(&s), black_box(&reject)));
            });
        });
    }
    wcscspn_group.finish();

    let mut wcspbrk_group = c.benchmark_group("wcspbrk");
    for len in sizes {
        let mut s = vec![0 as wchar_t; len + 1];
        for (i, v) in s[..len].iter_mut().enumerate() {
            *v = [b'a', b'b', b'c'][i % 3] as wchar_t;
        }
        let pos = len / 2;
        s[pos] = b'x' as wchar_t;
        s[len] = 0;
        let accept = vec![b'x' as wchar_t, 0];
        let label = format!("size_{len}_hit_mid");
        configure_group_for_len(&mut wcspbrk_group, len);
        wcspbrk_group.throughput(Throughput::Bytes(bytes_for(pos + 1)));
        wcspbrk_group.bench_with_input(BenchmarkId::new("glibc", &label), &len, |b, _| {
            b.iter(|| unsafe {
                let base = s.as_ptr() as usize;
                let p = libc_wcspbrk(black_box(s.as_ptr()), black_box(accept.as_ptr()));
                let rv = if p.is_null() { usize::MAX } else { (p as usize) - base };
                black_box(rv);
            });
        });
        wcspbrk_group.bench_with_input(BenchmarkId::new("faststrings", &label), &len, |b, _| {
            b.iter(|| {
                black_box(fast_wcspbrk(black_box(&s), black_box(&accept)).unwrap_or(usize::MAX));
            });
        });
    }
    wcspbrk_group.finish();
}

fn misc_benches(c: &mut Criterion) {
    let sizes = [31usize, 256, 4096];

    let mut wcstok_group = c.benchmark_group("wcstok");
    let delim = vec![b',' as wchar_t, 0];
    for len in sizes {
        let mut template = vec![0 as wchar_t; len + 1];
        for i in 0..len {
            template[i] = if i % 8 == 7 {
                b',' as wchar_t
            } else {
                (b'a' + (i % 13) as u8) as wchar_t
            };
        }
        template[len] = 0;
        let mut work = template.clone();
        let label = format!("size_{len}_comma_split");
        configure_group_for_len(&mut wcstok_group, len);
        wcstok_group.throughput(Throughput::Bytes(bytes_for(len)));
        wcstok_group.bench_with_input(BenchmarkId::new("glibc", &label), &len, |b, _| {
            b.iter(|| unsafe {
                work.copy_from_slice(&template);
                let mut save: *mut wchar_t = core::ptr::null_mut();
                let mut count = 0usize;
                let mut tok =
                    libc_wcstok(black_box(work.as_mut_ptr()), black_box(delim.as_ptr()), &mut save);
                while !tok.is_null() {
                    count += 1;
                    tok = libc_wcstok(core::ptr::null_mut(), black_box(delim.as_ptr()), &mut save);
                }
                black_box(count);
                black_box(core::ptr::read_volatile(work.as_ptr()));
            });
        });
        wcstok_group.bench_with_input(BenchmarkId::new("faststrings", &label), &len, |b, _| {
            b.iter(|| {
                let mut save = 0usize;
                let mut count = 0usize;
                while let Some(tok) = fast_wcstok(black_box(&template), black_box(&delim), &mut save)
                {
                    black_box(tok.len());
                    count += 1;
                }
                black_box(count);
            });
        });
    }
    wcstok_group.finish();

    let mut wcsxfrm_group = c.benchmark_group("wcsxfrm");
    for len in sizes {
        let src = make_wide_c_string(len);
        let dst_len = len + 64;
        let template = vec![0x77 as wchar_t; dst_len];
        let mut dst = template.clone();
        let label = format!("size_{len}_fit");
        configure_group_for_len(&mut wcsxfrm_group, len);
        wcsxfrm_group.throughput(Throughput::Bytes(bytes_for(len)));
        wcsxfrm_group.bench_with_input(BenchmarkId::new("glibc", &label), &dst_len, |b, &n| {
            b.iter(|| unsafe {
                dst.copy_from_slice(&template);
                black_box(libc_wcsxfrm(
                    black_box(dst.as_mut_ptr()),
                    black_box(src.as_ptr()),
                    black_box(n),
                ));
                black_box(core::ptr::read_volatile(dst.as_ptr()));
            });
        });
        wcsxfrm_group.bench_with_input(
            BenchmarkId::new("faststrings", &label),
            &dst_len,
            |b, _| {
                b.iter(|| {
                    dst.copy_from_slice(&template);
                    black_box(fast_wcsxfrm(black_box(&mut dst), black_box(&src)));
                    black_box(unsafe { core::ptr::read_volatile(dst.as_ptr()) });
                });
            },
        );
    }
    wcsxfrm_group.finish();

    let mut wcsdup_group = c.benchmark_group("wcsdup");
    for len in sizes {
        let src = make_wide_c_string(len);
        let label = format!("size_{len}");
        configure_group_for_len(&mut wcsdup_group, len);
        wcsdup_group.throughput(Throughput::Bytes(bytes_for(len)));
        wcsdup_group.bench_with_input(BenchmarkId::new("glibc", &label), &len, |b, _| {
            b.iter(|| unsafe {
                let ptr = libc_wcsdup(black_box(src.as_ptr()));
                if !ptr.is_null() {
                    black_box(core::ptr::read_volatile(ptr));
                    libc_free(ptr as *mut c_void);
                }
            });
        });
        wcsdup_group.bench_with_input(BenchmarkId::new("faststrings", &label), &len, |b, _| {
            b.iter(|| {
                let out = fast_wcsdup(black_box(&src));
                black_box(out.len());
                black_box(unsafe { core::ptr::read_volatile(out.as_ptr()) });
            });
        });
    }
    wcsdup_group.finish();
}

criterion_group!(
    benches,
    len_benches,
    copy_cat_benches,
    cmp_benches,
    search_benches,
    misc_benches
);
criterion_main!(benches);
