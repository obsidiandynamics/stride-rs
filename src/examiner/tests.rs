use crate::examiner::{Examiner, Record, Candidate};
use crate::examiner::Discord::{Permissive, Assertive};
use crate::examiner::Outcome::{Commit, Abort};
use uuid::Uuid;
use crate::suffix::TruncatedEntry;
use crate::examiner::AbortReason::{Staleness, Antidependency};

impl Examiner {
    fn knows(&self, candidate: &Candidate) -> bool {
        for read in candidate.rec.readset.iter() {
            match self.reads.get(read) {
                Some(&ver) if ver >= candidate.ver => {}
                _ => return false,
            }
        }
        for write in candidate.rec.writeset.iter() {
            match self.writes.get(write) {
                Some(&ver) if ver >= candidate.ver => {}
                _ => return false,
            }
        }
        true
    }
}

impl Candidate {
    fn truncated(&self) -> TruncatedEntry {
        TruncatedEntry {
            readset: self.rec.readset.clone(),
            writeset: self.rec.writeset.clone(),
            ver: self.ver
        }
    }
}

#[test]
fn learn_discard() {
    let mut examiner = Examiner::new();
    assert_eq!(None, examiner.base());
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

    examiner.learn(candidate.clone());
    assert_eq!(Some(5), examiner.base());
    assert_knows(&examiner, &candidate);

    examiner.discard(candidate.truncated());
    assert!(!examiner.knows(&candidate));
    assert_eq!(Some(6), examiner.base());
}

#[test]
fn learn_discard_two_with_identical_items() {
    let mut examiner = Examiner::new();
    let c1 = Candidate {
        rec: Record {
            xid: Uuid::from_u128(1),
            readset: vec!["x".into()],
            writeset: vec!["y".into()],
            readvers: vec![],
            snapshot: 0,
        },
        ver: 1,
    };
    examiner.learn(c1.clone());
    let c2 = Candidate {
        rec: Record {
            xid: Uuid::from_u128(2),
            readset: vec!["x".into()],
            writeset: vec!["y".into()],
            readvers: vec![],
            snapshot: 0,
        },
        ver: 2,
    };
    examiner.learn(c2.clone());
    assert_eq!(Some(1), examiner.base());
    assert_knows(&examiner, &c1);
    assert_knows(&examiner, &c2);

    examiner.discard(c1.truncated());
    assert_eq!(Some(2), examiner.base());
    // because c1 and c2 fully overlap, the examiner still appears to 'know' c1
    assert_knows(&examiner, &c1);
    assert_knows(&examiner, &c2);

    examiner.discard(c2.truncated());
    assert_eq!(Some(3), examiner.base());
    assert!(! examiner.knows(&c1));
    assert!(! examiner.knows(&c2));
}


#[test]
fn learn_discard_two_with_nonidentical_items() {
    let mut examiner = Examiner::new();
    let c1 = Candidate {
        rec: Record {
            xid: Uuid::from_u128(1),
            readset: vec!["a".into(), "b".into()],
            writeset: vec!["x".into(), "y".into()],
            readvers: vec![],
            snapshot: 0,
        },
        ver: 1,
    };
    examiner.learn(c1.clone());
    let c2 = Candidate {
        rec: Record {
            xid: Uuid::from_u128(2),
            readset: vec!["b".into(), "c".into()],
            writeset: vec!["x".into(), "y".into()],
            readvers: vec![],
            snapshot: 0,
        },
        ver: 2,
    };
    examiner.learn(c2.clone());
    assert_eq!(Some(1), examiner.base());
    assert_knows(&examiner, &c1);
    assert_knows(&examiner, &c2);

    examiner.discard(c1.truncated());
    assert_eq!(Some(2), examiner.base());
    assert!(! examiner.knows(&c1));
    assert_knows(&examiner, &c2);

    examiner.discard(c2.truncated());
    assert_eq!(Some(3), examiner.base());
    assert!(! examiner.knows(&c1));
    assert!(! examiner.knows(&c2));
}

#[test] #[should_panic(expected = "uninitialized examiner")]
fn discard_uninitialized() {
    Examiner::new().discard(TruncatedEntry {
        readset: vec![],
        writeset: vec![],
        ver: 0,
    })
}

#[test] #[should_panic(expected = "entry.ver (1) < self.base (2)")]
fn discard_nonmonotonic() {
    let mut examiner = Examiner::new();
    examiner.learn(Candidate {
        rec: Record {
            xid: Uuid::nil(),
            readset: vec![],
            writeset: vec![],
            readvers: vec![],
            snapshot: 0,
        },
        ver: 2,
    });
    examiner.discard(TruncatedEntry {
        ver: 1,
        readset: vec![],
        writeset: vec![]
    });
}

#[test] #[should_panic(expected = "unsupported version 0")]
fn learn_ver_0() {
    Examiner::new().learn(Candidate {
        rec: Record {
            xid: Uuid::default(),
            readset: vec![],
            writeset: vec![],
            readvers: vec![],
            snapshot: 0,
        },
        ver: 0,
    });
}

#[test] #[should_panic(expected = "unsupported version 0")]
fn assess_ver_0() {
    Examiner::new().assess(Candidate {
        rec: Record {
            xid: Uuid::default(),
            readset: vec![],
            writeset: vec![],
            readvers: vec![],
            snapshot: 0,
        },
        ver: 0,
    });
}

#[test]
fn paper_example_1() {
    let mut examiner = Examiner::new();
    examiner.base = 4;
    examiner.learn(Candidate {
        rec: Record {
            xid: Uuid::from_u128(1),
            readset: vec!["x".into(), "y".into()],
            writeset: vec!["x".into(), "y".into()],
            readvers: vec![],
            snapshot: 0,
        },
        ver: 4,
    });
    examiner.learn(Candidate {
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
    let outcome = examiner.assess(candidate.clone());
    assert_eq!(Commit {safepoint: 5, discord: Assertive}, outcome);
    assert_knows(&examiner, &candidate)
}

#[test]
fn paper_example_2() {
    let mut examiner = Examiner::new();
    examiner.base = 12;
    examiner.learn(Candidate {
        rec: Record {
            xid: Uuid::from_u128(1),
            readset: vec!["x".into(), "y".into()],
            writeset: vec!["x".into(), "y".into()],
            readvers: vec![],
            snapshot: 11,
        },
        ver: 12,
    });
    examiner.learn(Candidate {
        rec: Record {
            xid: Uuid::from_u128(2),
            readset: vec!["x".into(), "y".into()],
            writeset: vec![],
            readvers: vec![],
            snapshot: 12,
        },
        ver: 13,
    });
    examiner.learn(Candidate {
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
    let outcome = examiner.assess(candidate.clone());
    assert_eq!(Abort {reason: Staleness, discord: Permissive}, outcome);
    assert_knows(&examiner, &candidate)
}

#[test]
fn paper_example_3() {
    let mut examiner = Examiner::new();
    examiner.base = 24;
    examiner.learn(Candidate {
        rec: Record {
            xid: Uuid::from_u128(1),
            readset: vec!["x".into(), "y".into()],
            writeset: vec![],
            readvers: vec![],
            snapshot: 19,
        },
        ver: 24,
    });
    examiner.learn(Candidate {
        rec: Record {
            xid: Uuid::from_u128(2),
            readset: vec!["x".into(), "y".into()],
            writeset: vec!["x".into(), "y".into()],
            readvers: vec![],
            snapshot: 22,
        },
        ver: 25,
    });
    examiner.learn(Candidate {
        rec: Record {
            xid: Uuid::from_u128(3),
            readset: vec![],
            writeset: vec!["y".into(), "z".into()],
            readvers: vec![],
            snapshot: 25,
        },
        ver: 26,
    });
    examiner.learn(Candidate {
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
    let outcome = examiner.assess(candidate.clone());
    assert_eq!(Abort {reason: Antidependency(26), discord: Assertive}, outcome);
    assert_knows(&examiner, &candidate)
}

#[test]
fn paper_example_4() {
    let mut examiner = Examiner::new();
    examiner.base = 30;
    examiner.learn(Candidate {
        rec: Record {
            xid: Uuid::from_u128(1),
            readset: vec!["x".into(), "y".into()],
            writeset: vec![],
            readvers: vec![],
            snapshot: 23,
        },
        ver: 30,
    });
    examiner.learn(Candidate {
        rec: Record {
            xid: Uuid::from_u128(2),
            readset: vec!["x".into(), "y".into()],
            writeset: vec!["w".into(), "x".into()],
            readvers: vec![],
            snapshot: 24,
        },
        ver: 31,
    });
    examiner.learn(Candidate {
        rec: Record {
            xid: Uuid::from_u128(3),
            readset: vec![],
            writeset: vec!["y".into()],
            readvers: vec![],
            snapshot: 25,
        },
        ver: 32,
    });
    examiner.learn(Candidate {
        rec: Record {
            xid: Uuid::from_u128(4),
            readset: vec!["v".into(), "z".into()],
            writeset: vec!["y".into()],
            readvers: vec![],
            snapshot: 26,
        },
        ver: 33,
    });
    examiner.learn(Candidate {
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
    let outcome = examiner.assess(candidate.clone());
    assert_eq!(Commit {safepoint: 33, discord: Permissive}, outcome);
    assert_knows(&examiner, &candidate)
}

fn assert_knows(examiner: &Examiner, candidate: &Candidate) {
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