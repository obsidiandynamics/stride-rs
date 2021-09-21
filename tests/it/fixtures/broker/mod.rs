use std::cell::RefCell;
use std::rc::Rc;
use std::ops::Deref;

#[derive(Debug)]
pub struct Broker<M> {
    internals: Rc<RefCell<BrokerInternals<M>>>,
}

#[derive(Debug)]
struct BrokerInternals<M> {
    messages: Vec<Rc<M>>,
    base: usize,
}

impl<M> Broker<M> {
    pub fn new(base: usize) -> Self {
        Broker {
            internals: Rc::new(RefCell::new(BrokerInternals {
                messages: vec![],
                base,
            })),
        }
    }

    pub fn stream(&self) -> Stream<M> {
        let internals = Rc::clone(&self.internals);
        let offset = internals.borrow().base;
        Stream { internals, offset }
    }
}

#[derive(Debug)]
pub struct Stream<M> {
    internals: Rc<RefCell<BrokerInternals<M>>>,
    offset: usize,
}

impl<M> Stream<M> {
    pub fn produce(&self, message: Rc<M>) {
        self.internals
            .borrow_mut()
            .messages
            .push(Rc::clone(&message));
    }

    pub fn consume(&mut self) -> Option<(usize, Rc<M>)> {
        let internals = self.internals.borrow();
        let offset = self.offset;
        match internals.messages.get(offset - internals.base) {
            None => None,
            Some(message) => {
                self.offset += 1;
                Some((offset, Rc::clone(message)))
            }
        }
    }

    pub fn find<P>(&self, predicate: P) -> Vec<(usize, Rc<M>)>
        where
            P: Fn(&M) -> bool,
    {
        let internals = &self.internals.borrow();
        let messages = &internals.messages;
        let base = internals.base;
        messages
            .iter()
            .enumerate()
            .filter(|&(_, m)| predicate(m.deref()))
            .map(|(i, m)| (i + base, Rc::clone(&m)))
            .collect()
    }

    pub fn count<P>(&self, predicate: P) -> usize
        where
            P: Fn(&M) -> bool,
    {
        let internals = &self.internals.borrow();
        let messages = &internals.messages;
        messages
            .iter()
            .enumerate()
            .filter(|&(_, m)| predicate(m.deref()))
            .count()
    }

    pub fn offset(&self) -> usize {
        self.offset
    }

    pub fn low_watermark(&self) -> usize {
        self.internals.borrow().base
    }

    pub fn high_watermark(&self) -> usize {
        let internals = self.internals.borrow();
        internals.base + internals.messages.len()
    }

    pub fn len(&self) -> usize {
        self.internals.borrow().messages.len()
    }
}

#[cfg(test)]
mod tests;