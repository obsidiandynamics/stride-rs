use std::ops::{Deref, DerefMut};
use rustc_hash::FxHashSet;
use crate::havoc::ExecutionResult::Flawless;

pub struct Model<'a> {
    actions: Vec<ActionEntry<'a>>
}

pub enum ActionResult {
    Ran,
    Blocked,
    Joined,
    Panicked
}

// type Action = FnMut() -> ActionResult;

struct ActionEntry<'a> {
    name: String,
    action: Box<dyn Fn() -> ActionResult + 'a>,
}

impl<'a> Model<'a> {
    pub fn new() -> Self {
        Model { actions: vec![] }
    }

    fn push<F>(&mut self, name: String, f: F)
        where F: Fn() -> ActionResult + 'a {
        self.actions.push(ActionEntry { name, action: Box::new(f) });
    }

    // fn run(&self) {
    //     for mut entry in &self.actions {
    //         let action = entry.action.deref();
    //         action();
    //     }
    // }
}

struct Executor<'a> {
    model: &'a Model<'a>,
    stack: Vec<Frame>,
    depth: usize,
    live: FxHashSet<usize> // indexes of live actions
}

struct Frame {
    index: usize
}

enum ExecutionResult {
    Flawless,
    Flawed,
    Deadlocked
}

impl<'a> Executor<'a> {
    fn new(model: &'a Model<'a>) -> Self {
        Executor { model, stack: vec![], depth: 0, live: FxHashSet::default() }
    }

    fn run(&mut self) -> ExecutionResult {
        for i in 0..self.model.actions.len() {
            self.live.insert(i);
        }

        loop {
            if self.depth == self.stack.len() {
                self.stack.push(Frame { index: 0 });
            }
            let top = &self.stack[self.depth];
            let action_entry = &self.model.actions[top.index];
            let result = (*action_entry.action)();

            match result {
                ActionResult::Ran => {
                    self.depth += 1;
                }
                ActionResult::Blocked => {}
                ActionResult::Joined => {
                    self.live.remove(&top.index);
                    if self.live.is_empty() {
                       loop {
                            let mut top = &mut self.stack[self.depth];
                            top.index += 1;
                            if top.index == self.model.actions.len() {
                                self.stack.remove(self.depth);
                                if self.depth > 0 {
                                    self.depth -= 1;
                                } else {
                                    return Flawless
                                }
                            } else {
                                break
                            }
                        }
                        self.depth = 0;
                    }
                }
                ActionResult::Panicked => {}
            }

            // let mut model = &mut self.model;
            // let mut actions =  &mut model.actions;
            // let action = &mut model.actions[top.index];
            // action.action.deref_mut()();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::havoc::ActionResult::*;
    use std::cell::{Cell, RefCell};
    use std::borrow::BorrowMut;
    use std::rc::Rc;
    use rustc_hash::FxHashMap;
    use std::collections::hash_map::{Entry, OccupiedEntry};

    #[test]
    fn one_shot() {
        let run_count = Cell::new(0);
        let mut model = Model::new();
        model.push("one_shot".into(), || {
            run_count.set(run_count.get() + 1);
            ActionResult::Joined
        });

        let mut executor = Executor::new(&model);
        executor.run();
        assert_eq!(1, run_count.get());
    }

    #[test]
    fn two_shot() {
        let run_count = Cell::new(0);
        let mut model = Model::new();
        model.push("two_shot".into(), || {
            let prev_run_count = run_count.get();
            run_count.set(prev_run_count + 1);
            match prev_run_count {
                1 => ActionResult::Joined,
                _ => ActionResult::Ran
            }
        });

        let mut executor = Executor::new(&model);
        executor.run();
        assert_eq!(2, run_count.get());
    }

    #[derive(Debug)]
    struct Counter<'a> {
        counts: FxHashMap<&'a str, usize>
    }

    impl<'a> Counter<'a> {
        fn new() -> Self {
            Counter { counts: FxHashMap::default() }
        }

        fn add(&mut self, name: &'a str) -> usize {
            match self.counts.entry(name) {
                Entry::Occupied(mut entry) => {
                    entry.insert(entry.get() + 1);
                    *entry.get()
                },
                Entry::Vacant(mut entry) => {
                    entry.insert(1);
                    0
                }
            }
        }

        fn get(&self, name: &str) -> usize {
            *self.counts.get(name).unwrap_or(&0)
        }
    }

    #[test]
    fn two_actions() {
        let run_count = RefCell::new(Counter::new());
        let mut model = Model::new();
        model.push("two_actions_0".into(), || {
            run_count.borrow_mut().add("two_actions_0");
            Joined
        });
        Executor::new(&model).run();
        assert_eq!(1, run_count.borrow_mut().get("two_actions_0"));
    }
}