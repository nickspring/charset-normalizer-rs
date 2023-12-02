use charset_normalizer_rs::consts::TOO_BIG_SEQUENCE;
use charset_normalizer_rs::from_bytes;
use criterion::BenchmarkId;
use criterion::{black_box, criterion_group, criterion_main, Criterion};

pub fn large_payload(c: &mut Criterion) {
    let mut payload = b"hello simple ascii "
        .repeat(TOO_BIG_SEQUENCE)
        .as_slice()
        .to_vec();
    payload.extend("我没有埋怨，磋砣的只是一些时间。 磋砣的只是一些时间。".as_bytes());
    c.bench_with_input(BenchmarkId::new("large_payload", ""), &payload, |b, s| {
        b.iter(|| black_box(from_bytes(s, None)));
    });
}

criterion_group!(benches, large_payload);
criterion_main!(benches);
