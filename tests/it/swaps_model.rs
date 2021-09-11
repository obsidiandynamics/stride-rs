use std::borrow::Borrow;
use std::rc::Rc;

use super::fixtures::*;
use stride::havoc::model::ActionResult::{Blocked, Breached, Joined, Ran};
use stride::havoc::model::Retention::{Strong, Weak};
use stride::havoc::model::{name_of, rand_element, Model};
use stride::*;

fn asserter(values: &[i32]) -> impl Fn(&Replica) -> Option<String> {
    let expected_product: i32 = values.iter().product();
    move |r| {
        let computed_product: i32 = r.items.iter().map(|(item, _)| *item).product();
        if expected_product != computed_product {
            Some(format!(
                "expected: {}, computed: {} for {:?}",
                expected_product, computed_product, r
            ))
        } else {
            None
        }
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
            let statemap = Statemap::new(vec![(p, old_q_val), (q, old_p_val)]);
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

        let _asserter = asserter(values);
        model.add_action(format!("updater-{}", cohort_index), Weak, move |s, c| {
            let cohort = &mut s.cohorts[cohort_index];
            let installable_commits = cohort.decisions.find(|decision| match decision {
                DecisionMessage::Commit(commit) => cohort.replica.can_install_ooo(
                    &commit.statemap,
                    commit.safepoint,
                    commit.candidate.ver,
                ),
                DecisionMessage::Abort(_) => false,
            });

            if !installable_commits.is_empty() {
                log::trace!("Installable {:?}", installable_commits);
                let (_, commit) = rand_element(c, &installable_commits);
                let commit = commit.as_commit().unwrap();
                cohort.replica.install_ooo(
                    &commit.statemap,
                    commit.safepoint,
                    commit.candidate.ver,
                );
                if let Some(error) = _asserter(&cohort.replica) {
                    return Breached(error);
                }
                Ran
            } else {
                Blocked
            }
        });

        let _asserter = asserter(values);
        model.add_action(format!("replicator-{}", cohort_index), Weak, move |s, _| {
            let cohort = &mut s.cohorts[cohort_index];
            match cohort.decisions.consume() {
                None => Blocked,
                Some((_, decision)) => {
                    match decision.borrow() {
                        DecisionMessage::Commit(commit) => {
                            cohort
                                .replica
                                .install_ser(&commit.statemap, commit.candidate.ver);
                            if let Some(error) = _asserter(&cohort.replica) {
                                return Breached(error);
                            }
                        }
                        DecisionMessage::Abort(abort) => {
                            log::trace!("ABORTED {:?}", abort.reason);
                        }
                    }
                    Ran
                }
            }
        });
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
        100,
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
        100,
    );
}

#[test]
#[ignore]
fn sim_swaps_3x1() {
    sim(
        &build_model(
            &[(0, 1), (1, 2), (0, 2)],
            &[101, 103, 107],
            1,
            name_of(&sim_swaps_3x1),
        ),
        1_000_000,
    );
}

#[test]
#[ignore]
fn sim_swaps_3x2() {
    sim(
        &build_model(
            &[(0, 1), (1, 2), (0, 2)],
            &[101, 103, 107],
            2,
            name_of(&sim_swaps_3x2),
        ),
        1_000_000,
    );
}

#[test]
#[ignore]
fn sim_swaps_4x1() {
    sim(
        &build_model(
            &[(0, 1), (1, 2), (2, 3)],
            &[101, 103, 107, 111],
            1,
            name_of(&sim_swaps_4x1),
        ),
        1_000_000,
    );
}

#[test]
#[ignore]
fn sim_swaps_4x2() {
    sim(
        &build_model(
            &[(0, 1), (1, 2), (2, 3)],
            &[101, 103, 107, 111],
            2,
            name_of(&sim_swaps_4x2),
        ),
        1_000_000,
    );
}