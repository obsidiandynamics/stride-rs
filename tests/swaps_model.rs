use std::rc::Rc;
use std::time::{Duration, SystemTime};

use fixtures::*;
use stride::havoc::checker::{CheckResult, Checker};
use stride::havoc::model::ActionResult::{Blocked, Joined, Ran, Breached};
use stride::havoc::model::Retention::{Strong, Weak};
use stride::havoc::model::{name_of, Model, rand_element};
use stride::havoc::sim::{Sim, SimResult};
use stride::havoc::{checker, sim, Sublevel};
use stride::*;

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
            // candidates_broker,
            // decisions_broker,
            cohorts,
            certifier,
        }
    }

    fn asserter(values: &[i32]) -> impl Fn(&Replica) -> Option<String> {
        let expected_product: i32 = values.iter().product();
        move |r| {
            let computed_product: i32 = r.items.iter().map(|(item, _)| *item).product();
            if expected_product != computed_product {
                Some(format!("expected: {}, computed: {} for {:?}", expected_product, computed_product, r))
            } else {
                None
            }
        }
    }
}

fn init_log() {
    let _ = env_logger::builder().is_test(true).try_init();
}

#[test]
fn dfs_swaps_one() {
    dfs_test(&[(0, 1)], &[101, 103], name_of(&dfs_swaps_one));
}

#[test]
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

fn build_model<'a>(
    combos: &[(usize, usize)],
    values: &'a [i32],
    name: &str,
) -> Model<'a, State> {
    let num_cohorts = combos.len();
    let expect_txns = num_cohorts;
    let mut model = Model::new(move || State::new(num_cohorts, values)).with_name(name.into());

    for (cohort_index, &(p, q)) in combos.iter().enumerate() {
        let itemset = vec![format!("item-{}", p), format!("item-{}", q)];
        model.action(
            format!("initiator-{}-({}-{})", cohort_index, p, q),
            Weak,
            move |s, _| {
                let cohort = &s.cohorts[cohort_index];
                let (old_p, old_q) = (cohort.replica.items[p], cohort.replica.items[q]);
                let cpt_readvers = vec![old_p.1, old_q.1];
                let cpt_snapshot = cohort.replica.ver;
                let statemap = Statemap::new(vec![(p, old_q.0), (q, old_p.0)]);
                cohort.candidates.produce(Rc::new(CandidateMessage {
                    rec: Record {
                        xid: uuidify(cohort_index, 0),
                        readset: itemset.clone(),
                        writeset: itemset.clone(),
                        readvers: cpt_readvers,
                        snapshot: cpt_snapshot,
                    },
                    statemap,
                }));
                Joined
            },
        );

        let asserter = State::asserter(values);
        model.action(
            format!("updater-{}", cohort_index),
            Weak,
            move |s, c| {
                let cohort = &mut s.cohorts[cohort_index];
                let installable_commits = cohort.decisions.find(|decision| {
                    match decision.outcome {
                        Outcome::Commit(safepoint, _) => {
                            let statemap = decision.statemap.as_ref().unwrap();
                            cohort.replica.can_install_ooo(statemap, safepoint, decision.candidate.ver)
                        }
                        Outcome::Abort(_, _) => false
                    }
                });

                if ! installable_commits.is_empty() {
                    println!("Installable {:?}", installable_commits);
                    let (_, commit) = rand_element(c, &installable_commits);
                    match commit.outcome {
                        Outcome::Commit(safepoint, _) => {
                            let statemap = commit.statemap.as_ref().unwrap();
                            cohort.replica.install_ooo(statemap, safepoint, commit.candidate.ver);
                            if let Some(error) = asserter(&cohort.replica) {
                                return Breached(error);
                            }
                        }
                        Outcome::Abort(_, _) => unreachable!()
                    }
                    Ran
                } else {
                    Blocked
                }
            },
        );

        let asserter = State::asserter(values);
        model.action(
            format!("replicator-{}", cohort_index),
            Weak,
            move |s, _| {
                let cohort = &mut s.cohorts[cohort_index];
                match cohort.decisions.consume() {
                    None => Blocked,
                    Some((_, decision)) => {
                        match &decision.outcome {
                            Outcome::Commit(_, _) => {
                                let statemap =
                                    decision.statemap.as_ref().expect("no statemap in commit");
                                cohort.replica.install_ser(statemap, decision.candidate.ver);
                                if let Some(error) = asserter(&cohort.replica) {
                                    return Breached(error);
                                }
                            }
                            Outcome::Abort(reason, _) => {
                                log::trace!("ABORTED {:?}", reason);
                            }
                        }
                        Ran
                    }
                }
            },
        );
    }

    model.action("certifier".into(), Weak, |s, _| {
        let certifier = &mut s.certifier;
        match certifier.candidates.consume() {
            None => Blocked,
            Some((offset, candidate_message)) => {
                let candidate = Candidate {
                    rec: candidate_message.rec.clone(),
                    ver: offset as u64,
                };
                let outcome = certifier.examiner.assess(&candidate);
                // log::trace!("OUTCOME {:?}", outcome);
                let statemap = match outcome {
                    Outcome::Commit(_, _) => Some(candidate_message.statemap.clone()),
                    Outcome::Abort(_, _) => None,
                };
                certifier.decisions.produce(Rc::new(DecisionMessage {
                    candidate,
                    outcome,
                    statemap,
                }));
                Ran
            }
        }
    });

    model.action("supervisor".into(), Strong, move |s, _| {
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
    sim_test(&[(0, 1), (1, 2)], &[101, 103, 107], name_of(&sim_swaps_two), 100);
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

fn timed<F, R>(f: F) -> (R, Duration)
where
    F: Fn() -> R,
{
    let start = SystemTime::now();
    (f(), SystemTime::now().duration_since(start).unwrap_or(Duration::new(0, 0)))
}

fn sim_test(combos: &[(usize, usize)], values: &[i32], name: &str, max_schedules: usize) {
    init_log();
    let model = build_model(combos, values, name);
    let (result, elapsed) = timed(|| {
        Sim::new(&model)
            .with_config(
                sim::Config::default()
                    .with_sublevel(Sublevel::Fine)
                    .with_max_schedules(max_schedules),
            )
            .check()
    });
    log::debug!("took {:?}", elapsed);
    assert_eq!(SimResult::Pass, result);
}
