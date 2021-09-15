use criterion::{criterion_group, criterion_main, Criterion};
use uuid::Uuid;

use stride::{Candidate, Record};
use stride::examiner::{Examiner, Discord};
use stride::examiner::Outcome::Commit;
use stride::suffix::Suffix;
use stride::suffix::DecideResult::Decided;

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("suffix_insert_only", |b| {
        let mut suffix = Suffix::new(1_000);
        let mut ver = 1;
        b.iter(|| {
            assert_eq!(Ok(()), suffix.insert(vec![], vec![], ver));
            ver += 1;
            assert_eq!(1..ver, suffix.range());
        });
    });

    c.bench_function("suffix_insert_decide", |b| {
        let (min_extent, max_extent) = (1_000, 2_000);
        let mut suffix = Suffix::new(max_extent);
        let mut ver = 1;
        b.iter(|| {
            assert_eq!(Ok(()), suffix.insert(vec![], vec![], ver));
            assert_eq!(Ok(Decided(ver)), suffix.decide(ver));
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
