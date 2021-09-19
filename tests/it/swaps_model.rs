use std::rc::Rc;

use stride::*;
use stride::havoc::model::{Model, name_of};
use stride::havoc::model::ActionResult::{Joined, Ran};
use stride::havoc::model::Retention::{Strong, Weak};

use super::fixtures::*;
use Message::Candidate;

fn asserter(values: &[i32], cohort_index: usize) -> impl Fn(&[Cohort]) -> Box<dyn Fn(&[Cohort]) -> Option<String>> {
    let expected_product: i32 = values.iter().product();
    move |_| {
        Box::new(move |after| {
            let replica = &after[cohort_index].replica;
            let computed_product: i32 = replica.items.iter().map(|&(item_val, _)| item_val).product();
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

fn build_model<'a>(
    combos: &[(usize, usize)],
    values: &'a [i32],
    txns_per_cohort: usize,
    name: &str,
) -> Model<'a, SystemState> {
    let num_cohorts = combos.len();
    let mut model = Model::new(move || SystemState::new(num_cohorts, values)).with_name(name.into());

    for (cohort_index, &(p, q)) in combos.iter().enumerate() {
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
            cohort.stream.produce(Rc::new(Candidate(CandidateMessage {
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
        model.add_action(format!("updater-{}", cohort_index), Weak, updater_action(cohort_index, asserter(values, cohort_index)));
        model.add_action(format!("replicator-{}", cohort_index), Weak, replicator_action(cohort_index, asserter(values, cohort_index)));
    }
    model.add_action("certifier".into(), Weak, certifier_action());
    model.add_action("supervisor".into(), Strong, supervisor_action(num_cohorts * txns_per_cohort));
    model
}

#[test]
fn dfs_swaps_1x1() {
    dfs(&build_model(
        &[(0, 1)],
        &[101, 103],
        1,
        name_of(&dfs_swaps_1x1),
    ));
}

#[test]
fn dfs_swaps_1x2() {
    dfs(&build_model(
        &[(0, 1)],
        &[101, 103],
        2,
        name_of(&dfs_swaps_1x2),
    ));
}

#[test]
#[ignore]
fn dfs_swaps_2x1() {
    dfs(&build_model(
        &[(0, 1), (1, 2)],
        &[101, 103, 107],
        1,
        name_of(&dfs_swaps_2x1),
    ));
}

#[test]
#[ignore]
fn dfs_swaps_2x2() {
    dfs(&build_model(
        &[(0, 1), (1, 2)],
        &[101, 103, 107],
        2,
        name_of(&dfs_swaps_2x2),
    ));
}

#[test]
#[ignore]
fn dfs_swaps_3x1() {
    dfs(&build_model(
        &[(0, 1), (1, 2), (0, 2)],
        &[101, 103, 107],
        1,
        name_of(&dfs_swaps_3x1),
    ));
}

#[test]
fn sim_swaps_1x1() {
    sim(
        &build_model(&[(0, 1)], &[101, 103], 1, name_of(&sim_swaps_1x1)),
        10,
    );
}

#[test]
fn sim_swaps_2x1() {
    sim(
        &build_model(
            &[(0, 1), (1, 2)],
            &[101, 103, 107],
            1,
            name_of(&sim_swaps_2x1),
        ),
        20,
    );
}

#[test]
fn sim_swaps_2x2() {
    sim(
        &build_model(
            &[(0, 1), (1, 2)],
            &[101, 103, 107],
            2,
            name_of(&sim_swaps_2x2),
        ),
        40,
    );
}

#[test]
fn sim_swaps_3x1() {
    sim(
        &build_model(
            &[(0, 1), (1, 2), (0, 2)],
            &[101, 103, 107],
            1,
            name_of(&sim_swaps_3x1),
        ),
        40,
    );
}

#[test]
fn sim_swaps_3x2() {
    sim(
        &build_model(
            &[(0, 1), (1, 2), (0, 2)],
            &[101, 103, 107],
            2,
            name_of(&sim_swaps_3x2),
        ),
        80,
    );
}

#[test]
fn sim_swaps_4x1() {
    sim(
        &build_model(
            &[(0, 1), (1, 2), (2, 3)],
            &[101, 103, 107, 111],
            1,
            name_of(&sim_swaps_4x1),
        ),
        80,
    );
}

#[test]
fn sim_swaps_4x2() {
    sim(
        &build_model(
            &[(0, 1), (1, 2), (2, 3)],
            &[101, 103, 107, 111],
            2,
            name_of(&sim_swaps_4x2),
        ),
        160,
    );
}