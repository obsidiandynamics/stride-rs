use crate::fixtures::broker::Broker;
use std::rc::Rc;

#[test]
fn stream_produce_consume() {
    let broker = Broker::new(0);
    let mut s0 = broker.stream();
    assert_eq!(0, s0.offset());
    assert_eq!(0, s0.low_watermark());
    assert_eq!(0, s0.high_watermark());
    assert_eq!(0, s0.len());
    assert_eq!(None, s0.consume());
    assert_eq!(
        Vec::<(usize, Rc<&str>)>::new(),
        s0.find(|i| String::from(*i).contains("s"))
    );

    s0.produce(Rc::new("first"));
    assert_eq!(0, s0.low_watermark());
    assert_eq!(1, s0.high_watermark());
    assert_eq!(1, s0.len());
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
    assert_eq!(3, s0.offset());

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
    assert_eq!(10, s0.offset());
    assert_eq!(10, s0.low_watermark());
    assert_eq!(10, s0.high_watermark());
    assert_eq!(0, s0.len());
    assert_eq!(None, s0.consume());
    assert_eq!(
        Vec::<(usize, Rc<&str>)>::new(),
        s0.find(|i| String::from(*i).contains("s"))
    );

    s0.produce(Rc::new("first"));
    assert_eq!(10, s0.low_watermark());
    assert_eq!(11, s0.high_watermark());
    assert_eq!(1, s0.len());
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
