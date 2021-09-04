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

struct State {
    candidates_broker: Broker<CandidateMessage<Statemap>>,
    decisions_broker: Broker<DecisionMessage<Statemap>>,
    cohorts: Vec<Cohort>,
    certifier: Certifier,
    expect_product: i32,
}

impl State {
    fn new(num_cohorts: usize, values: Vec<i32>) -> Self {
        let candidates_broker = Broker::new();
        let decisions_broker = Broker::new();
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
fn one() {
    let mut model = Model::new(|| State::new(1, vec![5, 7]));
    const COMBOS: [(usize, usize); 1] = [(0, 1)];

    // model.push("receiver_0".into(), Strong, |s, _| {
    //     Ran
    // });
    for (cohort_index, (p, q)) in COMBOS.iter().enumerate() {
        let itemset = vec![format!("item-{}", p), format!("item-{}", q)];
        model.push(
            format!("initiator-{}-({}-{})", cohort_index, p, q),
            Weak,
            move |s, _| {
                let cohort = &s.cohorts[cohort_index];
                let (old_p, old_q) = (cohort.replica.items[*p], cohort.replica.items[*q]);
                let cpt_readvers = vec![old_p.1, old_q.1];
                let cpt_snapshot = cohort.replica.ver;
                let statemap = Statemap::new(vec![(*p, old_q.0), (*q, old_p.0)]);
                cohort.candidates.produce(Rc::new(CandidateMessage {
                    transaction: Candidate {
                        xid: Default::default(),
                        ver: 0,
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

        model.push(
            format!("replicator-{}-({}-{})", cohort_index, p, q),
            Weak,
            move |s, _| {
                let cohort = &mut s.cohorts[cohort_index];
                match cohort.decisions.consume() {
                    None => Blocked,
                    Some(decision) => {
                        cohort.replica.install_ser(&decision.statemap, decision.transaction.ver);
                        Ran
                    }
                }
            },
        );
    }

    let result = Checker::new(&model).check();
    assert_eq!(Flawless, result);
}
