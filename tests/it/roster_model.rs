use std::rc::Rc;

use super::fixtures::*;
use stride::havoc::model::ActionResult::{Joined, Ran, Blocked};
use stride::havoc::model::Retention::{Strong, Weak};
use stride::havoc::model::{name_of, Model, rand_element};
use stride::*;

fn asserter(cohort_index: usize) -> impl Fn(&[Cohort]) -> Box<dyn Fn(&[Cohort]) -> Option<String>> {
    move |_| {
        Box::new(move |after| {
            let replica = &after[cohort_index].replica;
            if !replica.items.iter().any(|&(item_val, _)| item_val != 0) {
                Some("blank roster".into())
            } else {
                None
            }
        })
    }
}

fn build_model<'a>(
    num_values: usize,
    num_cohorts: usize,
    txns_per_cohort: usize,
    name: &str,
) -> Model<'a, SystemState> {
    // initial values are alternating 0s and 1s (0 means rostered off, 1 means rostered on)
    let values: Vec<i32> = (0..num_values).map(|i| (i % 2) as i32).collect();
    let mut model = Model::new(move || SystemState::new(num_cohorts, &values)).with_name(name.into());
    let expected_txns = num_cohorts * txns_per_cohort;

    for cohort_index in 0..num_cohorts {
        let itemset: Vec<String> = (0..num_values).map(|i| format!("item-{}", i)).collect();
        // each cohort is assigned a specific item
        let our_item = cohort_index % num_values;
        model.add_action(format!("off-{}", cohort_index), Weak, move |s, c| {
            let run = s.total_txns();
            let cohort = &mut s.cohorts[cohort_index];
            if run == expected_txns {
                return Joined
            }

            let (our_item_val, our_item_ver) = cohort.replica.items[our_item];
            if our_item_val == 0 {
                // don't transact if our item is rostered off
                return Blocked
            }
            
            // find all items other than our item that are rostered on 
            let available_items: Vec<(usize, &(i32, u64))> = cohort.replica.items.iter().enumerate()
                .filter(|&(item, &(item_val, _))| item != our_item && item_val != 0)
                .collect();

            if available_items.is_empty() {
                return Blocked
            }
            
            // pick one such item and transact
            let &(chosen_item, &(_, chosen_item_ver)) = rand_element(c, &available_items);
            let readset = vec![itemset[our_item].clone(), itemset[chosen_item].clone()];
            let writeset = vec![itemset[our_item].clone()];
            let cpt_readvers = vec![our_item_ver, chosen_item_ver];
            let cpt_snapshot = cohort.replica.ver;
            let changes = &[(our_item, 0)];
            let (readvers, snapshot) = Record::compress(cpt_readvers, cpt_snapshot);
            let statemap = Statemap::map(changes, Op::Set);
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
            if run + 1 == expected_txns {
                Joined
            } else {
                Ran
            }
        });

        let itemset: Vec<String> = (0..num_values).map(|i| format!("item-{}", i)).collect();
        let our_item = cohort_index % num_values;
        model.add_action(format!("on-{}", cohort_index), Weak, move |s, _| {
            let run = s.total_txns();
            let cohort = &mut s.cohorts[cohort_index];
            if run == expected_txns {
                return Joined
            }

            let (our_item_val, our_item_ver) = cohort.replica.items[our_item];
            if our_item_val == 1 {
                // don't transact if our item is rostered on
                return Blocked
            }

            let readset = vec![itemset[our_item].clone()];
            let writeset = readset.clone();
            let cpt_readvers = vec![our_item_ver];
            let cpt_snapshot = cohort.replica.ver;
            let changes = &[(our_item, 1)];
            let (readvers, snapshot) = Record::compress(cpt_readvers, cpt_snapshot);
            let statemap = Statemap::map(changes, Op::Set);
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
            if run + 1 == expected_txns {
                Joined
            } else {
                Ran
            }
        });
        model.add_action(format!("updater-{}", cohort_index), Weak, updater_action(cohort_index, asserter(cohort_index)));
        model.add_action(format!("replicator-{}", cohort_index), Weak, replicator_action(cohort_index, asserter(cohort_index)));
    }
    model.add_action("certifier".into(), Weak, certifier_action());
    model.add_action("supervisor".into(), Strong, supervisor_action(num_cohorts * txns_per_cohort));
    model
}

#[test]
fn dfs_roster_1x1() {
    dfs(&build_model(2, 1, 1, name_of(&dfs_roster_1x1)));
}

#[test]
fn dfs_roster_1x2() {
    dfs(&build_model(2, 1, 2, name_of(&dfs_roster_1x2)));
}

#[test]
#[ignore]
fn dfs_roster_2x1() {
    dfs(&build_model(2, 2, 1, name_of(&dfs_roster_2x1)));
}

#[test]
#[ignore]
fn dfs_roster_2x2() {
    dfs(&build_model(2, 2, 2, name_of(&dfs_roster_2x2)));
}

#[test]
fn sim_roster_1x1() {
    sim(&build_model(2, 1, 1, name_of(&sim_roster_1x1)), 10);
}

#[test]
fn sim_roster_2x1() {
    sim(&build_model(2, 2, 1, name_of(&sim_roster_2x1)), 20);
}

#[test]
fn sim_roster_2x2() {
    sim(&build_model(2, 2, 2, name_of(&sim_roster_2x2)), 40);
}

#[test]
fn sim_roster_3x1() {
    sim(&build_model(2, 3, 1, name_of(&sim_roster_3x1)), 40);
}

#[test]
fn sim_roster_3x2() {
    sim(&build_model(2, 3, 2, name_of(&sim_roster_3x2)), 80);
}

#[test]
fn sim_roster_4x1() {
    sim(&build_model(2, 4, 1, name_of(&sim_roster_4x1)), 80);
}

#[test]
fn sim_roster_4x2() {
    sim(&build_model(2, 4, 2, name_of(&sim_roster_4x2)), 160);
}
