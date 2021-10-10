use criterion::{criterion_group, criterion_main, Criterion, black_box};
use stride::suffix::{Suffix, AppendResult, CompleteResult};

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("suffix_insert_only", |b| {
        let mut suffix = Suffix::new(1_000);
        let mut ver = 1;
        b.iter(|| {
            assert_eq!(AppendResult::Appended, suffix.append(black_box(vec![]), black_box(vec![]), black_box(ver)));
            ver += 1;
            assert_eq!(1..ver, suffix.range());
        });
    });

    c.bench_function("suffix_insert_decide", |b| {
        let (min_extent, max_extent) = (10_000, 20_000);
        let mut suffix = Suffix::new(max_extent);
        let mut ver = 1;
        b.iter(|| {
            assert_eq!(AppendResult::Appended, suffix.append(black_box(vec![]), black_box(vec![]), black_box(ver)));
            assert_eq!(CompleteResult::Completed(ver), suffix.complete(black_box(ver)));
            suffix.truncate(min_extent, max_extent);
            let range = suffix.range();
            let span = (range.end - range.start) as usize;
            assert!(span > 0 && span <= max_extent, "range {:?}", range);
            ver += 1;
        });
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
