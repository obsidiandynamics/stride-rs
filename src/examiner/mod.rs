use rustc_hash::FxHashMap;
use crate::examiner::Discord::{Assertive, Permissive};
use crate::examiner::Outcome::{Commit, Abort};
use std::collections::hash_map::Entry;
use crate::suffix::TruncatedEntry;
use uuid::Uuid;
use crate::examiner::AbortReason::{Antidependency, Staleness};

#[derive(Debug)]
pub struct Examiner {
    reads: FxHashMap<String, u64>,
    writes: FxHashMap<String, u64>,
    base: u64,
}

#[derive(PartialEq, Debug, Clone)]
pub enum Discord {
    Permissive,
    Assertive,
}

#[derive(PartialEq, Debug, Clone)]
pub enum Outcome {
    Commit(u64, Discord),
    Abort(AbortReason, Discord),
}

impl Outcome {
    // pub fn as_commit(&self) -> Option<(u64, &Discord)> {
    //     match self {
    //         Commit(safepoint, discord) => Some((*safepoint, discord)),
    //         Abort(_, _) => None
    //     }
    // }

    pub fn is_commit(&self) -> bool {
        match self {
            Commit(_, _) => true,
            Abort(_, _) => false
        }
    }

    // pub fn as_abort(&self) -> Option<(&AbortReason, &Discord)> {
    //     match self {
    //         Commit(_, _) => None,
    //         Abort(reason, discord) => Some((reason, discord))
    //     }
    // }

    pub fn is_abort(&self) -> bool {
        match self {
            Commit(_, _) => false,
            Abort(_, _) => true
        }
    }

    pub fn discord(&self) -> &Discord {
        match self {
            Commit(_, discord) => discord,
            Abort(_, discord) => discord
        }
    }
}

#[derive(Debug, Clone)]
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

impl Record {
    pub fn compress(cpt_readvers: Vec<u64>, cpt_snapshot: u64) -> (Vec<u64>, u64) {
        if cpt_readvers.is_empty() {
            (cpt_readvers, cpt_snapshot)
        } else {
            let smallest_readver = *cpt_readvers.iter().min().unwrap();
            let snapshot = std::cmp::max(cpt_snapshot, smallest_readver);
            let readvers = cpt_readvers.into_iter().filter(|&v| v > snapshot).collect();
            (readvers, snapshot)
        }
    }
}

#[derive(PartialEq, Debug, Clone)]
pub enum AbortReason {
    Antidependency(u64),
    Staleness,
}

impl Examiner {
    pub fn new() -> Examiner {
        Examiner {
            reads: FxHashMap::default(),
            writes: FxHashMap::default(),
            base: 0,
        }
    }

    fn ensure_initialized(&mut self, ver: u64) {
        if self.base == 0 {
            self.base = ver;
        }
    }

    pub fn learn(&mut self, candidate: Candidate) {
        assert_ne!(0, candidate.ver, "unsupported version 0");
        self.ensure_initialized(candidate.ver);
        for read in candidate.rec.readset {
            self.reads.insert(read, candidate.ver);
        }

        for write in candidate.rec.writeset {
            self.writes.insert(write, candidate.ver);
        }
    }

    fn update_writes_and_compute_safepoint(&mut self, writeset: Vec<String>, ver: u64) -> u64 {
        let mut safepoint = 0;
        for candidate_write in writeset {
            // update safepoint for read-write intersection
            if let Some(&self_read) = self.reads.get(&candidate_write) {
                if self_read > safepoint {
                    safepoint = self_read;
                }
            }

            // update safepoint for write-write intersection and learn the write
            match self.writes.entry(candidate_write) {
                Entry::Occupied(mut entry) => {
                    let self_write = entry.insert(ver);
                    if self_write > safepoint {
                        safepoint = self_write
                    }
                }
                Entry::Vacant(entry) => {
                    entry.insert(ver);
                }
            }
        }
        safepoint
    }

    pub fn assess(&mut self, candidate: Candidate) -> Outcome {
        assert_ne!(0, candidate.ver, "unsupported version 0");
        self.ensure_initialized(candidate.ver);
        let mut safepoint = self.base - 1;

        // rule R1: commit write-only transactions
        if candidate.rec.readset.is_empty() {
            // update safepoint for read-write and write-write intersection, and learn the writes
            let tmp_safepoint = self.update_writes_and_compute_safepoint(candidate.rec.writeset, candidate.ver);
            if tmp_safepoint > safepoint {
                safepoint = tmp_safepoint;
            }
            return Commit(safepoint, Assertive);
        }

        // rule R2: conditionally abort transactions outside the suffix
        if candidate.rec.snapshot < self.base - 1 {
            // println!("{} VENTURING snapshot {} base {}", candidate.ver, candidate.rec.snapshot, self.base);
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
        let tmp_safepoint = self.update_writes_and_compute_safepoint(candidate.rec.writeset, candidate.ver);
        if tmp_safepoint > safepoint {
            safepoint = tmp_safepoint;
        }

        // learn the reads
        for candidate_read in candidate.rec.readset {
            self.reads.insert(candidate_read, candidate.ver);
        }

        Commit(safepoint, Permissive)
    }

    pub fn discard(&mut self, entry: TruncatedEntry) {
        assert_ne!(self.base, 0, "uninitialized examiner");
        assert!(entry.ver >= self.base, "entry.ver ({}) < self.base ({})", entry.ver, self.base);
        Self::remove_items(&mut self.reads, entry.readset, entry.ver);
        Self::remove_items(&mut self.writes, entry.writeset, entry.ver);
        self.base = entry.ver + 1;
    }

    pub fn base(&self) -> Option<u64> {
        match self.base {
            0 => None,
            base => Some(base)
        }
    }

    fn remove_items(existing_items: &mut FxHashMap<String, u64>,  items_to_remove: Vec<String>, ver_to_remove: u64) {
        for item_to_remove in items_to_remove {
            match existing_items.entry(item_to_remove) {
                Entry::Occupied(existing) => {
                    if *existing.get() == ver_to_remove {
                        existing.remove();
                    } else {
                        assert!(ver_to_remove < *existing.get(),
                                "skipped version {} while trying to remove {}", *existing.get(), ver_to_remove)
                    }
                }
                Entry::Vacant(_) => {}
            }
        }
    }
}

impl Default for Examiner {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests;