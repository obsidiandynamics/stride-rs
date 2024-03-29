use criterion::{criterion_group, criterion_main, BatchSize, Criterion};

use stride::examiner::Discord::Permissive;
use stride::examiner::{Examiner, Record, Candidate};
use stride::examiner::Outcome::Commit;
use stride::suffix::{Suffix, AppendResult, CompleteResult};
use uuid::Uuid;
use stride::sortedvec::SortedVec;

fn criterion_benchmark(c: &mut Criterion) {
    let (min_extent, max_extent) = (10_000, 20_000);
    let num_items = 1_000;
    let items = (0..num_items)
        .map(|i| format!("item-{}", i))
        .collect::<Vec<_>>();
    let num_combos = num_items;
    let items_per_combo = 1;
    let item_combos = (0..num_combos)
        .map(|i| {
            (0..items_per_combo)
                .map(|j| items[(i + j) % num_items].clone())
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
                readvers: SortedVec::default(),
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
                let result = suffix.append(
                    candidate.rec.readset.clone(),
                    candidate.rec.writeset.clone(),
                    candidate.ver,
                );
                assert_eq!(AppendResult::Appended, result);
                assert_eq!(Some(candidate.ver + 1), suffix.hwm());

                let ver = candidate.ver;
                examiner.learn(candidate);

                assert_eq!(CompleteResult::Completed(ver), suffix.complete(ver));

                if {
                    let truncated = suffix.truncate(min_extent, max_extent);
                    match truncated {
                        None => false,
                        Some(truncated_entries) => {
                            for truncated_entry in truncated_entries {
                                examiner.discard(truncated_entry);
                            }
                            true
                        }
                    }
                } {
                    let range = suffix.range();
                    let span = (range.end - range.start) as usize;
                    assert!(span > 0 && span <= max_extent, "range {:?}", range);
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
                let result = suffix.append(
                    candidate.rec.readset.clone(),
                    candidate.rec.writeset.clone(),
                    candidate.ver,
                );
                assert_eq!(AppendResult::Appended, result);
                assert_eq!(Some(candidate.ver + 1), suffix.hwm());

                let ver = candidate.ver;
                let outcome = examiner.assess(candidate);
                assert_eq!(Commit {safepoint: ver - 1, discord: Permissive}, outcome);

                assert_eq!(CompleteResult::Completed(ver), suffix.complete(ver));

                if {
                    let truncated = suffix.truncate(min_extent, max_extent);
                    match truncated {
                        None => false,
                        Some(truncated_entries) => {
                            for truncated_entry in truncated_entries {
                                examiner.discard(truncated_entry);
                            }
                            true
                        }
                    }
                } {
                    let range = suffix.range();
                    let span = (range.end - range.start) as usize;
                    assert!(span > 0 && span <= max_extent, "range {:?}", range);
                }
            },
            BatchSize::SmallInput,
        );
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
