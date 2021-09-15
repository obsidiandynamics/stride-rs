use crate::suffix::InsertError::Nonmonotonic;
use std::ops::Range;
use crate::suffix::DecideResult::{Lapsed, Uninitialized, Decided};
use crate::suffix::DecideError::NoSuchCandidate;

#[derive(Debug, PartialEq)]
pub struct RetainedEntry {
    pub readset: Vec<String>,
    pub writeset: Vec<String>,
    pub decided: bool
}

#[derive(Debug, PartialEq)]
pub struct TruncatedEntry {
    pub ver: u64,
    pub readset: Vec<String>,
    pub writeset: Vec<String>,
}

#[derive(Debug)]
pub struct Suffix {
    base: u64,
    entries: Vec<Option<RetainedEntry>>,
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
        Self { base: 0, entries: Vec::with_capacity(capacity), highest_decided: 0 }
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
            base => Some(base + self.entries.len() as u64)
        }
    }

    pub fn range(&self) -> Range<u64> {
        Range { start: self.base, end: self.base + self.entries.len() as u64 }
    }

    pub fn insert(&mut self, readset: Vec<String>, writeset: Vec<String>, ver: u64) -> Result<(), InsertError> {
        assert_ne!(0, ver, "unsupported version 0");
        if self.base == 0 {
            // initialize the base offset and highest decided on the first inserted entry
            self.base = ver;
            self.highest_decided = ver - 1;
        }

        let hwm = self.base + self.entries.len() as u64;
        if ver < hwm {
            return Err(Nonmonotonic);
        }

        let pad = (ver - hwm) as usize;
        self.entries.reserve(pad + 1);
        for _ in (0..pad).into_iter() {
            self.entries.push(None)
        }
        self.entries.push(Some(RetainedEntry {
            readset,
            writeset,
            decided: false
        }));

        Ok(())
    }

    pub fn get(&self, ver: u64) -> Option<&RetainedEntry> {
        if self.base == 0 || ver < self.base {
            return None;
        }

        let hwm = self.base + self.entries.len() as u64;
        if ver >= hwm {
            return None;
        }

        return match &self.entries[(ver - self.base) as usize] {
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
        if index >= self.entries.len() {
            return Err(NoSuchCandidate)
        }

        match &mut self.entries[index] {
            None => return Err(NoSuchCandidate),
            Some(item) => item.decided = true
        }

        if ver == self.highest_decided + 1 {
            self.highest_decided = ver;
            for i in (index + 1)..self.entries.len() {
                match &mut self.entries[i] {
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

    pub fn highest_decided(&self) -> Option<u64> {
        match self.highest_decided {
            0 => None,
            highest_decided => Some(highest_decided)
        }
    }

    pub fn truncate(&mut self, min_extent: usize, max_extent: usize) -> Option<Vec<TruncatedEntry>>{
        assert_ne!(self.base, 0, "uninitialized suffix");
        assert!(min_extent > 0, "invalid min_extent ({})", min_extent);
        assert!(max_extent >= min_extent, "invalid min_extent ({}), max_extent ({})", min_extent, max_extent);
        let extent = (self.highest_decided + 1 - self.base) as usize;
        if extent <= max_extent {
            return None;
        }

        let num_to_truncate = extent - min_extent;
        let drained = self.entries.drain(..num_to_truncate);
        let mut truncated = Vec::with_capacity(num_to_truncate);
        for (entry_index, mut entry) in drained.enumerate() {
            match entry.take() {
                None => {}
                Some(entry) => {
                    truncated.push(TruncatedEntry {
                        ver: self.base + entry_index as u64,
                        readset: entry.readset,
                        writeset: entry.writeset
                    });
                }
            }
        }

        self.base += num_to_truncate as u64;
        Some(truncated)
    }
}

#[cfg(test)]
mod tests;