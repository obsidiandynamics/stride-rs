use rustc_hash::FxHashMap;
use crate::{AbortReason, Candidate};
use crate::examiner::Discord::{Assertive, Permissive};
use crate::examiner::Outcome::{Commit, Abort};
use crate::AbortReason::{Antidependency, Staleness};
use std::collections::hash_map::Entry;

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
mod tests;