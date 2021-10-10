use criterion::{criterion_group, criterion_main, Criterion, black_box};
use stride::sortedvec::SortedVec;
use rustc_hash::FxHashSet;
use std::iter::FromIterator;

struct Cycle(u64, u64);

impl Cycle {
    fn next(&mut self) -> u64 {
        if self.1 == 0 {
            0
        } else {
            let next = self.0 + 1;
            self.0 = if next == self.1 { 0 } else { next };
            self.0
            // self.0 += 1;
            // if self.0 == self.1 {
            //     self.0 = 0;
            // }
            // self.0
        }
    }
}

fn criterion_benchmark(c: &mut Criterion) {
    for &num_items in &[0, 1, 2, 4, 8, 16, 64, 256] {
        let vec = (0..num_items).collect::<Vec<u64>>();
        let expect_contain = if num_items == 0 { false } else { true };
        c.bench_function(&format!("sortedvec_contains_{}", num_items), |b| {
            let sortedvec: SortedVec<_> = vec.clone().into();
            let mut item = Cycle(0, num_items);
            b.iter(|| {
                assert_eq!(expect_contain, sortedvec.contains(black_box(&item.next())));
                // item += 1;
                // if item == num_items {
                //     item = 0;
                // }
            });
        });

        c.bench_function(&format!("stdvec_contains_{}", num_items), |b| {
            let mut item = Cycle(0, num_items);
            b.iter(|| {
                assert_eq!(expect_contain, vec.contains(black_box(&item.next())));
                // item += 1;
                // if item == num_items {
                //     item = 0;
                // }
            });
        });

        c.bench_function(&format!("hashset_contains_{}", num_items), |b| {
            let set = FxHashSet::from_iter(vec.clone());
            let mut item = Cycle(0, num_items);
            b.iter(|| {
                assert_eq!(expect_contain, set.contains(black_box(&item.next())));
                // item += 1;
                // if item == num_items {
                //     item = 0;
                // }
            });
        });
    }
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
