use criterion::{criterion_group, criterion_main, Criterion, black_box};
use uuid::Uuid;

use stride::{Candidate, Record};
use stride::examiner::{Examiner, Discord};
use stride::examiner::Outcome::Commit;

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("examiner_learn", |b| {
        let mut examiner = Examiner::new();
        let mut candidate = Candidate {
            rec: Record {
                xid: Uuid::nil(),
                readset: vec!["x".into()],
                writeset: vec!["y".into()],
                readvers: vec![],
                snapshot: 0,
            },
            ver: 1,
        };
        b.iter(|| {
            examiner.learn(black_box(candidate.clone()));
            candidate.rec.snapshot += 1;
            candidate.ver += 1;
        });
    });

    c.bench_function("examiner_assess", |b| {
        let mut examiner = Examiner::new();
        let mut candidate = Candidate {
            rec: Record {
                xid: Uuid::nil(),
                readset: vec!["x".into()],
                writeset: vec!["y".into()],
                readvers: vec![],
                snapshot: 0,
            },
            ver: 1,
        };
        b.iter(|| {
            let outcome = examiner.assess(black_box(candidate.clone()));
            assert_eq!(Commit(candidate.rec.snapshot, Discord::Permissive), outcome);
            candidate.rec.snapshot += 1;
            candidate.ver += 1;
        });
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
