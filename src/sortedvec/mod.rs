#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SortedVec<T: Ord> {
    elements: Vec<T>
}

impl<T: Ord> Default for SortedVec<T> {
    fn default() -> Self {
        Self { elements: vec![] }
    }
}

impl <T: Ord + Clone> From<Vec<T>> for SortedVec<T> {
    fn from(v: Vec<T>) -> Self {
        let mut elements = v.clone();
        elements.sort_unstable();
        Self { elements }
    }
}

impl<T: Ord> SortedVec<T> {
    pub fn new(capacity: usize) -> Self {
        Self { elements: Vec::with_capacity(capacity) }
    }

    pub fn insert(&mut self, element: T) {
        let result = self.elements.binary_search(&element);
        let pos = match result {
            Ok(pos) => pos,
            Err(pos) => pos
        };
        self.elements.insert(pos, element);
    }
    
    pub fn remove(&mut self, element: &T) -> bool {
        match self.elements.binary_search(element) {
            Ok(pos) => {
                self.elements.remove(pos);
                true
            }
            Err(_) => false
        }
    }

    pub fn len(&self) -> usize {
        self.elements.len()
    }

    pub fn contains(&self, element: &T) -> bool {
        // if self.elements.len() > 4 {
            match self.elements.binary_search(element) {
                Ok(_) => true,
                Err(_) => false
            }
        // } else {
        //     self.elements.contains(element)
        // }
    }

    pub fn clear(&mut self) {
        self.elements.clear();
    }

    pub fn as_slice(&self) -> &[T] {
        &self.elements
    }
}

#[cfg(test)]
mod tests;