use rustc_hash::FxHashMap;
use stride::examiner::{Outcome, Discord};
use uuid::Uuid;
use std::collections::hash_map::{Entry};
use crate::fixtures::xdb::XdbAssignmentError::Conflict;

#[derive(Debug)]
pub struct Xdb {
    pub txns: FxHashMap<Uuid, Outcome>
}

#[derive(Debug, PartialEq)]
pub enum XdbAssignmentError<'a> {
    Conflict { existing: &'a Outcome, new: &'a Outcome }
}

impl Xdb {
    pub fn new() -> Self {
        Self {
            txns: FxHashMap::default()
        }
    }

    pub fn assign<'a>(&'a mut self, xid: Uuid, outcome: &'a Outcome) -> Result<&'a Outcome, XdbAssignmentError<'a>> {
        match self.txns.entry(xid) {
            Entry::Occupied(entry) => {
                let existing = entry.into_mut();
                if (*existing.discord() == Discord::Assertive ||
                    *outcome.discord() == Discord::Assertive) &&
                    (existing.is_abort() ^ outcome.is_abort()) {
                    return Err(Conflict { existing, new: outcome })
                }
                Ok(existing)
            }
            Entry::Vacant(entry) => {
                entry.insert(outcome.clone());
                Ok(outcome)
            }
        }
    }
}

#[cfg(test)]
mod tests;