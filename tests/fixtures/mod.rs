use std::cell::RefCell;
use std::ops::Deref;
use std::rc::Rc;

use uuid::Uuid;

use stride::{CandidateMessage, DecisionMessage, Examiner};

#[derive(Debug, Clone)]
pub struct Statemap {
    changes: Vec<(usize, i32)>,
}

impl Statemap {
    pub fn new(changes: Vec<(usize, i32)>) -> Self {
        Statemap { changes }
    }
}

#[derive(Debug)]
pub struct Cohort {
    pub pending: Vec<Uuid>,
    pub replica: Replica,
    pub candidates: Stream<CandidateMessage<Statemap>>,
    pub decisions: Stream<DecisionMessage<Statemap>>,
}

#[derive(Debug)]
pub struct Certifier {
    pub examiner: Examiner,
    pub candidates: Stream<CandidateMessage<Statemap>>,
    pub decisions: Stream<DecisionMessage<Statemap>>,
}

#[derive(Debug)]
pub struct Replica {
    pub items: Vec<(i32, u64)>,
    pub ver: u64,
}

impl Replica {
    pub fn new(values: &[i32]) -> Self {
        Replica {
            items: values.iter().map(|&i| (i, 0)).collect(),
            ver: 0,
        }
    }

    fn install_items(&mut self, statemap: &Statemap, ver: u64) {
        for &(change_item, change_value) in &statemap.changes {
            let existing = &mut self.items[change_item];
            if ver > existing.1 {
                *existing = (change_value, ver);
            }
        }
    }

    pub fn install_ooo(&mut self, statemap: &Statemap, safepoint: u64, ver: u64) {
        if self.ver >= safepoint && ver > self.ver {
            self.install_items(statemap, ver);
        }
    }

    pub fn install_ser(&mut self, statemap: &Statemap, ver: u64) {
        if ver > self.ver {
            self.install_items(statemap, ver);
            self.ver = ver;
        }
    }
}

#[derive(Debug)]
pub struct Broker<M> {
    internals: Rc<RefCell<BrokerInternals<M>>>
}

#[derive(Debug)]
struct BrokerInternals<M> {
    messages: Vec<Rc<M>>,
    base: usize,
}

impl<M> Broker<M> {
    pub fn new(base: usize) -> Self {
        Broker {
            internals: Rc::new(RefCell::new(BrokerInternals {
                messages: vec![],
                base
            }))
        }
    }

    pub fn stream(&self) -> Stream<M> {
        let internals = Rc::clone(&self.internals);
        let offset = internals.borrow().base;
        Stream { internals, offset }
    }
}

#[derive(Debug)]
pub struct Stream<M> {
    internals: Rc<RefCell<BrokerInternals<M>>>,
    offset: usize,
}

impl<M> Stream<M> {
    pub fn produce(&self, message: Rc<M>) {
        self.internals.borrow_mut().messages.push(Rc::clone(&message));
    }

    pub fn consume(&mut self) -> Option<(usize, Rc<M>)> {
        let internals = self.internals.borrow();
        let offset = self.offset;
        match internals.messages.get(offset - internals.base) {
            None => None,
            Some(message) => {
                self.offset += 1;
                Some((offset, Rc::clone(message)))
            }
        }
    }

    pub fn find<P>(&self, predicate: P) -> Vec<(usize, Rc<M>)>
    where
        P: Fn(&M) -> bool,
    {
        let internals = &self.internals.borrow();
        let messages = &internals.messages;
        let base = internals.base;
        messages
            .iter().enumerate()
            .filter(|&(_, m)| predicate(m.deref()))
            .map(|(i, m)| (i + base, Rc::clone(&m)))
            .collect()
    }

    pub fn offset(&self) -> usize {
        self.offset
    }
}

pub fn uuidify(pid: usize, run: usize) -> Uuid {
    Uuid::from_u128((pid as u128) << 64 | run as u128)
}

#[test]
fn replica_install_items() {
    let mut replica = Replica {
        items: vec![(10, 5), (20, 5), (30, 5)],
        ver: 5,
    };

    // empty statemap at a newer version -- no change expected
    replica.install_items(&Statemap::new(vec![]), 6);
    assert_eq!(vec![(10, 5), (20, 5), (30, 5)], replica.items);
    assert_eq!(5, replica.ver);

    // non-empty statemap at the same version -- no change expected
    replica.install_items(&Statemap::new(vec![(0, 11)]), 5);
    assert_eq!(vec![(10, 5), (20, 5), (30, 5)], replica.items);
    assert_eq!(5, replica.ver);

    // non-empty statemap at a higher version -- expect changes
    replica.install_items(&Statemap::new(vec![(0, 11)]), 6);
    assert_eq!(vec![(11, 6), (20, 5), (30, 5)], replica.items);
    assert_eq!(5, replica.ver);
}

#[test]
fn replica_install_ooo() {
    let mut replica = Replica {
        items: vec![(10, 5), (20, 5), (30, 5)],
        ver: 5,
    };

    // non-empty statemap at the same safepoint and same version -- no change expected
    replica.install_ooo(&Statemap::new(vec![(0, 11)]), 5, 5);
    assert_eq!(vec![(10, 5), (20, 5), (30, 5)], replica.items);
    assert_eq!(5, replica.ver);

    // non-empty statemap at the greater safepoint and greater version -- no change expected
    replica.install_ooo(&Statemap::new(vec![(0, 11)]), 6, 6);
    assert_eq!(vec![(10, 5), (20, 5), (30, 5)], replica.items);
    assert_eq!(5, replica.ver);

    // non-empty statemap at the same safepoint and greater version -- expect changes
    replica.install_ooo(&Statemap::new(vec![(0, 11)]), 5, 6);
    assert_eq!(vec![(11, 6), (20, 5), (30, 5)], replica.items);
    assert_eq!(5, replica.ver);
}

#[test]
fn replica_install_ser() {
    let mut replica = Replica {
        items: vec![(10, 5), (20, 5), (30, 5)],
        ver: 5,
    };

    // non-empty statemap at the same version -- no change expected
    replica.install_ser(&Statemap::new(vec![(0, 11)]), 5);
    assert_eq!(vec![(10, 5), (20, 5), (30, 5)], replica.items);
    assert_eq!(5, replica.ver);

    // non-empty statemap at a higher version -- expect changes
    replica.install_ser(&Statemap::new(vec![(0, 11)]), 6);
    assert_eq!(vec![(11, 6), (20, 5), (30, 5)], replica.items);
    assert_eq!(6, replica.ver);
}

#[test]
fn stream_produce_consume() {
    let broker = Broker::new(0);
    let mut s0 = broker.stream();
    assert_eq!(0, s0.offset);
    assert_eq!(None, s0.consume());
    assert_eq!(
        Vec::<(usize, Rc<&str>)>::new(),
        s0.find(|i| String::from(*i).contains("s"))
    );

    s0.produce(Rc::new("first"));
    assert_eq!(Some((0, Rc::new("first"))), s0.consume());
    assert_eq!(
        vec![(0, Rc::new("first"))],
        s0.find(|i| String::from(*i).contains("s"))
    );

    s0.produce(Rc::new("second"));
    assert_eq!(Some((1, Rc::new("second"))), s0.consume());
    assert_eq!(
        vec![(0, Rc::new("first")), (1, Rc::new("second"))],
        s0.find(|i| String::from(*i).contains("s"))
    );

    s0.produce(Rc::new("third"));
    assert_eq!(Some((2, Rc::new("third"))), s0.consume());
    assert_eq!(
        vec![(0, Rc::new("first")), (1, Rc::new("second"))],
        s0.find(|i| String::from(*i).contains("s"))
    );
    assert_eq!(None, s0.consume());
    assert_eq!(3, s0.offset);

    let mut s1 = broker.stream();
    assert_eq!(Some((0, Rc::new("first"))), s1.consume());
    assert_eq!(Some((1, Rc::new("second"))), s1.consume());
    assert_eq!(Some((2, Rc::new("third"))), s1.consume());
    assert_eq!(None, s1.consume());
}

#[test]
fn stream_produce_consume_with_offset() {
    let broker = Broker::new(10);
    let mut s0 = broker.stream();
    assert_eq!(10, s0.offset);
    assert_eq!(None, s0.consume());
    assert_eq!(
        Vec::<(usize, Rc<&str>)>::new(),
        s0.find(|i| String::from(*i).contains("s"))
    );

    s0.produce(Rc::new("first"));
    assert_eq!(Some((10, Rc::new("first"))), s0.consume());
    assert_eq!(
        vec![(10, Rc::new("first"))],
        s0.find(|i| String::from(*i).contains("s"))
    );

    s0.produce(Rc::new("second"));
    assert_eq!(Some((11, Rc::new("second"))), s0.consume());
    assert_eq!(
        vec![(10, Rc::new("first")), (11, Rc::new("second"))],
        s0.find(|i| String::from(*i).contains("s"))
    );

    s0.produce(Rc::new("third"));
    assert_eq!(Some((12, Rc::new("third"))), s0.consume());
    assert_eq!(
        vec![(10, Rc::new("first")), (11, Rc::new("second"))],
        s0.find(|i| String::from(*i).contains("s"))
    );
    assert_eq!(None, s0.consume());
    assert_eq!(13, s0.offset);

    let mut s1 = broker.stream();
    assert_eq!(Some((10, Rc::new("first"))), s1.consume());
    assert_eq!(Some((11, Rc::new("second"))), s1.consume());
    assert_eq!(Some((12, Rc::new("third"))), s1.consume());
    assert_eq!(None, s1.consume());
}

#[test]
fn uuidify_test() {
    assert_eq!(
        "00000000-0000-0000-0000-000000000000",
        &uuidify(0, 0).to_string()
    );
    assert_eq!(
        "00000000-0000-0000-0000-000000000001",
        &uuidify(0, 1).to_string()
    );
    assert_eq!(
        "00000000-0000-0001-0000-000000000000",
        &uuidify(1, 0).to_string()
    );
}
