use std::collections::hash_map::Entry;
use std::rc::Rc;

use rustc_hash::FxHashMap;

use stride::*;
use stride::havoc::model::{Model, name_of};
use stride::havoc::model::ActionResult::{Joined, Ran};
use stride::havoc::model::Retention::{Strong, Weak};

use super::fixtures::*;

fn asserter() -> impl Fn(&[Cohort]) -> Box<dyn Fn(&[Cohort]) -> Option<String>> {
    move |_| {
        Box::new(move |after| {
            let mut values_for_version = FxHashMap::default();
            for cohort in after {
                let &(item_val, item_ver) = &cohort.replica.items[0];
                match values_for_version.entry(item_ver) {
                    Entry::Occupied(entry) => {
                        if item_val != *entry.get() {
                            return Some(format!(
                                "mismatch of values at version {:?}, found distinct {:?} and {:?} for {:?}",
                                item_ver, item_val, *entry.get(), after
                            ))
                        }
                    }
                    Entry::Vacant(entry) => {
                        entry.insert(item_val);
                    }
                }
            }
            None
        })
    }
}

fn build_model<'a>(
    num_cohorts: usize,
    txns_per_cohort: usize,
    name: &str,
) -> Model<'a, SystemState> {
    let ops = &[Op::Add(2), Op::Mpy(3)];
    let mut model = Model::new(move || SystemState::new(num_cohorts, &[1])).with_name(name.into());

    for cohort_index in 0..num_cohorts {
        model.add_action(format!("initiator-{}", cohort_index), Weak, move |s, _| {
            let run = s.cohort_txns(cohort_index);
            let cohort = &mut s.cohorts[cohort_index];
            let readset = vec![];
            let writeset = vec!["item-0".into()];
            let cpt_readvers = vec![];
            let cpt_snapshot = cohort.replica.ver;
            let selected_op = ops[run % 2];
            let (readvers, snapshot) = Record::compress(cpt_readvers, cpt_snapshot);
            let statemap = Statemap::new(vec![(0, selected_op)]);
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
        model.add_action(format!("updater-{}", cohort_index), Weak, updater_action(cohort_index, asserter()));
        model.add_action(format!("replicator-{}", cohort_index), Weak, replicator_action(cohort_index, asserter()));
    }
    model.add_action("certifier".into(), Weak, certifier_action());
    model.add_action("supervisor".into(), Strong, supervisor_action(num_cohorts * txns_per_cohort));
    model
}

#[test]
fn dfs_blind_1x1() {
    dfs(&build_model(1, 1, name_of(&dfs_blind_1x1)));
}

#[test]
fn dfs_blind_1x2() {
    dfs(&build_model(1, 2, name_of(&dfs_blind_1x2)));
}

#[test]
#[ignore]
fn dfs_blind_2x1() {
    dfs(&build_model(2, 1, name_of(&dfs_blind_2x1)));
}

#[test]
#[ignore]
fn dfs_blind_2x2() {
    dfs(&build_model(2, 2, name_of(&dfs_blind_2x2)));
}

#[test]
fn sim_blind_1x1() {
    sim(&build_model(1, 1, name_of(&sim_blind_1x1)), 10);
}

#[test]
fn sim_blind_2x1() {
    sim(&build_model(2, 1, name_of(&sim_blind_2x1)), 20);
}

#[test]
fn sim_blind_2x2() {
    sim(&build_model(2, 2, name_of(&sim_blind_2x2)), 40);
}

#[test]
fn sim_blind_3x1() {
    sim(&build_model(3, 1, name_of(&sim_blind_3x1)), 40);
}

#[test]
fn sim_blind_3x2() {
    sim(&build_model(3, 2, name_of(&sim_blind_3x2)), 80);
}

#[test]
fn sim_blind_4x1() {
    sim(&build_model(4, 1, name_of(&sim_blind_4x1)), 80);
}

#[test]
fn sim_blind_4x2() {
    sim(&build_model(4, 2, name_of(&sim_blind_4x2)), 160);
}
