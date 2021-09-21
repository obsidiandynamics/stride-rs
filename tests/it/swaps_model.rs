use std::rc::Rc;

use stride::havoc::model::ActionResult::{Joined, Ran};
use stride::havoc::model::Retention::{Strong, Weak};
use stride::havoc::model::{name_of, Model};
use stride::*;

use super::fixtures::*;
use stride::examiner::Record;
use MessageKind::CandidateMessage;

fn asserter(
    values: &[i32],
    cohort_index: usize,
) -> impl Fn(&[Cohort]) -> Box<dyn Fn(&[Cohort]) -> Option<String>> {
    let expected_product = values.iter().product::<i32>();
    move |_| {
        Box::new(move |after| {
            let replica = &after[cohort_index].replica;
            let computed_product: i32 = replica
                .items
                .iter()
                .map(|&(item_val, _)| item_val)
                .product();
            if expected_product != computed_product {
                Some(format!(
                    "expected: {}, computed: {} for {:?}",
                    expected_product, computed_product, replica
                ))
            } else {
                None
            }
        })
    }
}

struct SwapsCfg<'a> {
    values: &'a [i32],
    combos: &'a [(usize, usize)],
    txns_per_cohort: usize,
    extents: &'a [usize],
    name: &'a str,
}

fn build_model(cfg: SwapsCfg) -> Model<SystemState> {
    let num_cohorts = cfg.combos.len();
    let num_certifiers = cfg.extents.len();
    let values = cfg.values;
    let mut model = Model::new(move || SystemState::new(num_cohorts, values, num_certifiers))
        .with_name(cfg.name.into());

    let txns_per_cohort = cfg.txns_per_cohort;
    for (cohort_index, &(p, q)) in cfg.combos.iter().enumerate() {
        let itemset = [format!("item-{}", p), format!("item-{}", q)];
        model.add_action(format!("initiator-{}", cohort_index), Weak, move |s, _| {
            let run = s.cohort_txns(cohort_index);
            let cohort = &mut s.cohorts[cohort_index];
            let ((old_p_val, old_p_ver), (old_q_val, old_q_ver)) =
                (cohort.replica.items[p], cohort.replica.items[q]);
            let cpt_readvers = vec![old_p_ver, old_q_ver];
            let cpt_snapshot = cohort.replica.ver;
            let statemap = Statemap::map(&[(p, old_q_val), (q, old_p_val)], Op::Set);
            let (readvers, snapshot) = Record::compress(cpt_readvers, cpt_snapshot);
            cohort
                .stream
                .produce(Rc::new(CandidateMessage(CandidateData {
                    rec: Record {
                        xid: uuidify(cohort_index, run),
                        readset: itemset.to_vec(),
                        writeset: itemset.to_vec(),
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
        model.add_action(
            format!("updater-{}", cohort_index),
            Weak,
            updater_action(cohort_index, asserter(values, cohort_index)),
        );
        model.add_action(
            format!("replicator-{}", cohort_index),
            Weak,
            replicator_action(cohort_index, asserter(values, cohort_index)),
        );
    }
    for (certifier_index, &extent) in cfg.extents.iter().enumerate() {
        model.add_action(
            format!("certifier-{}", certifier_index),
            Weak,
            certifier_action(certifier_index, extent),
        );
    }
    model.add_action(
        "supervisor".into(),
        Strong,
        supervisor_action(num_cohorts * cfg.txns_per_cohort),
    );
    model
}

#[test]
fn dfs_swaps_1x1() {
    dfs(&build_model(SwapsCfg {
        values: &[101, 103],
        combos: &[(0, 1)],
        txns_per_cohort: 1,
        extents: &[1],
        name: name_of(&dfs_swaps_1x1),
    }));
}

#[test]
fn dfs_swaps_1x2() {
    dfs(&build_model(SwapsCfg {
        values: &[101, 103],
        combos: &[(0, 1)],
        txns_per_cohort: 2,
        extents: &[2],
        name: name_of(&dfs_swaps_1x2),
    }));
}

#[test]
#[ignore]
fn dfs_swaps_2x1() {
    dfs(&build_model(SwapsCfg {
        values: &[101, 103, 107],
        combos: &[(0, 1), (1, 2)],
        txns_per_cohort: 1,
        extents: &[2],
        name: name_of(&dfs_swaps_2x1),
    }));
}

#[test]
#[ignore]
fn dfs_swaps_2x2() {
    dfs(&build_model(SwapsCfg {
        values: &[101, 103, 107],
        combos: &[(0, 1), (1, 2)],
        txns_per_cohort: 2,
        extents: &[4],
        name: name_of(&dfs_swaps_2x2),
    }));
}

#[test]
#[ignore]
fn dfs_swaps_3x1() {
    dfs(&build_model(SwapsCfg {
        values: &[101, 103, 107],
        combos: &[(0, 1), (1, 2), (0, 2)],
        txns_per_cohort: 1,
        extents: &[3],
        name: name_of(&dfs_swaps_3x1),
    }));
}

#[test]
fn sim_swaps_1x1() {
    sim(
        &build_model(SwapsCfg {
            values: &[101, 103],
            combos: &[(0, 1)],
            txns_per_cohort: 1,
            extents: &[1],
            name: name_of(&sim_swaps_1x1),
        }),
        10,
    );
}

#[test]
fn sim_swaps_2x1() {
    sim(
        &build_model(SwapsCfg {
            values: &[101, 103, 107],
            combos: &[(0, 1), (1, 2)],
            txns_per_cohort: 1,
            extents: &[2],
            name: name_of(&sim_swaps_2x1),
        }),
        20,
    );
}

#[test]
fn sim_swaps_2x2() {
    sim(
        &build_model(SwapsCfg {
            values: &[101, 103, 107],
            combos: &[(0, 1), (1, 2)],
            txns_per_cohort: 2,
            extents: &[4],
            name: name_of(&sim_swaps_2x2),
        }),
        40,
    );
}

#[test]
fn sim_swaps_3x1() {
    sim(
        &build_model(SwapsCfg {
            values: &[101, 103, 107],
            combos: &[(0, 1), (1, 2), (0, 2)],
            txns_per_cohort: 1,
            extents: &[3],
            name: name_of(&sim_swaps_3x1),
        }),
        40,
    );
}

#[test]
fn sim_swaps_3x2() {
    sim(
        &build_model(SwapsCfg {
            values: &[101, 103, 107],
            combos: &[(0, 1), (1, 2), (0, 2)],
            txns_per_cohort: 2,
            extents: &[6],
            name: name_of(&sim_swaps_3x2),
        }),
        80,
    );
}

#[test]
fn sim_swaps_4x1() {
    sim(
        &build_model(SwapsCfg {
            values: &[101, 103, 107, 111],
            combos: &[(0, 1), (1, 2), (2, 3)],
            txns_per_cohort: 1,
            extents: &[4],
            name: name_of(&sim_swaps_4x1),
        }),
        80,
    );
}

#[test]
fn sim_swaps_4x2_2x1() {
    sim(
        &build_model(SwapsCfg {
            values: &[101, 103, 107, 111],
            combos: &[(0, 1), (1, 2), (2, 3)],
            txns_per_cohort: 2,
            extents: &[1, 1],
            name: name_of(&sim_swaps_4x2_2x1),
        }),
        160,
    );
}

#[test]
fn sim_swaps_4x2_asymmetric() {
    sim(
        &build_model(SwapsCfg {
            values: &[101, 103, 107, 111],
            combos: &[(0, 1), (1, 2), (2, 3)],
            txns_per_cohort: 2,
            extents: &[1, 2],
            name: name_of(&sim_swaps_4x2_asymmetric),
        }),
        160,
    );
}

#[test]
fn sim_swaps_4x2_2x8() {
    sim(
        &build_model(SwapsCfg {
            values: &[101, 103, 107, 111],
            combos: &[(0, 1), (1, 2), (2, 3)],
            txns_per_cohort: 2,
            extents: &[8, 8],
            name: name_of(&sim_swaps_4x2_2x8),
        }),
        160,
    );
}
