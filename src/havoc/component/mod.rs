use std::collections::hash_map::Entry;
use rustc_hash::FxHashMap;

#[derive(Debug)]
pub struct Counter {
    counts: FxHashMap<String, i64>,
}

impl Counter {
    pub fn new() -> Self {
        Counter {
            counts: FxHashMap::default(),
        }
    }

    pub fn inc(&mut self, name: String) -> i64 {
        self.add(name, 1)
    }

    pub fn add(&mut self, name: String, amount: i64) -> i64 {
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

    pub fn reset(&mut self, name: &str) -> i64 {
        self.counts.remove(name).unwrap_or(0)
    }

    pub fn set(&mut self, name: String, value: i64) -> i64 {
        match value {
            0 => self.reset(&name),
            _ => self.counts.insert(name, value).unwrap_or(0),
        }
    }

    pub fn get(&self, name: &str) -> i64 {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counter_add() {
        let mut counter = Counter::new();
        assert_eq!(0, counter.get("test"));
        counter.inc("test".into());
        assert_eq!(1, counter.get("test"));
        counter.add("test".into(), -1);
        assert_eq!(0, counter.get("test"));
    }

    #[test]
    fn counter_set_reset() {
        let mut counter = Counter::new();
        assert_eq!(0, counter.reset("test"));
        assert_eq!(0, counter.set("test".into(), 5));
        assert_eq!(5, counter.set("test".into(), 10));
        assert_eq!(10, counter.set("test".into(), 0));
        assert_eq!(0, counter.set("test".into(), -5));
        assert_eq!(-5, counter.set("test".into(), -10));
        assert_eq!(0, counter.get("other"));
        assert_eq!(-10, counter.reset("test"));
    }

    #[test]
    fn lock() {
        let mut lock = Lock::new();
        assert!(!lock.held("me"));

        // reentrancy
        assert!(lock.lock("me".into()));
        assert!(lock.held("me"));
        assert!(lock.lock("me".into()));
        assert!(lock.held("me"));

        // exclusion
        assert!(!lock.lock("other".into()));
        assert!(lock.held("me"));

        // unlocking
        lock.unlock();
        assert!(!lock.held("me"));
    }

    #[test] #[should_panic(expected = "lock not held")]
    fn unlock_not_held() {
        let mut lock = Lock::new();
        lock.unlock();
    }
}