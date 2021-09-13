use std::rc::Rc;

use super::fixtures::*;
use stride::havoc::model::ActionResult::{Joined, Ran, Blocked};
use stride::havoc::model::Retention::{Strong, Weak};
use stride::havoc::model::{name_of, Model};
use stride::*;

fn asserter(cohort_index: usize) -> impl Fn(&[Cohort]) -> Box<dyn Fn(&[Cohort]) -> Option<String>> {
    move |before| {
        let &(before_counter_val, _) = &before[cohort_index].replica.items[0];
        let &(before_shadow_val, _) = &before[cohort_index].replica.items[1];
        Box::new(move |after| {
            let replica = &after[cohort_index].replica;
            let &(after_counter_val, _) = &replica.items[0];
            if after_counter_val < before_counter_val {
                // the counter is monotonic; i.e., counter' >= counter
                Some(format!(
                    "after_counter_val ({}) < before_counter_val ({}) for {:?}",
                    after_counter_val, before_counter_val, replica
                ))
            } else {
                let &(after_shadow_val, _) = &replica.items[1];
                if after_shadow_val < before_shadow_val {
                    // the shadow is also monotonic; i.e., shadow' >= shadow
                    Some(format!(
                        "after_shadow_val ({}) < before_shadow_val ({}) for {:?}",
                        after_shadow_val, before_shadow_val, replica
                    ))
                } else if after_counter_val < after_shadow_val {
                    // the shadow trails the counter; i.e., counter' >= shadow'
                    Some(format!(
                        "after_counter_val ({}) < after_shadow_val ({}) for {:?}",
                        after_counter_val, after_shadow_val, replica
                    ))
                } else if before_shadow_val != after_shadow_val && before_counter_val != after_shadow_val {
                    // whenever the shadow is reassigned, it must mimic the counter;
                    // i.e., (shadow /= shadow') => counter = shadow'
                    Some(format!(
                        "after_counter_val ({}) != after_shadow_val ({}) for {:?}",
                        after_counter_val, after_shadow_val, replica
                    ))
                } else {
                    None
                }
            }
        })
    }
}

fn build_model<'a>(
    num_cohorts: usize,
    txns_per_cohort: usize,
    name: &str,
) -> Model<'a, SystemState> {
    // values[0] is the monotonic counter; values[1] is its copy
    let values= vec![0, 0];
    let mut model = Model::new(move || SystemState::new(num_cohorts, &values)).with_name(name.into());

    for cohort_index in 0..num_cohorts {
        let itemset: Vec<String> = (0..2).map(|i| format!("item-{}", i)).collect();
        model.add_action(format!("counter-{}", cohort_index), Weak, move |s, _| {
            let run = s.cohort_txns(cohort_index);
            if run == txns_per_cohort {
                return Blocked;
            }
            let cohort = &mut s.cohorts[cohort_index];
            let readset = vec![itemset[0].clone()];
            let writeset = readset.clone();
            let &(item_val, item_ver) = &cohort.replica.items[0];
            let cpt_readvers = vec![item_ver];
            let cpt_snapshot = cohort.replica.ver;
            let changes = &[(0, item_val + 1)];
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
            if run + 1 == txns_per_cohort {
                Joined
            } else {
                Ran
            }
        });
        let itemset: Vec<String> = (0..2).map(|i| format!("item-{}", i)).collect();
        model.add_action(format!("copier-{}", cohort_index), Weak, move |s, _| {
            let run = s.cohort_txns(cohort_index);
            if run == txns_per_cohort {
                return Blocked;
            }
            let cohort = &mut s.cohorts[cohort_index];
            let readset = vec![itemset[0].clone()];
            let writeset = vec![itemset[1].clone()];
            let &(source_item_val, source_item_ver) = &cohort.replica.items[0];
            let cpt_readvers = vec![source_item_ver];
            let cpt_snapshot = cohort.replica.ver;
            let changes = &[(1, source_item_val)];
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
            if run + 1 == txns_per_cohort {
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
fn dfs_monotonic_1x1() {
    dfs(&build_model(1, 1, name_of(&dfs_monotonic_1x1)));
}

#[test]
fn dfs_monotonic_1x2() {
    dfs(&build_model(1, 2, name_of(&dfs_monotonic_1x2)));
}

#[test]
#[ignore]
fn dfs_monotonic_2x1() {
    dfs(&build_model(2, 1, name_of(&dfs_monotonic_2x1)));
}

#[test]
#[ignore]
fn dfs_monotonic_2x2() {
    dfs(&build_model(2, 2, name_of(&dfs_monotonic_2x2)));
}

#[test]
fn sim_monotonic_1x1() {
    sim(&build_model(1, 1, name_of(&sim_monotonic_1x1)), 10);
}

#[test]
fn sim_monotonic_2x1() {
    sim(&build_model(2, 1, name_of(&sim_monotonic_2x1)), 20);
}

#[test]
fn sim_monotonic_2x2() {
    sim(&build_model(2, 2, name_of(&sim_monotonic_2x2)), 40);
}

#[test]
fn sim_monotonic_3x1() {
    sim(&build_model(3, 1, name_of(&sim_monotonic_3x1)), 40);
}

#[test]
fn sim_monotonic_3x2() {
    sim(&build_model(3, 2, name_of(&sim_monotonic_3x2)), 80);
}

#[test]
fn sim_monotonic_4x1() {
    sim(&build_model(4, 1, name_of(&sim_monotonic_4x1)), 80);
}

#[test]
fn sim_monotonic_4x2() {
    sim(&build_model(4, 2, name_of(&sim_monotonic_4x2)), 160);
}
