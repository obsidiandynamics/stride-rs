use crate::fixtures::{deuuid, uuidify, Broker, Replica, Statemap};
use std::convert::{TryFrom, TryInto};
use std::fmt::Debug;
use std::mem::size_of;
use std::rc::Rc;
use std::str::FromStr;
use uuid::Uuid;

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

#[derive(Debug)]
struct UuidExpectation<'a, P, R> {
    pid: P,
    run: R,
    str: &'a str,
}

fn test_uuid<P, R>(expectations: &[UuidExpectation<P, R>])
where
    P: TryInto<u64> + TryFrom<u64> + Copy + Debug + Eq,
    <P as TryInto<u64>>::Error: Debug,
    <P as TryFrom<u64>>::Error: Debug,
    R: TryInto<u64> + TryFrom<u64> + Copy + Debug + Eq,
    <R as TryFrom<u64>>::Error: Debug,
    <R as TryInto<u64>>::Error: Debug,
{
    for exp in expectations {
        let uuid = uuidify(exp.pid, exp.run);
        assert_eq!(exp.str, uuid.to_string(), "for {:?}", exp);
        let (pid, run) = deuuid::<P, R>(uuid);
        assert_eq!(exp.pid, pid, "for {:?}", exp);
        assert_eq!(exp.run, run, "for {:?}", exp);
    }
}

#[test]
fn uuidify_deuuid_usize() {
    test_uuid(&[
        UuidExpectation::<usize, usize> {
            pid: 0,
            run: 0,
            str: "00000000-0000-0000-0000-000000000000",
        },
        UuidExpectation::<usize, usize> {
            pid: 0,
            run: 1,
            str: "00000000-0000-0000-0000-000000000001",
        },
        UuidExpectation::<usize, usize> {
            pid: 0,
            run: 0x76543210,
            str: "00000000-0000-0000-0000-000076543210",
        },
        UuidExpectation::<usize, usize> {
            pid: 1,
            run: 0,
            str: "00000000-0000-0001-0000-000000000000",
        },
        UuidExpectation::<usize, usize> {
            pid: 0x76543210,
            run: 0,
            str: "00000000-7654-3210-0000-000000000000",
        },
        UuidExpectation::<usize, usize> {
            pid: 0x76543210,
            run: 0xfedcba98,
            str: "00000000-7654-3210-0000-0000fedcba98",
        },
    ]);

    // conditional on 64-bit architecture
    if size_of::<usize>() == 8 {
        test_uuid(&[
            UuidExpectation::<usize, usize> {
                pid: 0,
                run: 0xfedcba9876543210,
                str: "00000000-0000-0000-fedc-ba9876543210",
            },
            UuidExpectation::<usize, usize> {
                pid: 0xfedcba9876543210,
                run: 0,
                str: "fedcba98-7654-3210-0000-000000000000",
            },
        ]);
    }
}

#[test]
fn uuidify_deuuid_i32() {
    test_uuid(&[UuidExpectation {
        pid: 5i16,
        run: 6i16,
        str: "00000000-0000-0005-0000-000000000006",
    }]);
}

#[test]
#[should_panic]
fn uuidify_i32_negative() {
    uuidify(5i16, -6i16);
}

#[test]
fn uuidify_deuuid_u128() {
    test_uuid(&[UuidExpectation {
        pid: 0xffff_ffff_ffff_ffff_u128,
        run: 0xffff_ffff_ffff_ffff_u128,
        str: "ffffffff-ffff-ffff-ffff-ffffffffffff",
    }]);
}

#[test]
#[should_panic]
fn uuidify_u128_overflow() {
    uuidify(0xffff_ffff_ffff_ffff_u128, 0x1_0000_0000_0000_0000_u128);
}

#[test]
fn uuidify_deuuid_u8() {
    test_uuid(&[UuidExpectation {
        pid: 0xee_u8,
        run: 0xff_u8,
        str: "00000000-0000-00ee-0000-0000000000ff",
    }]);
}

#[test]
#[should_panic]
fn deuuid_u8_overflow() {
    let uuid = Uuid::from_str("00000000-0000-0000-0000-000000000100").unwrap();
    deuuid::<u8, u8>(uuid);
}
