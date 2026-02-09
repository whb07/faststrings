use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use faststrings::ffs::ffs as fast_ffs;
use std::time::Duration;

unsafe extern "C" {
    #[link_name = "ffs"]
    fn libc_ffs(i: i32) -> i32;
}

fn ffs_benches(c: &mut Criterion) {
    let mut cases: Vec<(String, i32)> = Vec::new();

    cases.push(("zero".to_string(), 0));
    for bit in 0..31 {
        cases.push((format!("bit_{bit}"), 1i32 << bit));
    }
    cases.push(("bit_31".to_string(), i32::MIN));

    cases.extend_from_slice(&[
        ("all_ones".to_string(), -1),
        ("alt_55".to_string(), 0x5555_5555),
        ("alt_aa".to_string(), 0xAAAA_AAAAu32 as i32),
        ("max_i32".to_string(), i32::MAX),
        ("min_plus_1".to_string(), i32::MIN + 1),
        ("rand_a".to_string(), 0x1357_9BDF),
        ("rand_b".to_string(), 0x2468_ACE0),
        ("rand_c_neg".to_string(), -123_456_789),
        ("rand_d_pos".to_string(), 123_456_789),
        ("low_even".to_string(), 0b10),
        ("low_odd".to_string(), 0b11),
    ]);

    let mut group = c.benchmark_group("ffs");
    group.sample_size(50);
    group.warm_up_time(Duration::from_millis(200));
    group.measurement_time(Duration::from_millis(500));

    for (label, value) in cases {
        group.bench_with_input(BenchmarkId::new("glibc", &label), &value, |b, &v| {
            b.iter(|| unsafe {
                let r = libc_ffs(black_box(v));
                black_box(r);
            });
        });

        group.bench_with_input(BenchmarkId::new("faststrings", &label), &value, |b, &v| {
            b.iter(|| {
                let r = fast_ffs(black_box(v));
                black_box(r);
            });
        });
    }

    group.finish();
}

criterion_group!(benches, ffs_benches);
criterion_main!(benches);
