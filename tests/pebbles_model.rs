use std::borrow::Borrow;
use std::ops::Div;
use std::rc::Rc;

use fixtures::*;
use stride::*;
use stride::havoc::{checker, sim, Sublevel};
use stride::havoc::checker::{Checker, CheckResult};
use stride::havoc::model::{Model, name_of, rand_element};
use stride::havoc::model::ActionResult::{Blocked, Breached, Joined, Ran};
use stride::havoc::model::Retention::{Strong, Weak};
use stride::havoc::sim::{Sim, SimResult};

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
                run: 0,
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

    fn asserter(num_values: usize) -> impl Fn(&Replica) -> Option<String> {
        move |r| {
            let computed_sum: usize = r.items.iter().map(|(item, _)| *item as usize).sum();
            if computed_sum != 0 && computed_sum != num_values {
                Some(format!(
                    "expected: 0 or {}, computed: {} for {:?}",
                    num_values, computed_sum, r
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

fn build_model<'a>(num_values: usize, num_cohorts: usize, txns_per_cohort: usize, name: &str) -> Model<'a, State> {
    let expect_txns = num_cohorts * txns_per_cohort;
    // initial values are alternating 0s and 1s
    let values: Vec<i32> = (0..num_values).map(|i| (i % 2) as i32).collect();
    let mut model = Model::new(move || State::new(num_cohorts, &values)).with_name(name.into());

    for cohort_index in 0..num_cohorts {
        let itemset: Vec<String> = (0..num_values).map(|i| format!("item-{}", i)).collect();
        // each cohort is assigned a specific 'to' color
        let target_color = (cohort_index % 2) as i32;
        model.add_action(format!("initiator-{}", cohort_index), Weak, move |s, _| {
            let cohort = &mut s.cohorts[cohort_index];
            let readset = itemset.clone();
            let cpt_readvers: Vec<u64> = cohort.replica.items.iter().map(|&(_, item_ver)| item_ver).collect();
            let cpt_snapshot = cohort.replica.ver;
            let changes: Vec<(usize, i32)> = cohort.replica.items.iter().enumerate()
                .filter(|(_, &(item_val, _))| item_val != target_color)
                .map(|(item_index, _)| (item_index, target_color))
                .collect();
            let writeset: Vec<String> = changes.iter().map(|&(item_index, _)| itemset[item_index].clone()).collect();
            let (readvers, snapshot) = Record::compress(cpt_readvers, cpt_snapshot);
            let statemap = Statemap::new(changes);
            cohort.candidates.produce(Rc::new(CandidateMessage {
                rec: Record {
                    xid: uuidify(cohort_index, cohort.run),
                    readset,
                    writeset,
                    readvers,
                    snapshot,
                },
                statemap,
            }));
            cohort.run += 1;
            if cohort.run == txns_per_cohort {
                Joined
            } else {
                Ran
            }
        });

        let asserter = State::asserter(num_values);
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

        let asserter = State::asserter(num_values);
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
                log::trace!("Certified {:?} with {:?}", candidate, outcome);
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
fn dfs_pebbles_1x1() {
    dfs_test(2,
             1,
             1,
             name_of(&dfs_pebbles_1x1));
}

#[test]
fn dfs_pebbles_1x2() {
    dfs_test(2,
             1,
             2,
             name_of(&dfs_pebbles_1x2));
}

#[test]
#[ignore]
fn dfs_pebbles_2x1() {
    dfs_test(2,
             2,
             1,
             name_of(&dfs_pebbles_2x1));
}

#[test]
#[ignore]
fn dfs_pebbles_2x2() {
    dfs_test(2,
             2,
             2,
             name_of(&dfs_pebbles_2x2));
}

fn dfs_test(num_values: usize, num_cohorts: usize, txns_per_cohort: usize, name: &str) {
    init_log();
    let model = build_model(num_values, num_cohorts, txns_per_cohort, name);
    let (result, elapsed) = timed(|| {
        Checker::new(&model)
            .with_config(checker::Config::default().with_sublevel(Sublevel::Fine))
            .check()
    });
    log::debug!("took {:?}", elapsed);
    assert_eq!(CheckResult::Pass, result);
}

#[test]
fn sim_pebbles_1x1() {
    sim_test(2,
             1,
             1,
             name_of(&sim_pebbles_1x1),
             10);
}

#[test]
fn sim_pebbles_2x1() {
    sim_test(
        2,
        2,
        1,
        name_of(&sim_pebbles_2x1),
        100,
    );
}

#[test]
fn sim_pebbles_2x2() {
    sim_test(
        2,
        2,
        2,
        name_of(&sim_pebbles_2x2),
        100,
    );
}

#[test]
#[ignore]
fn sim_pebbles_3x1() {
    sim_test(
        2,
        3,
        1,
        name_of(&sim_pebbles_3x1),
        1_000_000,
    );
}

#[test]
#[ignore]
fn sim_pebbles_3x2() {
    sim_test(
        2,
        3,
        2,
        name_of(&sim_pebbles_3x2),
        1_000_000,
    );
}

#[test]
#[ignore]
fn sim_pebbles_4x1() {
    sim_test(
        2,
        4,
        1,
        name_of(&sim_pebbles_4x1),
        1_000_000,
    );
}

#[test]
#[ignore]
fn sim_pebbles_4x2() {
    sim_test(
        2,
        4,
        2,
        name_of(&sim_pebbles_4x2),
        1_000_000,
    );
}

fn sim_test(num_values: usize, num_cohorts: usize, txns_per_cohort: usize, name: &str, max_schedules: usize) {
    init_log();
    let model = build_model(num_values, num_cohorts, txns_per_cohort, name);
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
    if let SimResult::Fail(fail) = &result {
        let pretty_trace = fail.trace.prettify(&model);
        log::error!("trace:\n{}", pretty_trace);
    }
    assert_eq!(SimResult::Pass, result);
}
