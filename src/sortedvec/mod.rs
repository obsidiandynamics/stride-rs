#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SortedVec<T: Ord> {
    items: Vec<T>
}

impl<T: Ord> Default for SortedVec<T> {
    fn default() -> Self {
        Self { items: vec![] }
    }
}

impl <T: Ord + Clone> From<Vec<T>> for SortedVec<T> {
    fn from(v: Vec<T>) -> Self {
        let mut items = v.clone();
        items.sort();
        Self { items }
    }
}

impl<T: Ord> SortedVec<T> {
    pub fn new(capacity: usize) -> Self {
        Self { items: Vec::with_capacity(capacity) }
    }

    pub fn insert(&mut self, item: T) {
        let result = self.items.binary_search(&item);
        let pos = match result {
            Ok(pos) => pos,
            Err(pos) => pos
        };
        self.items.insert(pos, item);
    }

    pub fn contains(&self, item: &T) -> bool {
        match self.items.binary_search(item) {
            Ok(_) => true,
            Err(_) => false
        }
    }
}

#[cfg(test)]
mod tests;