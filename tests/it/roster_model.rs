use std::rc::Rc;

use stride::examiner::Record;
use stride::havoc::model::{Model, name_of, rand_element};
use stride::havoc::model::ActionResult::{Blocked, Joined, Ran};
use stride::havoc::model::Retention::{Strong, Weak};

use crate::fixtures::schema::CandidateData;
use crate::fixtures::schema::MessageKind::CandidateMessage;
use crate::harness::{dfs, sim};
use crate::utils::uuidify;

use super::fixtures::*;

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

struct RosterCfg<'a> {
    // allowable values are 0s and 1s (0 means rostered off, 1 means rostered on)
    values: &'a [i32],
    num_cohorts: usize,
    txns_per_cohort: usize,
    extents: &'a [usize],
    name: &'a str
}

fn build_model(cfg: RosterCfg) -> Model<SystemState> {
    let num_cohorts = cfg.num_cohorts;
    let num_certifiers = cfg.extents.len();
    let values = cfg.values;
    let mut model = Model::new(move || SystemState::new(num_cohorts, values, num_certifiers))
        .with_name(cfg.name.into());
    let expected_txns = cfg.num_cohorts * cfg.txns_per_cohort;
    let num_values = values.len();

    for cohort_index in 0..cfg.num_cohorts {
        let itemset = (0..num_values).map(|i| format!("item-{}", i)).collect::<Vec<_>>();
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
            let available_items = cohort.replica.items.iter().enumerate()
                .filter(|&(item, &(item_val, _))| item != our_item && item_val != 0)
                .collect::<Vec<_>>();

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
            cohort.stream.produce(Rc::new(CandidateMessage(CandidateData {
                rec: Record {
                    xid: uuidify(cohort_index, run),
                    readset,
                    writeset,
                    readvers,
                    snapshot,
                },
                statemap,
            })));
            if run + 1 == expected_txns {
                Joined
            } else {
                Ran
            }
        });

        let itemset = (0..num_values).map(|i| format!("item-{}", i)).collect::<Vec<_>>();
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
            cohort.stream.produce(Rc::new(CandidateMessage(CandidateData {
                rec: Record {
                    xid: uuidify(cohort_index, run),
                    readset,
                    writeset,
                    readvers,
                    snapshot,
                },
                statemap,
            })));
            if run + 1 == expected_txns {
                Joined
            } else {
                Ran
            }
        });
        model.add_action(format!("updater-{}", cohort_index), Weak, updater_action(cohort_index, asserter(cohort_index)));
        model.add_action(format!("replicator-{}", cohort_index), Weak, replicator_action(cohort_index, asserter(cohort_index)));
    }
    for (certifier_index, &extent) in cfg.extents.iter().enumerate() {
        model.add_action(
            format!("certifier-{}", certifier_index),
            Weak,
            certifier_action(certifier_index, extent),
        );
    }
    model.add_action("supervisor".into(), Strong, supervisor_action(cfg.num_cohorts * cfg.txns_per_cohort));
    model
}

#[test]
fn dfs_roster_1x1() {
    dfs(&build_model(RosterCfg {
        values: &[0, 1],
        num_cohorts: 1,
        txns_per_cohort: 1,
        extents: &[1],
        name: name_of(&dfs_roster_1x1)
    }));
}

#[test]
fn dfs_roster_1x2() {
    dfs(&build_model(RosterCfg {
        values: &[0, 1],
        num_cohorts: 1,
        txns_per_cohort: 2,
        extents: &[2],
        name: name_of(&dfs_roster_1x2)
    }));
}

#[test]
#[ignore]
fn dfs_roster_2x1() {
    dfs(&build_model(RosterCfg {
        values: &[0, 1],
        num_cohorts: 2,
        txns_per_cohort: 1,
        extents: &[2],
        name: name_of(&dfs_roster_2x1)
    }));
}

#[test]
#[ignore]
fn dfs_roster_2x2() {
    dfs(&build_model(RosterCfg {
        values: &[0, 1],
        num_cohorts: 2,
        txns_per_cohort: 2,
        extents: &[4],
        name: name_of(&dfs_roster_2x2)
    }))
}

#[test]
fn sim_roster_1x1() {
    sim(&build_model(RosterCfg {
        values: &[0, 1],
        num_cohorts: 1,
        txns_per_cohort: 1,
        extents: &[1],
        name: name_of(&sim_roster_1x1)
    }), 10);
}

#[test]
fn sim_roster_2x1() {
    sim(&build_model(RosterCfg {
        values: &[0, 1],
        num_cohorts: 2,
        txns_per_cohort: 1,
        extents: &[2],
        name: name_of(&sim_roster_2x1)
    }), 20);
}

#[test]
fn sim_roster_2x2() {
    sim(&build_model(RosterCfg {
        values: &[0, 1],
        num_cohorts: 2,
        txns_per_cohort: 2,
        extents: &[4],
        name: name_of(&sim_roster_2x2)
    }), 40);
}

#[test]
fn sim_roster_3x1() {
    sim(&build_model(RosterCfg {
        values: &[0, 1, 0],
        num_cohorts: 3,
        txns_per_cohort: 1,
        extents: &[3],
        name: name_of(&sim_roster_3x1)
    }), 40);
}

#[test]
fn sim_roster_3x2() {
    sim(&build_model(RosterCfg {
        values: &[0, 1, 0],
        num_cohorts: 3,
        txns_per_cohort: 2,
        extents: &[6],
        name: name_of(&sim_roster_3x2)
    }), 80);
}

#[test]
fn sim_roster_4x1() {
    sim(&build_model(RosterCfg {
        values: &[0, 1, 0],
        num_cohorts: 4,
        txns_per_cohort: 1,
        extents: &[4],
        name: name_of(&sim_roster_4x1)
    }), 80);
}

#[test]
fn sim_roster_4x2_2x1() {
    sim(&build_model(RosterCfg {
        values: &[1, 1, 1, 1],
        num_cohorts: 4,
        txns_per_cohort: 1,
        extents: &[1, 1],
        name: name_of(&sim_roster_4x2_2x1)
    }), 160);
}

#[test]
fn sim_roster_4x2_2x8() {
    sim(&build_model(RosterCfg {
        values: &[0, 1, 0, 1],
        num_cohorts: 3,
        txns_per_cohort: 2,
        extents: &[8, 8],
        name: name_of(&sim_roster_4x2_2x8)
    }), 160);
}
