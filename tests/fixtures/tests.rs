use crate::fixtures::{uuidify, Statemap, Replica, Broker};
use std::rc::Rc;

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
    assert!(!replica.can_install_ooo(&Statemap::new(vec![(0, 11)]), 5, 5));
    replica.install_ooo(&Statemap::new(vec![(0, 11)]), 5, 5);
    assert_eq!(vec![(10, 5), (20, 5), (30, 5)], replica.items);
    assert_eq!(5, replica.ver);

    // non-empty statemap at the greater safepoint and greater version -- no change expected
    assert!(!replica.can_install_ooo(&Statemap::new(vec![(0, 11)]), 6, 6));
    replica.install_ooo(&Statemap::new(vec![(0, 11)]), 6, 6);
    assert_eq!(vec![(10, 5), (20, 5), (30, 5)], replica.items);
    assert_eq!(5, replica.ver);

    // non-empty statemap at the same safepoint and greater version -- expect changes
    assert!(replica.can_install_ooo(&Statemap::new(vec![(0, 11)]), 5, 6));
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
