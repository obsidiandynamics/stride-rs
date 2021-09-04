use criterion::{BatchSize, Criterion, criterion_group, criterion_main};
use uuid::Uuid;

use stride::{Record, Examiner, Discord};
use stride::Outcome::Commit;

fn criterion_benchmark(c: &mut Criterion) {
    // c.bench_function("learn read 1/write 1", |b| {
    //     let mut examiner = Examiner::new();
    //     // b.iter(|| {
    //     //     examiner.learn(black_box(&Candidate {
    //     //         xid: Uuid::from_u128(1),
    //     //         ver: 1,
    //     //         readset: vec!["x".into()],
    //     //         writeset: vec!["y".into()],
    //     //         readvers: vec![],
    //     //         snapshot: 0,
    //     //     }));
    //     // });
    //
    //     let mut i = 1;
    //     b.iter_batched(|| {
    //         let candidate = Candidate {
    //             xid: Uuid::from_u128(i as u128),
    //             ver: i,
    //             readset: vec!["x".into()],
    //             writeset: vec!["y".into()],
    //             readvers: vec![],
    //             snapshot: 0,
    //         };
    //         i += 1;
    //         candidate
    //     }, |candidate| {
    //         examiner.learn(&candidate);
    //     }, BatchSize::SmallInput);
    // });

    c.bench_function("assess read 1/write 1", |b| {
        let mut examiner = Examiner::new();
        let mut i = 1;
        b.iter_batched(|| {
            let candidate = Record {
                xid: Uuid::from_u128(i as u128),
                ver: i,
                readset: vec!["x".into()],
                writeset: vec!["y".into()],
                readvers: vec![],
                snapshot: i - 1,
            };
            i += 1;
            candidate
        }, |candidate| {
            let outcome = examiner.assess(&candidate);
            // assert_outcome(Commit(candidate.snapshot, Discord::Assertive), outcome);
            assert_eq!(Commit(candidate.snapshot, Discord::Permissive), outcome);
        }, BatchSize::SmallInput);
    });

}

// fn assert_outcome(expected: Outcome, actual: Outcome) {
//     if expected != actual {
//         let error = format!("expected: {:?}, actual: {:?}", expected, actual);
//         eprintln!("{}", error);
//         panic!(error);
//     }
// }

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);