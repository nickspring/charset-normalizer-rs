use charset_normalizer_rs::from_path;
use charset_normalizer_rs::utils::get_large_test_datasets;
use criterion::BenchmarkId;
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::path::PathBuf;

fn bench_foo(paths: &[String]) {
    for path in paths {
        let _ = from_path(&PathBuf::from(path), None);
    }
}

pub fn large_datasets(c: &mut Criterion) {
    let paths: Vec<String> = get_large_test_datasets()
        .unwrap()
        .iter()
        .map(|v| v.0.clone())
        .collect();

    let mut group = c.benchmark_group("sample-size-example");
    group.significance_level(0.1).sample_size(10);
    group.bench_with_input(BenchmarkId::new("large_datasets", ""), &paths, |b, s| {
        b.iter(|| {
            bench_foo(s);
            black_box(())
        });
    });
}

criterion_group!(benches, large_datasets);
criterion_main!(benches);
