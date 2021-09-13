use std::rc::Rc;

use stride::*;
use stride::havoc::model::{Model, name_of};
use stride::havoc::model::ActionResult::{Blocked, Joined, Ran};
use stride::havoc::model::Retention::{Strong, Weak};

use super::fixtures::*;

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
    let expect_txns = num_cohorts * txns_per_cohort;
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
            let statemap = Statemap::set(vec![(p, old_q_val), (q, old_p_val)]);
            let (readvers, snapshot) = Record::compress(cpt_readvers, cpt_snapshot);
            cohort.candidates.produce(Rc::new(CandidateMessage {
                rec: Record {
                    xid: uuidify(cohort_index, run),
                    readset: itemset.to_vec(),
                    writeset: itemset.to_vec(),
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

    model.add_action("certifier".into(), Weak, |s, _| {
        let certifier = &mut s.certifier;
        match certifier.candidates.consume() {
            None => Blocked,
            Some((offset, candidate_message)) => {
                let candidate = Candidate {
                    rec: candidate_message.rec.clone(),
                    ver: offset as u64,
                };
                let outcome = certifier.examiner.assess(&candidate);
                let decision_message = match outcome {
                    Outcome::Commit(safepoint, _) => DecisionMessage::Commit(CommitMessage {
                        candidate,
                        safepoint,
                        statemap: candidate_message.statemap.clone(),
                    }),
                    Outcome::Abort(reason, _) => {
                        DecisionMessage::Abort(AbortMessage { candidate, reason })
                    }
                };
                certifier.decisions.produce(Rc::new(decision_message));
                Ran
            }
        }
    });

    model.add_action("supervisor".into(), Strong, move |s, _| {
        let finished_cohorts = s
            .cohorts
            .iter()
            .filter(|&cohort| cohort.decisions.offset() == expect_txns + 1)
            .count();
        if finished_cohorts == num_cohorts {
            Joined
        } else {
            Blocked
        }
    });

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