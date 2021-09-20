use std::rc::Rc;

use super::fixtures::*;
use stride::havoc::model::ActionResult::{Joined, Ran};
use stride::havoc::model::Retention::{Strong, Weak};
use stride::havoc::model::{name_of, Model};
use stride::*;
use Message::Candidate;
use stride::examiner::Record;

fn asserter(num_values: usize, cohort_index: usize) -> impl Fn(&[Cohort]) -> Box<dyn Fn(&[Cohort]) -> Option<String>> {
    move |_| {
        Box::new(move |after| {
            let replica = &after[cohort_index].replica;
            let computed_sum: usize = replica.items.iter().map(|&(item_val, _)| item_val as usize).sum();
            if computed_sum != 0 && computed_sum != num_values {
                Some(format!(
                    "expected: 0 or {}, computed: {} for {:?}",
                    num_values, computed_sum, replica
                ))
            } else {
                None
            }
        })
    }
}

struct MarblesCfg<'a> {
    num_values: usize,
    num_cohorts: usize,
    txns_per_cohort: usize,
    extent: usize,
    name: &'a str,
}

fn build_model(cfg: MarblesCfg) -> Model<SystemState> {
    // initial values are alternating 0s and 1s
    let values: Vec<i32> = (0..cfg.num_values).map(|i| (i % 2) as i32).collect();
    let num_cohorts = cfg.num_cohorts;
    let mut model = Model::new(move || SystemState::new(num_cohorts, &values)).with_name(cfg.name.into());

    let txns_per_cohort = cfg.txns_per_cohort;
    for cohort_index in 0..cfg.num_cohorts {
        let itemset: Vec<String> = (0..cfg.num_values).map(|i| format!("item-{}", i)).collect();
        // each cohort is assigned a specific 'to' color
        let target_color = (cohort_index % 2) as i32;
        model.add_action(format!("initiator-{}", cohort_index), Weak, move |s, _| {
            let run = s.cohort_txns(cohort_index);
            let cohort = &mut s.cohorts[cohort_index];
            let readset = itemset.clone();
            let cpt_readvers: Vec<u64> = cohort
                .replica
                .items
                .iter()
                .map(|&(_, item_ver)| item_ver)
                .collect();
            let cpt_snapshot = cohort.replica.ver;
            let changes: Vec<(usize, i32)> = cohort
                .replica
                .items
                .iter()
                .enumerate()
                .filter(|(_, &(item_val, _))| item_val != target_color)
                .map(|(item, _)| (item, target_color))
                .collect();
            let writeset: Vec<String> = changes
                .iter()
                .map(|&(item, _)| itemset[item].clone())
                .collect();
            let (readvers, snapshot) = Record::compress(cpt_readvers, cpt_snapshot);
            let statemap = Statemap::map(&changes, Op::Set);
            cohort.stream.produce(Rc::new(Candidate(CandidateMessage {
                rec: Record {
                    xid: uuidify(cohort_index, run),
                    readset,
                    writeset,
                    readvers,
                    snapshot,
                },
                statemap,
            })));
            if run + 1 == txns_per_cohort {
                Joined
            } else {
                Ran
            }
        });
        model.add_action(format!("updater-{}", cohort_index), Weak, updater_action(cohort_index, asserter(cfg.num_values, cohort_index)));
        model.add_action(format!("replicator-{}", cohort_index), Weak, replicator_action(cohort_index, asserter(cfg.num_values, cohort_index)));
    }
    model.add_action("certifier".into(), Weak, certifier_action(cfg.extent));
    model.add_action("supervisor".into(), Strong, supervisor_action(cfg.num_cohorts * cfg.txns_per_cohort));
    model
}

#[test]
fn dfs_marbles_1x1() {
    dfs(&build_model(MarblesCfg {
        num_values: 2,
        num_cohorts: 1,
        txns_per_cohort: 1,
        extent: 1,
        name: name_of(&dfs_marbles_1x1)
    }));
}

#[test]
fn dfs_marbles_1x2() {
    dfs(&build_model(MarblesCfg {
        num_values: 2,
        num_cohorts: 1,
        txns_per_cohort: 2,
        extent: 2,
        name: name_of(&dfs_marbles_1x2)
    }));
}

#[test]
#[ignore]
fn dfs_marbles_2x1() {
    dfs(&build_model(MarblesCfg {
        num_values: 2,
        num_cohorts: 2,
        txns_per_cohort: 1,
        extent: 2,
        name: name_of(&dfs_marbles_2x1)
    }));
}

#[test]
#[ignore]
fn dfs_marbles_2x2() {
    dfs(&build_model(MarblesCfg {
        num_values: 2,
        num_cohorts: 2,
        txns_per_cohort: 2,
        extent: 4,
        name: name_of(&dfs_marbles_2x2)
    }));
}

#[test]
fn sim_marbles_1x1() {
    sim(&build_model(MarblesCfg {
        num_values: 2,
        num_cohorts: 1,
        txns_per_cohort: 1,
        extent: 1,
        name: name_of(&sim_marbles_1x1)
    }), 10);
}

#[test]
fn sim_marbles_2x1() {
    sim(&build_model(MarblesCfg {
        num_values: 2,
        num_cohorts: 2,
        txns_per_cohort: 1,
        extent: 2,
        name: name_of(&sim_marbles_2x1)
    }), 20);
}

#[test]
fn sim_marbles_2x2() {
    sim(&build_model(MarblesCfg {
        num_values: 2,
        num_cohorts: 2,
        txns_per_cohort: 2,
        extent: 4,
        name: name_of(&sim_marbles_2x2)
    }), 40);
}

#[test]
fn sim_marbles_3x1() {
    sim(&build_model(MarblesCfg {
        num_values: 2,
        num_cohorts: 3,
        txns_per_cohort: 1,
        extent: 3,
        name: name_of(&sim_marbles_3x1)
    }), 40);
}

#[test]
fn sim_marbles_3x2() {
    sim(&build_model(MarblesCfg {
        num_values: 2,
        num_cohorts: 3,
        txns_per_cohort: 2,
        extent: 6,
        name: name_of(&sim_marbles_3x2)
    }), 80);
}

#[test]
fn sim_marbles_4x1() {
    sim(&build_model(MarblesCfg {
        num_values: 2,
        num_cohorts: 4,
        txns_per_cohort: 1,
        extent: 4,
        name: name_of(&sim_marbles_4x1)
    }), 80);
}

#[test]
fn sim_marbles_4x2_1() {
    sim(&build_model(MarblesCfg {
        num_values: 2,
        num_cohorts: 4,
        txns_per_cohort: 2,
        extent: 1,
        name: name_of(&sim_marbles_4x2_1),
    }), 160);
}

#[test]
fn sim_marbles_4x2_8() {
    sim(&build_model(MarblesCfg {
        num_values: 2,
        num_cohorts: 4,
        txns_per_cohort: 2,
        extent: 8,
        name: name_of(&sim_marbles_4x2_8),
    }), 160);
}
