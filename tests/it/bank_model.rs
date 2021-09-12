use std::rc::Rc;

use super::fixtures::*;
use stride::havoc::model::ActionResult::{Joined, Ran, Blocked};
use stride::havoc::model::Retention::{Strong, Weak};
use stride::havoc::model::{name_of, Model, rand_element};
use stride::*;

fn asserter(values: &[i32], cohort_index: usize) -> impl Fn(&[Cohort]) -> Box<dyn Fn(&[Cohort]) -> Option<String>> {
    let expected_sum: i32 = values.iter().sum();
    move |_| {
        Box::new(move |after| {
            let replica = &after[cohort_index].replica;
            let mut computed_sum = 0;
            for &(item_val, _) in &replica.items {
                if item_val < 0 {
                    return Some(format!("account negative: {:?}", replica));
                }
                computed_sum += item_val;
            }
            if expected_sum != computed_sum {
                Some(format!("expected: {}, computed: {} for {:?}", expected_sum, computed_sum, replica))
            } else {
                None
            }
        })
    }
}

fn build_model<'a>(
    values: &'a [i32],
    num_cohorts: usize,
    txns_per_cohort: usize,
    name: &str,
) -> Model<'a, SystemState> {
    let mut model = Model::new(move || SystemState::new(num_cohorts, &values)).with_name(name.into());

    for cohort_index in 0..num_cohorts {
        let itemset: Vec<String> = (0..values.len()).map(|i| format!("item-{}", i)).collect();
        model.add_action(format!("initiator-{}", cohort_index), Weak, move |s, c| {
            let run = s.cohort_txns(cohort_index);
            let cohort = &mut s.cohorts[cohort_index];
            // list of 'from' accounts that have sufficient funds to initiate a transfer
            let from_accounts: Vec<(usize, &(i32, u64))> = cohort.replica.items.iter().enumerate()
                .filter(|&(_, &(item_val, _))| item_val > 0)
                .collect();
            if from_accounts.is_empty() {
                return Blocked;
            }

            // pick a 'from' account at random
            let &(from, &(from_val, from_ver)) = rand_element(c, &from_accounts);

            // list of 'to' accounts that excludes the 'from' account
            let to_accounts: Vec<(usize, &(i32, u64))> = cohort.replica.items.iter().enumerate()
                .filter(|&(item, _)| item != from)
                .collect();

            // pick a 'to' account at random
            let &(to, &(to_val, to_ver)) = rand_element(c, &to_accounts);

            // transfer at least half of the value in the 'from' account
            let xfer_amount = (from_val + 1) / 2;

            let readset = vec![itemset[from].clone(), itemset[to].clone()];
            let writeset = readset.clone();
            let cpt_readvers = vec![from_ver, to_ver];
            let cpt_snapshot = cohort.replica.ver;
            let changes = vec![(from, from_val - xfer_amount), (to, to_val + xfer_amount)];
            let (readvers, snapshot) = Record::compress(cpt_readvers, cpt_snapshot);
            let statemap = Statemap::new(changes);
            cohort.candidates.produce(Rc::new(CandidateMessage {
                rec: Record {
                    xid: uuidify(cohort_index, run),
                    readset,
                    writeset,
                    readvers,
                    snapshot,
                },
                statemap,
            }));
            if run + 1 == txns_per_cohort {
                Joined
            } else {
                Ran
            }
        });
        model.add_action(format!("updater-{}", cohort_index), Weak, updater_action(cohort_index, asserter(values, cohort_index)));
        model.add_action(format!("replicator-{}", cohort_index), Weak, replicator_action(cohort_index, asserter(values, cohort_index)));
    }
    model.add_action("certifier".into(), Weak, certifier_action());
    model.add_action("supervisor".into(), Strong, supervisor_action(num_cohorts * txns_per_cohort));
    model
}

#[test]
fn dfs_bank_2x1x1() {
    dfs(&build_model(&[101, 103], 1, 1, name_of(&dfs_bank_2x1x1)));
}

#[test]
fn dfs_bank_2x1x2() {
    dfs(&build_model(&[101, 103], 1, 2, name_of(&dfs_bank_2x1x2)));
}

#[test]
#[ignore]
fn dfs_bank_2x2x1() {
    dfs(&build_model(&[101, 103], 2, 1, name_of(&dfs_bank_2x2x1)));
}

#[test]
#[ignore]
fn dfs_bank_2x2x2() {
    dfs(&build_model(&[101, 103], 2, 2, name_of(&dfs_bank_2x2x2)));
}

#[test]
fn sim_bank_2x1x1() {
    sim(&build_model(&[101, 103], 1, 1, name_of(&sim_bank_2x1x1)), 10);
}

#[test]
fn sim_bank_2x2x1() {
    sim(&build_model(&[101, 103], 2, 1, name_of(&sim_bank_2x2x1)), 20);
}

#[test]
fn sim_bank_2x2x2() {
    sim(&build_model(&[101, 103], 2, 2, name_of(&sim_bank_2x2x2)), 40);
}

#[test]
fn sim_bank_2x3x1() {
    sim(&build_model(&[101, 103], 3, 1, name_of(&sim_bank_2x3x1)), 40);
}

#[test]
fn sim_bank_2x3x2() {
    sim(&build_model(&[101, 103], 3, 2, name_of(&sim_bank_2x3x2)), 80);
}

#[test]
fn sim_bank_3x3x2() {
    sim(&build_model(&[101, 103, 105], 3, 2, name_of(&sim_bank_3x3x2)), 160);
}

#[test]
fn sim_bank_2x4x1() {
    sim(&build_model(&[101, 103], 4, 1, name_of(&sim_bank_2x4x1)), 80);
}

#[test]
fn sim_bank_2x4x2() {
    sim(&build_model(&[101, 103], 4, 2, name_of(&sim_bank_2x4x2)), 160);
}

#[test]
fn sim_bank_3x4x2() {
    sim(&build_model(&[101, 103, 105], 4, 2, name_of(&sim_bank_3x4x2)), 160);
}
