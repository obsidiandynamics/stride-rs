use crate::suffix::InsertError::Nonmonotonic;
use std::ops::Range;
use std::iter::Zip;
use std::slice::Iter;

#[derive(Debug, PartialEq)]
pub struct SuffixEntry {
    pub readset: Vec<String>,
    pub writeset: Vec<String>,
    pub decided: bool
}

#[derive(Debug)]
pub struct Suffix {
    base: u64,
    items: Vec<Option<SuffixEntry>>
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

impl Suffix {
    pub fn new(capacity: usize) -> Self {
        Self { base: 0, items: Vec::with_capacity(capacity) }
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
            // initialize the base offset on the first appended entry
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
        if self.base == 0 || ver < self.base as u64 {
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
}

#[cfg(test)]
mod tests;