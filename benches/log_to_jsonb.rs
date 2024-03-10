use criterion::{black_box, criterion_group, criterion_main, Criterion};
use postgreth::parsing::log_to_jsonb;

pub fn criterion_benchmark(c: &mut Criterion) {
    let abi = include_str!("../testdata/erc20.json");
    let log: &str = include_str!("../testdata/log.json");
    c.bench_function("erc20-Transfer", |b| {
        b.iter(|| log_to_jsonb(black_box(abi), black_box(log)))
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
