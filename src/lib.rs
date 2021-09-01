use crate::Outcome::{Abort, Commit};
use crate::Discord::{Assertive, Permissive};
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use uuid::Uuid;
use crate::AbortReason::{Staleness, Antidependency};

#[derive(Debug)]
pub struct Candidate {
    pub xid: Uuid,
    pub ver: u64,
    pub readset: Vec<String>,
    pub writeset: Vec<String>,
    pub readvers: Vec<u64>,
    pub snapshot: u64,
}

#[derive(Debug)]
pub struct Examiner {
    reads: HashMap<String, u64>,
    writes: HashMap<String, u64>,
    base: u64,
}

#[derive(PartialEq, Debug)]
pub enum Outcome {
    Commit(u64, Discord),
    Abort(AbortReason, Discord),
}

#[derive(PartialEq, Debug)]
pub enum AbortReason {
    Antidependency(u64),
    Staleness
}

#[derive(PartialEq, Debug)]
pub enum Discord {
    Permissive,
    Assertive,
}

impl Examiner {
    pub fn new() -> Examiner {
        Examiner {
            reads: HashMap::new(),
            writes: HashMap::new(),
            base: 1,
        }
    }

    pub fn learn(&mut self, candidate: &Candidate) {
        for read in candidate.readset.iter() {
            self.reads.insert(read.clone(), candidate.ver);
        }

        for write in candidate.writeset.iter() {
            self.writes.insert(write.clone(), candidate.ver);
        }
    }

    pub fn assess(&mut self, candidate: &Candidate) -> Outcome {
        let mut safepoint = self.base - 1;

        // rule R1: commit write-only transactions
        if candidate.readset.is_empty() {
            for candidate_write in candidate.writeset.iter() {
                // update read-write safepoint
                if let Some(&self_read) = self.reads.get(candidate_write) {
                    if self_read > safepoint {
                        safepoint = self_read;
                    }
                }

                // update safepoint for write-write intersection and learn the write
                match self.writes.entry(candidate_write.clone()) {
                    Entry::Occupied(mut entry) => {
                        let &self_write = entry.get();
                        if self_write > safepoint {
                            safepoint = self_write
                        }
                        entry.insert(candidate.ver);
                    }
                    Entry::Vacant(entry) => {
                        entry.insert(candidate.ver);
                    }
                }
            }
            return Commit(safepoint, Assertive);
        }

        // rule R2: conditionally abort transactions outside the suffix
        if candidate.snapshot < self.base - 1 {
            self.learn(candidate);
            return Abort(Staleness, Permissive);
        }

        // rule R3: abort on antidependency
        for candidate_read in candidate.readset.iter() {
            if let Some(&self_write) = self.writes.get(candidate_read) {
                if self_write > candidate.snapshot && !candidate.readvers.contains(&self_write) {
                    self.learn(candidate);
                    return Abort(Antidependency(self_write), Assertive);
                }

                if self_write > safepoint {
                    safepoint = self_write;
                }
            }
        }

        // rule R4 conditionally commit

        // update safepoint for read-write and write-write intersection, and learn the write
        for candidate_write in candidate.writeset.iter() {
            if let Some(&self_read) = self.reads.get(candidate_write) {
                if self_read > safepoint {
                    safepoint = self_read;
                }
            }

            match self.writes.entry(candidate_write.clone()) {
                Entry::Occupied(mut entry) => {
                    let &self_write = entry.get();
                    if self_write > safepoint {
                        safepoint = self_write
                    }
                    entry.insert(candidate.ver);
                }
                Entry::Vacant(entry) => {
                    entry.insert(candidate.ver);
                }
            }
        }

        // learn the reads
        for candidate_read in candidate.readset.iter() {
            self.reads.insert(candidate_read.clone(), candidate.ver);
        }

        Commit(safepoint, Permissive)
    }

    pub fn knows(&self, candidate: &Candidate) -> bool {
        for read in candidate.readset.iter() {
            match self.reads.get(read) {
                Some(&ver) if ver >= candidate.ver => {}
                _ => return false,
            }
        }
        for write in candidate.writeset.iter() {
            match self.writes.get(write) {
                Some(&ver) if ver >= candidate.ver => {}
                _ => return false,
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::{Candidate, Examiner};
    use crate::Outcome::{Abort, Commit};
    use crate::Discord::{Assertive, Permissive};
    use uuid::Uuid;
    use crate::AbortReason::{Staleness, Antidependency};

    #[test]
    fn learn_forget() {
        let mut examiner = Examiner::new();
        let candidate = Candidate {
            xid: Uuid::from_u128(1),
            ver: 5,
            readset: vec!["x".into()],
            writeset: vec!["y".into()],
            readvers: vec![],
            snapshot: 0,
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
            xid: Uuid::from_u128(1),
            ver: 4,
            readset: vec!["x".into(), "y".into()],
            writeset: vec!["x".into(), "y".into()],
            readvers: vec![],
            snapshot: 0,
        });
        examiner.learn(&Candidate {
            xid: Uuid::from_u128(2),
            ver: 5,
            readset: vec!["x".into(), "y".into()],
            writeset: vec![],
            readvers: vec![4],
            snapshot: 0,
        });
        let candidate = Candidate {
            xid: Uuid::from_u128(3),
            ver: 6,
            readset: vec![],
            writeset: vec!["x".into(), "y".into()],
            readvers: vec![],
            snapshot: 4,
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
            xid: Uuid::from_u128(1),
            ver: 12,
            readset: vec!["x".into(), "y".into()],
            writeset: vec!["x".into(), "y".into()],
            readvers: vec![],
            snapshot: 11,
        });
        examiner.learn(&Candidate {
            xid: Uuid::from_u128(2),
            ver: 13,
            readset: vec!["x".into(), "y".into()],
            writeset: vec![],
            readvers: vec![],
            snapshot: 12,
        });
        examiner.learn(&Candidate {
            xid: Uuid::from_u128(3),
            ver: 14,
            readset: vec![],
            writeset: vec!["x".into(), "y".into()],
            readvers: vec![],
            snapshot: 5,
        });
        let candidate = Candidate {
            xid: Uuid::from_u128(4),
            ver: 15,
            readset: vec!["v".into(), "w".into()],
            writeset: vec!["z".into()],
            readvers: vec![],
            snapshot: 10,
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
            xid: Uuid::from_u128(1),
            ver: 24,
            readset: vec!["x".into(), "y".into()],
            writeset: vec![],
            readvers: vec![],
            snapshot: 19,
        });
        examiner.learn(&Candidate {
            xid: Uuid::from_u128(2),
            ver: 25,
            readset: vec!["x".into(), "y".into()],
            writeset: vec!["x".into(), "y".into()],
            readvers: vec![],
            snapshot: 22,
        });
        examiner.learn(&Candidate {
            xid: Uuid::from_u128(3),
            ver: 26,
            readset: vec![],
            writeset: vec!["y".into(), "z".into()],
            readvers: vec![],
            snapshot: 25,
        });
        examiner.learn(&Candidate {
            xid: Uuid::from_u128(4),
            ver: 27,
            readset: vec!["v".into(), "w".into()],
            writeset: vec![],
            readvers: vec![],
            snapshot: 26,
        });
        let candidate = Candidate {
            xid: Uuid::from_u128(5),
            ver: 28,
            readset: vec!["x".into(), "z".into()],
            writeset: vec!["z".into()],
            readvers: vec![25],
            snapshot: 23,
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
            xid: Uuid::from_u128(1),
            ver: 30,
            readset: vec!["x".into(), "y".into()],
            writeset: vec![],
            readvers: vec![],
            snapshot: 23,
        });
        examiner.learn(&Candidate {
            xid: Uuid::from_u128(2),
            ver: 31,
            readset: vec!["x".into(), "y".into()],
            writeset: vec!["w".into(), "x".into()],
            readvers: vec![],
            snapshot: 24,
        });
        examiner.learn(&Candidate {
            xid: Uuid::from_u128(3),
            ver: 32,
            readset: vec![],
            writeset: vec!["y".into()],
            readvers: vec![],
            snapshot: 25,
        });
        examiner.learn(&Candidate {
            xid: Uuid::from_u128(4),
            ver: 33,
            readset: vec!["v".into(), "z".into()],
            writeset: vec!["y".into()],
            readvers: vec![],
            snapshot: 26,
        });
        examiner.learn(&Candidate {
            xid: Uuid::from_u128(5),
            ver: 34,
            readset: vec![],
            writeset: vec!["w".into()],
            readvers: vec![],
            snapshot: 31,
        });
        let candidate = Candidate {
            xid: Uuid::from_u128(6),
            ver: 35,
            readset: vec!["x".into(), "z".into()],
            writeset: vec!["z".into()],
            readvers: vec![],
            snapshot: 31,
        };
        assert!(!examiner.knows(&candidate));
        let outcome = examiner.assess(&candidate);
        assert_eq!(Commit(33, Permissive), outcome);
        assert_learned(&examiner, &candidate)
    }

    fn assert_learned(examiner: &Examiner, candidate: &Candidate) {
        if !examiner.knows(&candidate) {
            for read in candidate.readset.iter() {
                match examiner.reads.get(read) {
                    Some(&ver) if ver >= candidate.ver => {}
                    _ => panic!(
                        "{:?} not known to {:?} for read of {}",
                        candidate, examiner, read
                    ),
                }
            }
            for write in candidate.writeset.iter() {
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
}