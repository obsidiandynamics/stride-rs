use rustc_hash::FxHashMap;
use stride::examiner::{Outcome, Discord};
use uuid::Uuid;
use std::collections::hash_map::{Entry};
use crate::fixtures::xdb::XdbAssignmentError::Conflict;
use crate::fixtures::xdb::Redaction::{Existing, New};

#[derive(Debug)]
pub struct Xdb {
    pub txns: FxHashMap<Uuid, Outcome>
}

#[derive(Clone, Debug, PartialEq)]
pub enum XdbAssignmentError<'a> {
    Conflict { existing: &'a Outcome, new: &'a Outcome }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Redaction<'a> {
    New(&'a Outcome),
    Existing(&'a Outcome)
}

impl Xdb {
    pub fn new() -> Self {
        Self {
            txns: FxHashMap::default()
        }
    }

    pub fn assign<'a>(&'a mut self, xid: Uuid, new: &'a Outcome) -> Result<Redaction<'a>, XdbAssignmentError<'a>> {
        match self.txns.entry(xid) {
            Entry::Occupied(entry) => {
                let existing = entry.into_mut();
                let existing_assertive = *existing.discord() == Discord::Assertive;
                let new_assertive = *new.discord() == Discord::Assertive;
                if (existing_assertive || new_assertive) && (existing.is_abort() ^ new.is_abort()) {
                    return Err(Conflict { existing, new })
                }

                if !existing_assertive && new_assertive {
                    // upgrade a permissive discord to an assertive one
                    *existing = new.clone();
                }

                Ok(Existing(existing))
            }
            Entry::Vacant(entry) => {
                entry.insert(new.clone());
                Ok(New(new))
            }
        }
    }

    pub fn get(&self, xid: &Uuid) -> Option<&Outcome> {
        self.txns.get(xid)
    }
}

impl Default for Xdb {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests;