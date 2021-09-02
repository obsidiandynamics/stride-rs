use rustc_hash::FxHashMap;
use std::collections::hash_map::Entry;

#[derive(Debug)]
pub struct Counter<'a> {
    counts: FxHashMap<&'a str, u64>,
}

impl<'a> Counter<'a> {
    pub fn new() -> Self {
        Counter {
            counts: FxHashMap::default(),
        }
    }

    pub fn inc(&mut self, name: &'a str) -> u64 {
        self.add(name, 1)
    }

    pub fn add(&mut self, name: &'a str, amount: u64) -> u64 {
        match self.counts.entry(name) {
            Entry::Occupied(mut entry) => {
                let updated = *entry.get() + amount;
                match updated {
                    0 => entry.remove(),
                    _ => entry.insert(updated),
                };
                updated
            }
            Entry::Vacant(entry) => {
                entry.insert(amount);
                0
            }
        }
    }

    pub fn reset(&mut self, name: &str) -> u64 {
        self.counts.remove(name).unwrap_or(0)
    }

    pub fn set(&mut self, name: &'a str, value: u64) -> u64 {
        match value {
            0 => self.reset(name),
            _ => self.counts.insert(name, value).unwrap_or(0),
        }
    }

    pub fn get(&self, name: &str) -> u64 {
        *self.counts.get(name).unwrap_or(&0)
    }
}

pub struct Lock<> {
    owner: Option<String>
}

impl Lock {
    pub fn new() -> Self {
        Lock { owner: None }
    }

    pub fn lock(&mut self, owner: String) -> bool {
        match &self.owner {
            None => {
                self.owner = Some(owner);
                true
            }
            Some(existing) if *existing == owner => {
                true
            },
            Some(_) => false
        }
    }

    pub fn held(&self, owner: &str) -> bool {
        match &self.owner {
            Some(existing) if existing == owner => {
                true
            },
            _ => false
        }
    }

    pub fn unlock(&mut self) {
        if self.owner == None {
            panic!("lock not held");
        }
        self.owner = None
    }
}
