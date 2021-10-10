use criterion::{criterion_group, criterion_main, BatchSize, Criterion};
use uuid::Uuid;

use stride::examiner::Outcome::Commit;
use stride::examiner::{Discord, Examiner, Record, Candidate};
use stride::sortedvec::SortedVec;

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("examiner_learn", |b| {
        let mut examiner = Examiner::new();
        let mut ver = 1;
        b.iter_batched(
            || {
                let candidate = Candidate {
                    rec: Record {
                        xid: Uuid::nil(),
                        readset: vec!["x".into()],
                        writeset: vec!["y".into()],
                        readvers: SortedVec::default(),
                        snapshot: ver - 1,
                    },
                    ver,
                };
                ver += 1;
                candidate
            },
            |candidate| {
                examiner.learn(candidate);
            },
            BatchSize::SmallInput,
        )
    });

    c.bench_function("examiner_assess", |b| {
        let mut examiner = Examiner::new();
        let mut ver = 1;
        b.iter_batched(
            || {
                let candidate = Candidate {
                    rec: Record {
                        xid: Uuid::nil(),
                        readset: vec!["x".into()],
                        writeset: vec!["y".into()],
                        readvers: SortedVec::default(),
                        snapshot: ver - 1,
                    },
                    ver,
                };
                ver += 1;
                candidate
            },
            |candidate| {
                let expected_safepoint = candidate.rec.snapshot;
                let outcome = examiner.assess(candidate);
                assert_eq!(Commit {safepoint: expected_safepoint, discord: Discord::Permissive}, outcome);
            },
            BatchSize::SmallInput,
        )
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
