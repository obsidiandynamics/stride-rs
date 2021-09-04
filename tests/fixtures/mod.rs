use uuid::Uuid;
use std::cell::RefCell;
use std::rc::Rc;
use std::ops::Deref;
use stride::{Message, Examiner, CandidateMessage, DecisionMessage};

pub struct Statemap {
    changes: Vec<(usize, i32)>
}

impl Statemap {
    pub fn new(changes: Vec<(usize, i32)>) -> Self {
        Statemap { changes }
    }
}

pub struct Cohort {
    pub pending: Vec<Uuid>,
    pub replica: Replica,
    pub candidates: Stream<CandidateMessage<Statemap>>,
    pub decisions: Stream<DecisionMessage<Statemap>>
}

pub struct Certifier {
    pub examiner: Examiner,
    pub candidates: Stream<CandidateMessage<Statemap>>,
    pub decisions: Stream<DecisionMessage<Statemap>>
}

pub struct Replica {
    pub items: Vec<(i32, u64)>,
    pub ver: u64
}

impl Replica {
    pub fn new(values: &Vec<i32>) -> Self {
        Replica {
            items: values.iter().map(|&i| (i, 0)).collect(),
            ver: 0
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

pub struct Broker<M> {
    messages: Rc<RefCell<Vec<Rc<M>>>>
}

impl<M> Broker<M> {
    pub fn new() -> Self {
        Broker { messages: Rc::new(RefCell::new(vec![])) }
    }

    pub fn stream(&self) -> Stream<M> {
        Stream { messages: Rc::clone(&self.messages), offset: 0 }
    }
}

pub struct Stream<M> {
    messages: Rc<RefCell<Vec<Rc<M>>>>,
    offset: usize
}

impl<M> Stream<M> {
    pub fn produce(&self, message: Rc<M>) {
        self.messages.borrow_mut().push(Rc::clone(&message));
    }

    pub fn consume(&mut self) -> Option<Rc<M>> {
        let messages = self.messages.borrow();
        match messages.get(self.offset) {
            None => None,
            Some(message) => {
                self.offset += 1;
                Some(Rc::clone(message))
            }
        }
    }

    pub fn find<P>(&self, predicate: P) -> Vec<Rc<M>>
        where P: Fn(&M) -> bool {
        let messages = self.messages.borrow();
        messages.iter()
            .filter(|&i| predicate(i.deref()))
            .map(|i| Rc::clone(&i))
            .collect()
    }
}

#[test]
fn replica_install_items() {
    let mut replica = Replica { items: vec![(10, 5), (20, 5), (30, 5)], ver: 5 };

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
    let mut replica = Replica { items: vec![(10, 5), (20, 5), (30, 5)], ver: 5 };

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
    let mut replica = Replica { items: vec![(10, 5), (20, 5), (30, 5)], ver: 5 };

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
    let broker = Broker::new();
    let mut s0 = broker.stream();
    assert_eq!(None, s0.consume());
    assert_eq!(Vec::<Rc<&str>>::new(),
               s0.find(|i| String::from(*i).contains("s") ));

    s0.produce(Rc::new("first"));
    assert_eq!(Some(Rc::new("first")), s0.consume());
    assert_eq!(vec![Rc::new("first")],
               s0.find(|i| String::from(*i).contains("s") ));
    s0.produce(Rc::new("second"));
    assert_eq!(Some(Rc::new("second")), s0.consume());
    assert_eq!(vec![Rc::new("first"), Rc::new("second")],
               s0.find(|i| String::from(*i).contains("s") ));
    s0.produce(Rc::new("third"));
    assert_eq!(Some(Rc::new("third")), s0.consume());
    assert_eq!(vec![Rc::new("first"), Rc::new("second")],
               s0.find(|i| String::from(*i).contains("s") ));
    assert_eq!(None, s0.consume());

    let mut s1 = broker.stream();
    assert_eq!(Some(Rc::new("first")), s1.consume());
    assert_eq!(Some(Rc::new("second")), s1.consume());
    assert_eq!(Some(Rc::new("third")), s1.consume());
    assert_eq!(None, s1.consume());
}