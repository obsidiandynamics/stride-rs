mod fixtures;

use fixtures::*;
use std::rc::Rc;
use stride::havoc::ActionResult::{Blocked, Joined, Ran};
use stride::havoc::CheckResult::Flawless;
use stride::havoc::Retention::Weak;
use stride::havoc::*;
use stride::*;
use uuid::Uuid;
use Retention::Strong;
use std::borrow::Borrow;
use stride::Message::Decision;

struct State {
    candidates_broker: Broker<CandidateMessage<Statemap>>,
    decisions_broker: Broker<DecisionMessage<Statemap>>,
    cohorts: Vec<Cohort>,
    certifier: Certifier,
    expect_product: i32,
}

impl State {
    fn new(num_cohorts: usize, values: Vec<i32>) -> Self {
        let candidates_broker = Broker::new(1);
        let decisions_broker = Broker::new(1);
        let cohorts = (0..num_cohorts)
            .into_iter()
            .map(|i| Cohort {
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
        // let expect_product = values.into_iter().reduce(|a, b| a * b).unwrap();
        let expect_product = values.iter().product();

        State {
            candidates_broker,
            decisions_broker,
            cohorts,
            certifier,
            expect_product,
        }
    }

    fn assert(&self, replica: &Replica) {
        let product = replica.items.iter().map(|(item, _)| *item).product();
        assert_eq!(self.expect_product, product);
    }
}

#[test]
fn swaps_one() {
    let mut model = Model::new(|| State::new(1, vec![5, 7]))
        .with_name(name_of(&swaps_one).into());
    let combos = [(0, 1)];

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
                        snapshot: cpt_snapshot
                    },
                    statemap
                }));
                Joined
            },
        );

        model.action(
            format!("replicator-{}-({}-{})", cohort_index, p, q),
            Weak,
            move |s, _| {
                let cohort = &mut s.cohorts[cohort_index];
                match cohort.decisions.consume() {
                    None => Blocked,
                    Some((_, decision)) => {
                        match decision.outcome {
                            Outcome::Commit(_, _) => {
                                let statemap = decision.statemap.as_ref().expect("no statemap in commit");
                                cohort.replica.install_ser(statemap, decision.candidate.ver);
                            }
                            Outcome::Abort(_, _) => {}
                        }
                        Ran
                    }
                }
            },
        );
    }

    model.action(
        "certifier".into(), Weak,
        |s, _| {
            let certifier = &mut s.certifier;
            match certifier.candidates.consume() {
                None => Blocked,
                Some((offset, candidate_message)) => {
                    let candidate = Candidate {
                        rec: candidate_message.rec.clone(),
                        ver: offset as u64
                    };
                    let outcome = certifier.examiner.assess(&candidate);
                    let statemap = match outcome {
                        Outcome::Commit(_, _) => Some(candidate_message.statemap.clone()),
                        Outcome::Abort(_, _) => None
                    };
                    certifier.decisions.produce(Rc::new(DecisionMessage {
                        candidate,
                        outcome,
                        statemap
                    }));
                    Ran
                }
            }
        }
    );

    model.action(
        "supervisor".into(), Strong,
        |s, _| {
            let finished_cohorts = s.cohorts.iter()
                .filter(|&cohort| cohort.replica.ver == 1)
                .count();
            match finished_cohorts {
                1 => Joined,
                _ => Blocked
            }
        }
    );

    let result = Checker::new(&model).check();
    assert_eq!(Flawless, result);
}
