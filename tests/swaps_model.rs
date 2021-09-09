use std::borrow::Borrow;
use std::rc::Rc;

use fixtures::*;
use stride::*;
use stride::havoc::{checker, sim, Sublevel};
use stride::havoc::checker::{Checker, CheckResult};
use stride::havoc::model::{Model, name_of, rand_element};
use stride::havoc::model::ActionResult::{Blocked, Breached, Joined, Ran};
use stride::havoc::model::Retention::{Strong, Weak};
use stride::havoc::sim::{Sim, SimResult};
use std::ops::Div;

mod fixtures;

struct State {
    cohorts: Vec<Cohort>,
    certifier: Certifier,
}

impl State {
    fn new(num_cohorts: usize, values: &[i32]) -> Self {
        let candidates_broker = Broker::new(1);
        let decisions_broker = Broker::new(1);
        let cohorts = (0..num_cohorts)
            .into_iter()
            .map(|_| Cohort {
                pending: vec![],
                replica: Replica::new(&values),
                candidates: candidates_broker.stream(),
                decisions: decisions_broker.stream(),
            })
            .collect();
        let certifier = Certifier {
            examiner: Examiner::new(),
            candidates: candidates_broker.stream(),
            decisions: decisions_broker.stream(),
        };

        State {
            cohorts,
            certifier,
        }
    }

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
}

fn init_log() {
    let _ = env_logger::builder().is_test(true).try_init();
}

fn build_model<'a>(combos: &[(usize, usize)], values: &'a [i32], name: &str) -> Model<'a, State> {
    let num_cohorts = combos.len();
    let expect_txns = num_cohorts;
    let mut model = Model::new(move || State::new(num_cohorts, values)).with_name(name.into());

    for (cohort_index, &(p, q)) in combos.iter().enumerate() {
        let itemset = vec![format!("item-{}", p), format!("item-{}", q)];
        model.add_action(format!("initiator-{}", cohort_index), Weak, move |s, _| {
            let cohort = &s.cohorts[cohort_index];
            let (old_p, old_q) = (cohort.replica.items[p], cohort.replica.items[q]);
            let cpt_readvers = vec![old_p.1, old_q.1];
            let cpt_snapshot = cohort.replica.ver;
            let statemap = Statemap::new(vec![(p, old_q.0), (q, old_p.0)]);
            let (readvers, snapshot) = Record::compress(cpt_readvers, cpt_snapshot);
            cohort.candidates.produce(Rc::new(CandidateMessage {
                rec: Record {
                    xid: uuidify(cohort_index, 0),
                    readset: itemset.clone(),
                    writeset: itemset.clone(),
                    readvers,
                    snapshot,
                },
                statemap,
            }));
            Joined
        });

        let asserter = State::asserter(values);
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
                if let Some(error) = asserter(&cohort.replica) {
                    return Breached(error);
                }
                Ran
            } else {
                Blocked
            }
        });

        let asserter = State::asserter(values);
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
                            if let Some(error) = asserter(&cohort.replica) {
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
fn dfs_swaps_one() {
    dfs_test(&[(0, 1)], &[101, 103], name_of(&dfs_swaps_one));
}

#[test]
#[ignore]
fn dfs_swaps_two() {
    dfs_test(&[(0, 1), (1, 2)], &[101, 103, 107], name_of(&dfs_swaps_two));
}

#[test]
#[ignore]
fn dfs_swaps_three() {
    dfs_test(
        &[(0, 1), (1, 2), (0, 2)],
        &[101, 103, 107],
        name_of(&dfs_swaps_three),
    );
}

fn dfs_test(combos: &[(usize, usize)], values: &[i32], name: &str) {
    init_log();
    let model = build_model(combos, values, name);
    let (result, elapsed) = timed(|| {
        Checker::new(&model)
            .with_config(checker::Config::default().with_sublevel(Sublevel::Fine))
            .check()
    });
    log::debug!("took {:?}", elapsed);
    assert_eq!(CheckResult::Flawless, result);
}

#[test]
fn sim_swaps_one() {
    sim_test(&[(0, 1)], &[101, 103], name_of(&sim_swaps_one), 10);
}

#[test]
fn sim_swaps_two() {
    sim_test(
        &[(0, 1), (1, 2)],
        &[101, 103, 107],
        name_of(&sim_swaps_two),
        100,
    );
}

#[test]
#[ignore]
fn sim_swaps_three() {
    sim_test(
        &[(0, 1), (1, 2), (0, 2)],
        &[101, 103, 107],
        name_of(&sim_swaps_three),
        1_000_000,
    );
}

#[test]
#[ignore]
fn sim_swaps_four() {
    sim_test(
        &[(0, 1), (1, 2), (2, 3)],
        &[101, 103, 107, 111],
        name_of(&sim_swaps_four),
        1_000_000,
    );
}

fn sim_test(combos: &[(usize, usize)], values: &[i32], name: &str, max_schedules: usize) {
    init_log();
    let model = build_model(combos, values, name);
    let seed = seed();
    let max_schedules = max_schedules * scale();
    let sim = Sim::new(&model)
        .with_config(
            sim::Config::default()
                .with_sublevel(Sublevel::Fine)
                .with_max_schedules(max_schedules),
        )
        .with_seed(seed);
    log::debug!("simulating model '{}' with {:?} (seed: {})", model.name().unwrap_or("untitled"), sim.config(), seed);
    let (result, elapsed) = timed(|| sim.check());
    let per_schedule = elapsed.div(max_schedules as u32);
    let rate_s = 1_000_000_000 as f64 / per_schedule.as_nanos() as f64;
    log::debug!("took {:?} ({:?}/schedule, {:.3} schedules/sec)", elapsed, per_schedule, rate_s);
    assert_eq!(SimResult::Pass, result);
}
