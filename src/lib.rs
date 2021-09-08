pub mod havoc;

use crate::AbortReason::{Antidependency, Staleness};
use crate::Discord::{Assertive, Permissive};
use crate::Outcome::{Abort, Commit};
use rustc_hash::FxHashMap;
use std::collections::hash_map::Entry;
use uuid::Uuid;

//TODO this doesn't need to be part of the examiner
#[derive(Debug)]
pub enum Message<S: Clone> {
    Candidate(CandidateMessage<S>),
    Decision(DecisionMessage<S>),
}

#[derive(Debug)]
pub struct CandidateMessage<S: Clone> {
    pub rec: Record,
    pub statemap: S,
}

#[derive(Debug)]
pub struct DecisionMessage<S: Clone> {
    pub candidate: Candidate,
    pub outcome: Outcome,
    pub statemap: Option<S>,
}

#[derive(Debug)]
pub struct Candidate {
    pub rec: Record,
    pub ver: u64,
}

#[derive(Debug, Clone)]
pub struct Record {
    pub xid: Uuid,
    pub readset: Vec<String>,
    pub writeset: Vec<String>,
    pub readvers: Vec<u64>,
    pub snapshot: u64,
}

#[derive(Debug)]
pub struct Examiner {
    reads: FxHashMap<String, u64>,
    writes: FxHashMap<String, u64>,
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
    Staleness,
}

#[derive(PartialEq, Debug)]
pub enum Discord {
    Permissive,
    Assertive,
}

impl Examiner {
    pub fn new() -> Examiner {
        Examiner {
            reads: FxHashMap::default(),
            writes: FxHashMap::default(),
            base: 1,
        }
    }

    pub fn learn(&mut self, candidate: &Candidate) {
        for read in candidate.rec.readset.iter() {
            self.reads.insert(read.clone(), candidate.ver);
        }

        for write in candidate.rec.writeset.iter() {
            self.writes.insert(write.clone(), candidate.ver);
        }
    }

    fn update_writes_and_compute_safepoint(&mut self, candidate: &Candidate) -> u64 {
        let mut safepoint = 0;
        for candidate_write in candidate.rec.writeset.iter() {
            // update safepoint for read-write intersection
            if let Some(&self_read) = self.reads.get(candidate_write) {
                if self_read > safepoint {
                    safepoint = self_read;
                }
            }

            // update safepoint for write-write intersection and learn the write
            match self.writes.entry(candidate_write.clone()) {
                Entry::Occupied(mut entry) => {
                    let self_write = entry.insert(candidate.ver);
                    if self_write > safepoint {
                        safepoint = self_write
                    }
                }
                Entry::Vacant(entry) => {
                    entry.insert(candidate.ver);
                }
            }
        }
        safepoint
    }

    pub fn assess(&mut self, candidate: &Candidate) -> Outcome {
        // if (true) {
        //     return Commit(0, Assertive)
        // }//TODO

        let mut safepoint = self.base - 1;

        // rule R1: commit write-only transactions
        if candidate.rec.readset.is_empty() {
            // update safepoint for read-write and write-write intersection, and learn the writes
            let tmp_safepoint = self.update_writes_and_compute_safepoint(&candidate);
            if tmp_safepoint > safepoint {
                safepoint = tmp_safepoint;
            }
            return Commit(safepoint, Assertive);
        }

        // rule R2: conditionally abort transactions outside the suffix
        if candidate.rec.snapshot < self.base - 1 {
            self.learn(candidate);
            return Abort(Staleness, Permissive);
        }

        // rule R3: abort on antidependency
        for candidate_read in candidate.rec.readset.iter() {
            if let Some(&self_write) = self.writes.get(candidate_read) {
                if self_write > candidate.rec.snapshot
                    && !candidate.rec.readvers.contains(&self_write)
                {
                    self.learn(candidate);
                    return Abort(Antidependency(self_write), Assertive);
                }

                // update safepoint for write-read intersection
                if self_write > safepoint {
                    safepoint = self_write;
                }
            }
        }

        // rule R4 conditionally commit

        // update safepoint for read-write and write-write intersection, and learn the writes
        let tmp_safepoint = self.update_writes_and_compute_safepoint(&candidate);
        if tmp_safepoint > safepoint {
            safepoint = tmp_safepoint;
        }

        // learn the reads
        for candidate_read in candidate.rec.readset.iter() {
            self.reads.insert(candidate_read.clone(), candidate.ver);
        }

        Commit(safepoint, Permissive)
    }

    pub fn knows(&self, candidate: &Candidate) -> bool {
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

#[cfg(test)]
mod tests {
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
}
