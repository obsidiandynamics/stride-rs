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

#[derive(Debug)]
struct Frame {
    index: usize
}

#[derive(PartialEq, Debug)]
enum ExecutionResult {
    Flawless,
    Flawed,
    Deadlocked
}

impl<'a> Executor<'a> {
    fn new(model: &'a Model<'a>) -> Self {
        Executor { model, stack: vec![], depth: 0, live: FxHashSet::default() }
    }

    fn reset_live(&mut self) {
        println!("NEW RUN");
        for i in 0..self.model.actions.len() {
            self.live.insert(i);
        }
    }

    fn run(&mut self) -> ExecutionResult {
        self.reset_live();

        let mut i = 0;
        loop {
            i += 1;
            if i > 30 {
                println!("TOO MANY RUNS");
                return ExecutionResult::Flawed
            }

            if self.depth == self.stack.len() {
                print!("pushing...");
                self.stack.push(Frame { index: 0 });
            }
            println!("depth: {}, stack {:?}", self.depth, self.stack);
            let top = &self.stack[self.depth];
            if !self.live.contains(&top.index) {
                println!("  skipping {} due to join", top.index);

                // let top = &mut self.stack[self.depth];
                // loop {
                //     if top.index + 1 == self.model.actions.len() {
                //         break
                //     }
                // }
                // let top = &mut self.stack[self.depth];
                // top.index += 1;
                if top.index + 1 == self.model.actions.len() {
                    println!("    exhausted");
                    loop {
                        let mut top = &mut self.stack[self.depth];
                        top.index += 1;
                        println!("    top {:?}", top);
                        if top.index == self.model.actions.len() {
                            self.stack.remove(self.depth);
                            println!("    popped {}", self.depth);
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
                    self.reset_live();
                } else {
                    let top = &mut self.stack[self.depth];
                    top.index += 1;
                    continue
                }
            }

            let top = &self.stack[self.depth];
            let action_entry = &self.model.actions[top.index];
            println!("  running {}", action_entry.name);
            let result = (*action_entry.action)();

            match result {
                ActionResult::Ran => {
                    println!("    ran");
                    self.depth += 1;
                }
                ActionResult::Blocked => {}
                ActionResult::Joined => {
                    println!("    joined");
                    self.live.remove(&top.index);
                    if self.live.is_empty() {
                        // experimental
                        let mut top = &mut self.stack[self.depth];
                        top.index = self.model.actions.len() - 1;

                        loop {
                            let mut top = &mut self.stack[self.depth];
                            top.index += 1;
                            println!("    top {:?}", top);
                            if top.index == self.model.actions.len() {
                                self.stack.remove(self.depth);
                                println!("    popped {}", self.depth);
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
                        self.reset_live();
                    } else {
                        let mut top = &mut self.stack[self.depth];
                        self.depth += 1;
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
        assert_eq!(ExecutionResult::Flawless, executor.run());
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
        assert_eq!(ExecutionResult::Flawless, executor.run());
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
        println!("two actions");
        let run_count = RefCell::new(Counter::new());
        let mut model = Model::new();
        model.push("two_actions_0".into(), || {
            run_count.borrow_mut().add("two_actions_0");
            Joined
        });
        model.push("two_actions_1".into(), || {
            run_count.borrow_mut().add("two_actions_1");
            Joined
        });
        let mut executor = Executor::new(&model);
        let result = executor.run();
        assert_eq!(2, run_count.borrow().get("two_actions_0"));
        assert_eq!(2, run_count.borrow().get("two_actions_1"));
        assert_eq!(ExecutionResult::Flawless, result);
    }

    #[test]
    fn three_actions() {
        println!("three actions");
        let run_count = RefCell::new(Counter::new());
        let mut model = Model::new();
        model.push("three_actions_a".into(), || {
            run_count.borrow_mut().add("three_actions_a");
            Joined
        });
        model.push("three_actions_b".into(), || {
            run_count.borrow_mut().add("three_actions_b");
            Joined
        });
        model.push("three_actions_c".into(), || {
            run_count.borrow_mut().add("three_actions_c");
            Joined
        });
        let mut executor = Executor::new(&model);
        let result = executor.run();
        assert_eq!(6, run_count.borrow().get("three_actions_a"));
        assert_eq!(6, run_count.borrow().get("three_actions_b"));
        assert_eq!(6, run_count.borrow().get("three_actions_c"));
        assert_eq!(ExecutionResult::Flawless, result);
    }
}