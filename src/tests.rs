use super::{Examiner, Record};
use crate::AbortReason::{Antidependency, Staleness};
use crate::Candidate;
use crate::Discord::{Assertive, Permissive};
use crate::Outcome::{Abort, Commit};
use uuid::Uuid;

#[test]
fn learn_forget() {
    let mut examiner = Examiner::new();
    let candidate = Candidate {
        rec: Record {
            xid: Uuid::from_u128(1),
            readset: vec!["x".into()],
            writeset: vec!["y".into()],
            readvers: vec![],
            snapshot: 0,
        },
        ver: 5,
    };
    assert!(!examiner.knows(&candidate));
    examiner.learn(&candidate);
    assert_learned(&examiner, &candidate)
    //TODO test forget()
}

#[test]
fn paper_example_1() {
    let mut examiner = Examiner::new();
    examiner.base = 4;
    examiner.learn(&Candidate {
        rec: Record {
            xid: Uuid::from_u128(1),
            readset: vec!["x".into(), "y".into()],
            writeset: vec!["x".into(), "y".into()],
            readvers: vec![],
            snapshot: 0,
        },
        ver: 4,
    });
    examiner.learn(&Candidate {
        rec: Record {
            xid: Uuid::from_u128(2),
            readset: vec!["x".into(), "y".into()],
            writeset: vec![],
            readvers: vec![4],
            snapshot: 0,
        },
        ver: 5,
    });
    let candidate = Candidate {
        rec: Record {
            xid: Uuid::from_u128(3),
            readset: vec![],
            writeset: vec!["x".into(), "y".into()],
            readvers: vec![],
            snapshot: 4,
        },
        ver: 6,
    };
    assert!(!examiner.knows(&candidate));
    let outcome = examiner.assess(&candidate);
    assert_eq!(Commit(5, Assertive), outcome);
    assert_learned(&examiner, &candidate)
}

#[test]
fn paper_example_2() {
    let mut examiner = Examiner::new();
    examiner.base = 12;
    examiner.learn(&Candidate {
        rec: Record {
            xid: Uuid::from_u128(1),
            readset: vec!["x".into(), "y".into()],
            writeset: vec!["x".into(), "y".into()],
            readvers: vec![],
            snapshot: 11,
        },
        ver: 12,
    });
    examiner.learn(&Candidate {
        rec: Record {
            xid: Uuid::from_u128(2),
            readset: vec!["x".into(), "y".into()],
            writeset: vec![],
            readvers: vec![],
            snapshot: 12,
        },
        ver: 13,
    });
    examiner.learn(&Candidate {
        rec: Record {
            xid: Uuid::from_u128(3),
            readset: vec![],
            writeset: vec!["x".into(), "y".into()],
            readvers: vec![],
            snapshot: 5,
        },
        ver: 14,
    });
    let candidate = Candidate {
        rec: Record {
            xid: Uuid::from_u128(4),
            readset: vec!["v".into(), "w".into()],
            writeset: vec!["z".into()],
            readvers: vec![],
            snapshot: 10,
        },
        ver: 15,
    };
    assert!(!examiner.knows(&candidate));
    let outcome = examiner.assess(&candidate);
    assert_eq!(Abort(Staleness, Permissive), outcome);
    assert_learned(&examiner, &candidate)
}

#[test]
fn paper_example_3() {
    let mut examiner = Examiner::new();
    examiner.base = 24;
    examiner.learn(&Candidate {
        rec: Record {
            xid: Uuid::from_u128(1),
            readset: vec!["x".into(), "y".into()],
            writeset: vec![],
            readvers: vec![],
            snapshot: 19,
        },
        ver: 24,
    });
    examiner.learn(&Candidate {
        rec: Record {
            xid: Uuid::from_u128(2),
            readset: vec!["x".into(), "y".into()],
            writeset: vec!["x".into(), "y".into()],
            readvers: vec![],
            snapshot: 22,
        },
        ver: 25,
    });
    examiner.learn(&Candidate {
        rec: Record {
            xid: Uuid::from_u128(3),
            readset: vec![],
            writeset: vec!["y".into(), "z".into()],
            readvers: vec![],
            snapshot: 25,
        },
        ver: 26,
    });
    examiner.learn(&Candidate {
        rec: Record {
            xid: Uuid::from_u128(4),
            readset: vec!["v".into(), "w".into()],
            writeset: vec![],
            readvers: vec![],
            snapshot: 26,
        },
        ver: 27,
    });
    let candidate = Candidate {
        rec: Record {
            xid: Uuid::from_u128(5),
            readset: vec!["x".into(), "z".into()],
            writeset: vec!["z".into()],
            readvers: vec![25],
            snapshot: 23,
        },
        ver: 28,
    };
    assert!(!examiner.knows(&candidate));
    let outcome = examiner.assess(&candidate);
    assert_eq!(Abort(Antidependency(26), Assertive), outcome);
    assert_learned(&examiner, &candidate)
}

#[test]
fn paper_example_4() {
    let mut examiner = Examiner::new();
    examiner.base = 30;
    examiner.learn(&Candidate {
        rec: Record {
            xid: Uuid::from_u128(1),
            readset: vec!["x".into(), "y".into()],
            writeset: vec![],
            readvers: vec![],
            snapshot: 23,
        },
        ver: 30,
    });
    examiner.learn(&Candidate {
        rec: Record {
            xid: Uuid::from_u128(2),
            readset: vec!["x".into(), "y".into()],
            writeset: vec!["w".into(), "x".into()],
            readvers: vec![],
            snapshot: 24,
        },
        ver: 31,
    });
    examiner.learn(&Candidate {
        rec: Record {
            xid: Uuid::from_u128(3),
            readset: vec![],
            writeset: vec!["y".into()],
            readvers: vec![],
            snapshot: 25,
        },
        ver: 32,
    });
    examiner.learn(&Candidate {
        rec: Record {
            xid: Uuid::from_u128(4),
            readset: vec!["v".into(), "z".into()],
            writeset: vec!["y".into()],
            readvers: vec![],
            snapshot: 26,
        },
        ver: 33,
    });
    examiner.learn(&Candidate {
        rec: Record {
            xid: Uuid::from_u128(5),
            readset: vec![],
            writeset: vec!["w".into()],
            readvers: vec![],
            snapshot: 31,
        },
        ver: 34,
    });
    let candidate = Candidate {
        rec: Record {
            xid: Uuid::from_u128(6),
            readset: vec!["x".into(), "z".into()],
            writeset: vec!["z".into()],
            readvers: vec![],
            snapshot: 31,
        },
        ver: 35,
    };
    assert!(!examiner.knows(&candidate));
    let outcome = examiner.assess(&candidate);
    assert_eq!(Commit(33, Permissive), outcome);
    assert_learned(&examiner, &candidate)
}

fn assert_learned(examiner: &Examiner, candidate: &Candidate) {
    if !examiner.knows(&candidate) {
        for read in candidate.rec.readset.iter() {
            match examiner.reads.get(read) {
                Some(&ver) if ver >= candidate.ver => {}
                _ => panic!(
                    "{:?} not known to {:?} for read of {}",
                    candidate, examiner, read
                ),
            }
        }
        for write in candidate.rec.writeset.iter() {
            match examiner.writes.get(write) {
                Some(&ver) if ver >= candidate.ver => {}
                _ => panic!(
                    "{:?} not known to {:?} for write of {}",
                    candidate, examiner, write
                ),
            }
        }
    }
}

#[test]
fn compress() {
    assert_eq!((vec![], 10), Record::compress(vec![3, 6, 9], 10));
    assert_eq!((vec![6, 9], 4), Record::compress(vec![3, 6, 9], 4));
    assert_eq!((vec![6, 9], 3), Record::compress(vec![3, 6, 9], 1));
}