use crate::suffix::InsertError::Nonmonotonic;
use std::ops::Range;
use crate::suffix::DecideResult::{Lapsed, Uninitialized, Decided};
use crate::suffix::DecideError::NoSuchCandidate;

#[derive(Debug, PartialEq)]
pub struct SuffixEntry {
    pub readset: Vec<String>,
    pub writeset: Vec<String>,
    pub decided: bool
}

#[derive(Debug)]
pub struct Suffix {
    base: u64,
    items: Vec<Option<SuffixEntry>>,
    highest_decided: u64
}

impl Default for Suffix {
    fn default() -> Self {
        Self::new(0)
    }
}

#[derive(Debug, PartialEq)]
pub enum InsertError {
    Nonmonotonic
}

#[derive(Debug, PartialEq)]
pub enum DecideResult {
    Uninitialized,
    Lapsed(u64),          // the low-water mark
    Decided(u64)          // the highest decided version in the suffix
}

#[derive(Debug, PartialEq)]
pub enum DecideError {
    NoSuchCandidate
}

impl Suffix {
    pub fn new(capacity: usize) -> Self {
        Self { base: 0, items: Vec::with_capacity(capacity), highest_decided: 0 }
    }

    pub fn lwm(&self) -> Option<u64> {
        match self.base {
            0 => None,
            base => Some(base)
        }
    }

    pub fn hwm(&self) -> Option<u64> {
        match self.base {
            0 => None,
            base => Some(base + self.items.len() as u64)
        }
    }

    pub fn range(&self) -> Range<u64> {
        Range { start: self.base, end: self.base + self.items.len() as u64 }
    }

    pub fn insert(&mut self, readset: Vec<String>, writeset: Vec<String>, ver: u64) -> Result<(), InsertError> {
        if self.base == 0 {
            if ver == 0 {
                return Err(Nonmonotonic);
            }
            // initialize the base offset on the first inserted entry
            self.base = ver;
        }

        let hwm = self.base + self.items.len() as u64;
        if ver < hwm {
            return Err(Nonmonotonic);
        }

        let pad = (ver - hwm) as usize;
        self.items.reserve(pad + 1);
        for _ in (0..pad).into_iter() {
            self.items.push(None)
        }
        self.items.push(Some(SuffixEntry {
            readset,
            writeset,
            decided: false
        }));

        Ok(())
    }

    pub fn get(&self, ver: u64) -> Option<&SuffixEntry> {
        if self.base == 0 || ver < self.base {
            return None;
        }

        let hwm = self.base + self.items.len() as u64;
        if ver >= hwm {
            return None;
        }

        return match &self.items[(ver - self.base) as usize] {
            None => None,
            Some(entry) => Some(entry)
        }
    }

    pub fn decide(&mut self, ver: u64) -> Result<DecideResult, DecideError> {
        if self.base == 0 {
            return Ok(Uninitialized);
        }
        if ver < self.base {
            return Ok(Lapsed(self.base));
        }

        let index = (ver - self.base) as usize;
        if index >= self.items.len() {
            return Err(NoSuchCandidate)
        }

        match &mut self.items[index] {
            None => return Err(NoSuchCandidate),
            Some(item) => item.decided = true
        }

        if ver == self.highest_decided {
            for i in (index + 1)..self.items.len() {
                match &mut self.items[i] {
                    None => self.highest_decided += 1,
                    Some(item) => {
                        if item.decided {
                            self.highest_decided += 1;
                        } else {
                            break;
                        }
                    }
                }
            }
        }

        Ok(Decided(self.highest_decided))
    }
}

#[cfg(test)]
mod tests;