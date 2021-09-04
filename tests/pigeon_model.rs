mod fixtures;

use stride::*;
use stride::havoc::*;
use uuid::Uuid;
use fixtures::*;

struct State {
    candidates_broker: Broker<Message<Statemap>>,
    decisions_broker: Broker<Message<Statemap>>,
    cohorts: Vec<Cohort>,
    certifier: Certifier,
    assert_product: i32,
}

impl State {
    fn new(num_cohorts: usize, num_items: usize) -> Self {
        let values = (0..num_items)
            .into_iter().map(|i| (i * 10) as i32).collect();
        let candidates_broker = Broker::new();
        let decisions_broker = Broker::new();
        let cohorts = (0..num_cohorts)
            .into_iter().map(|i| Cohort {
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
        let assert_product = values.into_iter().reduce(|a, b| a * b).unwrap();

        State {
            candidates_broker,
            decisions_broker,
            cohorts,
            certifier,
            assert_product
        }
    }

    fn assert() {
        //TODO
    }
}

#[test]
fn test() {
}