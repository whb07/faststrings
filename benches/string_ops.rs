use criterion::{
    black_box, criterion_group, criterion_main, AxisScale, BenchmarkId, Criterion,
    PlotConfiguration, Throughput,
};
use std::time::Duration;

#[cfg(target_arch = "x86_64")]
use faststrings::simd::memcpy_unified;

#[cfg(target_arch = "x86_64")]
use faststrings::memcpy::optimized_memcpy_unified;

fn memcpy_benches(c: &mut Criterion) {
    let mut group = c.benchmark_group("memcpy");

    // Log scale for wide size range
    let plot_config = PlotConfiguration::default().summary_scale(AxisScale::Logarithmic);
    group.plot_config(plot_config);

    let sizes = [
        8,
        16,
        32,
        64,
        128,
        256,
        512,
        1024,
        4096,
        16384,
        65536,
        256 * 1024,
        1024 * 1024,
        10 * 1024 * 1024,
    ];

    for size in sizes {
        // Pre-allocate outside the benchmark loop
        let src: Vec<u8> = (0..size).map(|i| (i % 251) as u8).collect();
        let mut dst: Vec<u8> = vec![0u8; size];

        // Tell criterion the throughput for GiB/s calculation
        group.throughput(Throughput::Bytes(size as u64));

        // Tune based on size
        if size >= 1024 * 1024 {
            group.sample_size(50); // Fewer samples for large copies
            group.measurement_time(Duration::from_secs(10));
        } else if size >= 65536 {
            group.sample_size(100);
            group.measurement_time(Duration::from_secs(5));
        } else {
            group.sample_size(200); // More samples for noisy small copies
            group.measurement_time(Duration::from_secs(3));
        }

        // 1. System Libc Baseline
        group.bench_with_input(BenchmarkId::new("System Libc", size), &size, |b, &s| {
            b.iter(|| unsafe {
                std::ptr::copy_nonoverlapping(
                    black_box(src.as_ptr()),
                    black_box(dst.as_mut_ptr()),
                    s,
                )
            });
        });

        // 2. Unified (Basic AVX2)
        #[cfg(target_arch = "x86_64")]
        if is_x86_feature_detected!("avx2") {
            group.bench_with_input(BenchmarkId::new("Unified", size), &size, |b, &s| {
                b.iter(|| unsafe {
                    memcpy_unified(
                        black_box(dst.as_mut_ptr()),
                        black_box(src.as_ptr()),
                        black_box(s),
                    )
                });
            });
        }

        // 3. Optimized (NT/Alignment Refined)
        #[cfg(target_arch = "x86_64")]
        if is_x86_feature_detected!("avx2") {
            group.bench_with_input(BenchmarkId::new("Optimized", size), &size, |b, &s| {
                b.iter(|| unsafe {
                    optimized_memcpy_unified(
                        black_box(dst.as_mut_ptr()),
                        black_box(src.as_ptr()),
                        black_box(s),
                    )
                });
            });
        }
    }

    group.finish();
}

criterion_group!(benches, memcpy_benches);
criterion_main!(benches);

#[cfg(test)]
mod tests {
    #[test]
    fn test_bench_sizes_sanity() {
        let sizes = [
            8,
            16,
            32,
            64,
            128,
            256,
            512,
            1024,
            4096,
            16384,
            65536,
            256 * 1024,
            1024 * 1024,
            10 * 1024 * 1024,
        ];
        assert_eq!(sizes.first().copied(), Some(8));
        assert_eq!(sizes.last().copied(), Some(10 * 1024 * 1024));
    }
}
