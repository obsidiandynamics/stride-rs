use std::convert::{TryFrom, TryInto};
use std::mem::size_of;
use crate::utils::{uuidify, deuuid};
use uuid::Uuid;
use std::str::FromStr;
use std::fmt::Debug;

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
