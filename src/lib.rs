use std::collections::HashMap;
use crate::Outcome::{Commit, Abort};
use std::collections::hash_map::Entry;
use crate::Past::{Assertive, Permissive};
use uuid::Uuid;

#[derive(Debug)]
struct Candidate {
    xid: Uuid,
    ver: u64,
    readset: Vec<String>,
    writeset: Vec<String>,
    readvers: Vec<u64>,
    snapshot: u64
}

#[derive(Debug)]
struct Examiner {
    reads: HashMap<String, u64>,
    writes: HashMap<String, u64>,
    base: u64
}

#[derive(PartialEq, Debug)]
enum Outcome {
    Commit(u64, Past),
    Abort(Past)
}

#[derive(PartialEq, Debug)]
enum Past {
    Permissive, Assertive
}

impl Examiner {
    fn new() -> Examiner {
        Examiner { reads: HashMap::new(), writes: HashMap::new(), base: 1 }
    }

    fn learn(&mut self, candidate: &Candidate) {
        for read in candidate.readset.iter() {
            self.reads.insert(read.clone(), candidate.ver);
        }

        for write in candidate.writeset.iter() {
            self.writes.insert(write.clone(), candidate.ver);
        }
    }

    fn assess(&mut self, candidate: &Candidate) -> Outcome {
        let mut safepoint = self.base - 1;

        // rule R1: commit write-only transactions
        if candidate.readset.is_empty() {
            for write in candidate.writeset.iter() {
                // update read-write safepoint
                if let Some(&read) = self.reads.get(write) {
                    if read > safepoint {
                        safepoint = read;
                    }
                }

                // update write-write safepoint and learn the write
                match self.writes.entry(write.clone()) {
                    Entry::Occupied(mut entry) => {
                        let &write = entry.get();
                        if write > safepoint {
                            safepoint = write
                        }
                        entry.insert(candidate.ver);
                    }
                    Entry::Vacant(entry) => {
                        entry.insert(candidate.ver);
                    }
                }
            }
            return Commit(safepoint, Assertive)
        }

        // rule R2: conditionally abort transactions outside the suffix
        if candidate.snapshot < self.base - 1 {
            self.learn(candidate);
            return Abort(Permissive)
        }

        // rule R3: abort on antidependency
        for read in candidate.readset.iter() {
            if let Some(&write) = self.writes.get(read) {
                if write > candidate.snapshot && ! candidate.readvers.contains(&write) {
                    self.learn(candidate);
                    return Abort(Assertive)
                }
            }
        }

        // rule R4 conditionally commit
        Commit(safepoint, Permissive)
    }

    fn knows(&self, candidate: &Candidate) -> bool {
        for read in candidate.readset.iter() {
            match self.reads.get(read) {
                Some(&ver) if ver >= candidate.ver => {}
                _ => return false
            }
        }
        for write in candidate.writeset.iter() {
            match self.writes.get(write) {
                Some(&ver) if ver >= candidate.ver => {}
                _ => return false
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::{Examiner, Candidate};
    use crate::Outcome::{Commit, Abort};
    use crate::Past::{Assertive, Permissive};
    use uuid::Uuid;

    #[test]
    fn learn_forget() {
        let mut examiner = Examiner::new();
        let candidate = Candidate {
            xid: Uuid::from_u128(1),
            ver: 5,
            readset: vec!["x".into()],
            writeset: vec!["y".into()],
            readvers: vec![],
            snapshot: 0
        };
        println!("candidate {:?}", candidate);
        assert!(!examiner.knows(&candidate));
        examiner.learn(&candidate);
        assert!(examiner.knows(&candidate));
        //TODO test forget()
    }

    #[test]
    fn paper_example_1() {
        let mut examiner = Examiner::new();
        examiner.base = 4;
        examiner.learn(&Candidate {
            xid: Uuid::from_u128(1),
            ver: 4,
            readset: vec!["x".into(), "y".into()],
            writeset: vec!["x".into(), "y".into()],
            readvers: vec![],
            snapshot: 0
        });
        examiner.learn(&Candidate {
            xid: Uuid::from_u128(2),
            ver: 5,
            readset: vec!["x".into(), "y".into()],
            writeset: vec![],
            readvers: vec![4],
            snapshot: 0
        });
        let candidate = Candidate {
            xid: Uuid::from_u128(3),
            ver: 6,
            readset: vec![],
            writeset: vec!["x".into(), "y".into()],
            readvers: vec![],
            snapshot: 4
        };
        assert!(!examiner.knows(&candidate));
        let outcome = examiner.assess(&candidate);
        assert_eq!(Commit(5, Assertive), outcome);
        assert!(examiner.knows(&candidate));
    }

    #[test]
    fn paper_example_2() {
        let mut examiner = Examiner::new();
        examiner.base = 12;
        examiner.learn(&Candidate {
            xid: Uuid::from_u128(1),
            ver: 12,
            readset: vec!["x".into(), "y".into()],
            writeset: vec!["x".into(), "y".into()],
            readvers: vec![],
            snapshot: 11
        });
        examiner.learn(&Candidate {
            xid: Uuid::from_u128(2),
            ver: 13,
            readset: vec!["x".into(), "y".into()],
            writeset: vec![],
            readvers: vec![],
            snapshot: 12
        });
        examiner.learn(&Candidate {
            xid: Uuid::from_u128(3),
            ver: 14,
            readset: vec![],
            writeset: vec!["x".into(), "y".into()],
            readvers: vec![],
            snapshot: 5
        });
        let candidate = Candidate {
            xid: Uuid::from_u128(4),
            ver: 15,
            readset: vec!["v".into(), "w".into()],
            writeset: vec!["z".into()],
            readvers: vec![],
            snapshot: 10
        };
        assert!(!examiner.knows(&candidate));
        let outcome = examiner.assess(&candidate);
        assert_eq!(Abort(Permissive), outcome);
        assert!(examiner.knows(&candidate));
    }

    #[test]
    fn paper_example_3() {
        let mut examiner = Examiner::new();
        examiner.base = 24;
        examiner.learn(&Candidate {
            xid: Uuid::from_u128(1),
            ver: 24,
            readset: vec!["x".into(), "y".into()],
            writeset: vec![],
            readvers: vec![],
            snapshot: 19
        });
        examiner.learn(&Candidate {
            xid: Uuid::from_u128(2),
            ver: 25,
            readset: vec!["x".into(), "y".into()],
            writeset: vec!["x".into(), "y".into()],
            readvers: vec![],
            snapshot: 22
        });
        examiner.learn(&Candidate {
            xid: Uuid::from_u128(3),
            ver: 26,
            readset: vec![],
            writeset: vec!["y".into(), "z".into()],
            readvers: vec![],
            snapshot: 25
        });
        examiner.learn(&Candidate {
            xid: Uuid::from_u128(4),
            ver: 27,
            readset: vec!["v".into(), "w".into()],
            writeset: vec![],
            readvers: vec![],
            snapshot: 26
        });
        let candidate = Candidate {
            xid: Uuid::from_u128(5),
            ver: 28,
            readset: vec!["x".into(), "z".into()],
            writeset: vec!["z".into()],
            readvers: vec![25],
            snapshot: 23
        };
        assert!(!examiner.knows(&candidate));
        let outcome = examiner.assess(&candidate);
        assert_eq!(Abort(Assertive), outcome);
        assert!(examiner.knows(&candidate));
    }
    // fn assert_learned(examiner: &Examiner, candidate: &Candidate) {
    //     for read in candidate.readset.iter() {
    //         match examiner.reads.get(read) {
    //             Some(&ver) if ver >= candidate.ver => {}
    //             _ => panic!("{:?} not known to {:?} for read of {}", candidate, examiner, read)
    //         }
    //     }
    //     for write in candidate.writeset.iter() {
    //         match examiner.writes.get(write) {
    //             Some(&ver) if ver >= candidate.ver => {}
    //             _ => panic!("{:?} not known to {:?} for write of {}", candidate, examiner, write)
    //         }
    //     }
    // }
}
