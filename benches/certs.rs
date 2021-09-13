use criterion::{criterion_group, criterion_main, BatchSize, Criterion};
use uuid::Uuid;

use stride::{Candidate, Record};
use stride::examiner::{Examiner, Discord};
use stride::examiner::Outcome::Commit;

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("learn read 1/write 1", |b| {
        let mut examiner = Examiner::new();
        let mut i = 1;
        b.iter_batched(
            || {
                let candidate = Candidate {
                    rec: Record {
                        xid: Uuid::from_u128(i as u128),
                        readset: vec!["x".into()],
                        writeset: vec!["y".into()],
                        readvers: vec![],
                        snapshot: 0,
                    },
                    ver: i,
                };
                i += 1;
                candidate
            },
            |candidate| {
                examiner.learn(&candidate);
            },
            BatchSize::SmallInput,
        );
    });

    c.bench_function("assess read 1/write 1", |b| {
        let mut examiner = Examiner::new();
        let mut i = 1;
        b.iter_batched(
            || {
                let candidate = Candidate {
                    rec: Record {
                        xid: Uuid::from_u128(i as u128),
                        readset: vec!["x".into()],
                        writeset: vec!["y".into()],
                        readvers: vec![],
                        snapshot: i - 1,
                    },
                    ver: i,
                };
                i += 1;
                candidate
            },
            |candidate| {
                let outcome = examiner.assess(&candidate);
                assert_eq!(Commit(candidate.rec.snapshot, Discord::Permissive), outcome);
            },
            BatchSize::SmallInput,
        );
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
