use criterion::{black_box, Criterion, criterion_group, criterion_main, BatchSize};

use stride::{Candidate, Record};
use stride::examiner::Discord::Permissive;
use stride::examiner::Examiner;
use stride::examiner::Outcome::Commit;
use stride::suffix::Suffix;
use uuid::Uuid;

fn criterion_benchmark(c: &mut Criterion) {
    let (min_extent, max_extent) = (100_000, 200_000);
    let num_items = 1_000;
    let items = (0..num_items)
        .map(|i| format!("item-{}", i))
        .collect::<Vec<_>>();
    let num_combos = num_items;
    let items_per_combo = 1;
    let item_combos = (0..num_combos)
        .map(|i| {
            (0..items_per_combo).map(|j| items[(i + j) % num_items]
                .clone())
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();

    let setup_candidate = |ver: &mut u64| {
        let readset = &item_combos[*ver as usize % num_combos];
        let writeset = &item_combos[(*ver + 1) as usize % num_combos];
        let candidate = Candidate {
            rec: Record {
                xid: Uuid::from_u128(*ver as u128),
                readset: readset.clone(),
                writeset: writeset.clone(),
                readvers: vec![],
                snapshot: *ver - 1,
            },
            ver: *ver,
        };
        *ver += 1;
        candidate
    };

    c.bench_function("cert_learn", |b| {
        let mut suffix = Suffix::new(max_extent);
        let mut examiner = Examiner::new();
        let mut ver: u64 = 1;
        b.iter_batched(
            || setup_candidate(&mut ver),
            |candidate| {
                let result = suffix.insert(
                    candidate.rec.readset.clone(),
                    candidate.rec.writeset.clone(),
                    candidate.ver,
                );
                assert_eq!(Ok(()), result);
                assert_eq!(Some(candidate.ver + 1), suffix.hwm());

                examiner.learn(candidate);

                let truncated = suffix.truncate(min_extent, max_extent);
                if let Some(truncated_entries) = truncated {
                    let range = suffix.range();
                    let span = (range.end - range.start) as usize;
                    assert!(span > 0 && span <= max_extent, "range {:?}", range);
                    for truncated_entry in truncated_entries {
                        examiner.discard(truncated_entry);
                    }
                }
            },
            BatchSize::SmallInput);
    });

    c.bench_function("cert_assess", |b| {
        let mut suffix = Suffix::new(max_extent);
        let mut examiner = Examiner::new();
        let mut ver: u64 = 1;
        b.iter_batched(
            || setup_candidate(&mut ver),
            |candidate| {
                let result = suffix.insert(
                    candidate.rec.readset.clone(),
                    candidate.rec.writeset.clone(),
                    candidate.ver,
                );
                assert_eq!(Ok(()), result);
                assert_eq!(Some(candidate.ver + 1), suffix.hwm());

                let expected_safepoint = candidate.rec.snapshot;
                let outcome = examiner.assess(candidate);
                assert_eq!(Commit(expected_safepoint, Permissive), outcome);

                let truncated = suffix.truncate(min_extent, max_extent);
                if let Some(truncated_entries) = truncated {
                    let range = suffix.range();
                    let span = (range.end - range.start) as usize;
                    assert!(span > 0 && span <= max_extent, "range {:?}", range);
                    for truncated_entry in truncated_entries {
                        examiner.discard(truncated_entry);
                    }
                }
            },
            BatchSize::SmallInput);
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
