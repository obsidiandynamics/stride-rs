use crate::suffix::AppendSkipReason::Nonmonotonic;
use std::ops::Range;
use std::collections::VecDeque;

#[derive(Debug, PartialEq)]
pub struct RetainedEntry {
    pub readset: Vec<String>,
    pub writeset: Vec<String>,
    pub completed: bool,
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
    entries: VecDeque<Option<RetainedEntry>>,
    highest_completed: u64,
}

impl Default for Suffix {
    fn default() -> Self {
        Self::new(0)
    }
}

#[derive(Debug, PartialEq)]
pub enum AppendResult {
    Appended,
    Skipped(AppendSkipReason)
}

#[derive(Debug, PartialEq)]
pub enum AppendSkipReason {
    Nonmonotonic,
}

#[derive(Debug, PartialEq)]
pub enum CompleteResult {
    Completed(u64),  // the highest completed version in the suffix
    Skipped(CompleteSkipReason)
}

#[derive(Debug, PartialEq)]
pub enum CompleteSkipReason {
    Uninitialized,
    Lapsed(u64),     // the low-water mark
    NoSuchCandidate,
}

impl Suffix {
    pub fn new(capacity: usize) -> Self {
        Self {
            base: 0,
            entries: VecDeque::with_capacity(capacity),
            highest_completed: 0,
        }
    }

    pub fn lwm(&self) -> Option<u64> {
        match self.base {
            0 => None,
            base => Some(base),
        }
    }

    pub fn hwm(&self) -> Option<u64> {
        match self.base {
            0 => None,
            base => Some(base + self.entries.len() as u64),
        }
    }

    pub fn range(&self) -> Range<u64> {
        Range {
            start: self.base,
            end: self.base + self.entries.len() as u64,
        }
    }

    pub fn append(
        &mut self,
        readset: Vec<String>,
        writeset: Vec<String>,
        ver: u64,
    ) -> AppendResult {
        assert_ne!(0, ver, "unsupported version 0");
        if self.base == 0 {
            // initialize the base offset and highest completed on the first inserted entry
            self.base = ver;
            self.highest_completed = ver - 1;
        }

        let hwm = self.base + self.entries.len() as u64;
        if ver < hwm {
            return AppendResult::Skipped(Nonmonotonic);
        }

        let pad = (ver - hwm) as usize;
        self.entries.reserve(pad + 1);
        for _ in (0..pad).into_iter() {
            self.entries.push_back(None)
        }
        self.entries.push_back(Some(RetainedEntry {
            readset,
            writeset,
            completed: false,
        }));

        AppendResult::Appended
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
            Some(entry) => Some(entry),
        };
    }

    pub fn complete(&mut self, ver: u64) -> CompleteResult {
        if self.base == 0 {
            return CompleteResult::Skipped(CompleteSkipReason::Uninitialized);
        }
        if ver < self.base {
            return CompleteResult::Skipped(CompleteSkipReason::Lapsed(self.base));
        }

        let index = (ver - self.base) as usize;
        if index >= self.entries.len() {
            return CompleteResult::Skipped(CompleteSkipReason::NoSuchCandidate);
        }

        match &mut self.entries[index] {
            None => return CompleteResult::Skipped(CompleteSkipReason::NoSuchCandidate),
            Some(item) => item.completed = true,
        }

        if ver == self.highest_completed + 1 {
            self.highest_completed = ver;
            for i in (index + 1)..self.entries.len() {
                match &mut self.entries[i] {
                    None => self.highest_completed += 1,
                    Some(item) => {
                        if item.completed {
                            self.highest_completed += 1;
                        } else {
                            break;
                        }
                    }
                }
            }
        }

        CompleteResult::Completed(self.highest_completed)
    }

    pub fn highest_completed(&self) -> Option<u64> {
        match self.highest_completed {
            0 => None,
            highest_completed => Some(highest_completed),
        }
    }

    pub fn truncate(
        &mut self,
        min_extent: usize,
        max_extent: usize,
    ) -> Option<impl Iterator<Item = TruncatedEntry> + '_> {
        assert_ne!(self.base, 0, "uninitialized suffix");
        assert!(min_extent > 0, "invalid min_extent ({})", min_extent);
        assert!(
            max_extent >= min_extent,
            "invalid min_extent ({}), max_extent ({})",
            min_extent,
            max_extent
        );

        if self.entries.len() <= max_extent {
            return None;
        }
        let base = self.base;
        let overhang = (self.highest_completed + 1 - base) as usize;
        let num_to_truncate = std::cmp::min(self.entries.len() - min_extent, overhang);
        let drained = self.entries.drain(..num_to_truncate);
        self.base = base + num_to_truncate as u64;

        let truncated = drained
            .enumerate()
            .filter(|(_, entry)| entry.is_some())
            .map(move |(entry_index, entry)| {
                let entry = entry.unwrap();
                TruncatedEntry {
                    ver: base + entry_index as u64,
                    readset: entry.readset,
                    writeset: entry.writeset
                }
            });

        Some(truncated)
    }
}

#[cfg(test)]
mod tests;
