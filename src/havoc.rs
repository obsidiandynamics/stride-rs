use rustc_hash::FxHashSet;
use crate::havoc::ExecutionResult::{Flawless, Deadlocked};

pub struct Model<'a, S> {
    setup: Box<dyn Fn() -> S + 'a>,
    actions: Vec<ActionEntry<'a, S>>
}

pub enum ActionResult {
    Ran,
    Blocked,
    Joined,
    Panicked
}

// type Action = FnMut() -> ActionResult;

struct ActionEntry<'a, S> {
    name: String,
    action: Box<dyn Fn(&mut S) -> ActionResult + 'a>,
}

impl<'a, S> Model<'a, S> {
    pub fn new<G>(setup: G) -> Self
        where G: Fn() -> S + 'a {
        Model { setup: Box::new(setup), actions: vec![] }
    }

    fn push<F>(&mut self, name: String, action: F)
        where F: Fn(&mut S) -> ActionResult + 'a {
        self.actions.push(ActionEntry { name, action: Box::new(action) });
    }

    // fn run(&self) {
    //     for mut entry in &self.actions {
    //         let action = entry.action.deref();
    //         action();
    //     }
    // }
}

struct Executor<'a, S> {
    model: &'a Model<'a, S>,
    stack: Vec<Frame>,
    depth: usize,
    live: FxHashSet<usize> // indexes of live actions
}

#[derive(Debug)]
struct Frame {
    index: usize,
    live_snapshot: FxHashSet<usize>,
    blocked: usize
}

#[derive(PartialEq, Debug)]
enum ExecutionResult {
    Flawless,
    Flawed,
    Deadlocked
}

impl<'a, S> Executor<'a, S> {
    fn new(model: &'a Model<'a, S>) -> Self {
        Executor { model, stack: vec![], depth: 0, live: FxHashSet::default() }
    }

    fn reset_live(&mut self) {
        println!("NEW RUN---------------------");
        for i in 0..self.model.actions.len() {
            self.live.insert(i);
        }
    }

    fn unwind(&mut self) -> Option<S> {
        loop {
            let mut top = &mut self.stack[self.depth];
            loop {
                top.index += 1;
                if top.index == self.model.actions.len() || top.live_snapshot.contains(&top.index) {
                    break;
                }
            }
            println!("    top {:?}", top);
            if top.index == self.model.actions.len() {
                self.stack.remove(self.depth);
                println!("    popped {}", self.depth);
                if self.depth > 0 {
                    self.depth -= 1;
                } else {
                    return None
                }
            } else {
                break
            }
        }
        self.depth = 0;
        self.reset_live();
        Some((*self.model.setup)())
    }

    fn run(&mut self) -> ExecutionResult {
        self.reset_live();

        let mut i = 0;
        let mut state = (*self.model.setup)();
        loop {
            i += 1;
            if i > 70 {
                println!("TOO MANY RUNS");
                return ExecutionResult::Flawed
            }

            if self.depth == self.stack.len() {
                print!("pushing...");
                self.stack.push(Frame { index: 0, live_snapshot: self.live.clone(), blocked: 0 });
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
                    panic!("    exhausted");
                    // loop {
                    //     let mut top = &mut self.stack[self.depth];
                    //     top.index += 1;
                    //     println!("    top {:?}", top);
                    //     if top.index == self.model.actions.len() {
                    //         self.stack.remove(self.depth);
                    //         println!("    popped {}", self.depth);
                    //         if self.depth > 0 {
                    //             self.depth -= 1;
                    //         } else {
                    //             return Flawless
                    //         }
                    //     } else {
                    //         break
                    //     }
                    // }
                    // self.depth = 0;
                    // self.reset_live();
                    // state = (*self.model.setup)();
                } else {
                    let top = &mut self.stack[self.depth];
                    top.index += 1;
                    continue
                }
            }

            let top = &self.stack[self.depth];
            let action_entry = &self.model.actions[top.index];
            println!("  running {}", action_entry.name);
            let result = (*action_entry.action)(&mut state);

            match result {
                ActionResult::Ran => {
                    println!("    ran");
                    self.depth += 1;
                }
                ActionResult::Blocked => {
                    println!("    blocked");
                    let mut top = &mut self.stack[self.depth];
                    top.blocked += 1;

                    if top.index + 1 == self.model.actions.len() {
                        if top.blocked == self.live.len() {
                            println!("      deadlocked");
                            return Deadlocked
                        } else {
                            println!("      abandoning");
                            match self.unwind() {
                                None => return Flawless,
                                Some(s) => state = s
                            }
                        }
                    } else {
                        top.index += 1;
                    }
                    continue
                }
                ActionResult::Joined => {
                    println!("    joined");
                    self.live.remove(&top.index);
                    if self.live.is_empty() {
                        // experimental
                        // let mut top = &mut self.stack[self.depth];
                        // top.index = self.model.actions.len() - 1;

                        loop {
                            let mut top = &mut self.stack[self.depth];
                            loop {
                                top.index += 1;
                                if top.index == self.model.actions.len() || top.live_snapshot.contains(&top.index) {
                                    break;
                                }
                            }
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
                        state = (*self.model.setup)();
                    } else {
                        // let mut top = &mut self.stack[self.depth];
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
    use rustc_hash::FxHashMap;
    use std::collections::hash_map::{Entry};

    #[test]
    fn one_shot() {
        let run_count = Cell::new(0);
        let mut model = Model::new(Counter::new);
        model.push("one_shot".into(), |_| {
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
        let mut model = Model::new(Counter::new);
        model.push("two_shot".into(), |s| {
            run_count.set(run_count.get() + 1);
            match s.add("two_shot") {
                2 => ActionResult::Joined,
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
                Entry::Vacant(entry) => {
                    entry.insert(1);
                    0
                }
            }
        }

        fn reset(&mut self, name: &str) -> usize {
            self.counts.remove(name).unwrap_or(0)
        }

        fn get(&self, name: &str) -> usize {
            *self.counts.get(name).unwrap_or(&0)
        }
    }

    #[test]
    fn two_actions() {
        println!("two actions");
        let total_runs = RefCell::new(Counter::new());
        let mut model = Model::new(Counter::new);
        model.push("two_actions_a".into(), |_| {
            total_runs.borrow_mut().add("two_actions_a");
            Joined
        });
        model.push("two_actions_b".into(), |_| {
            total_runs.borrow_mut().add("two_actions_b");
            Joined
        });
        let mut executor = Executor::new(&model);
        let result = executor.run();
        assert_eq!(2, total_runs.borrow().get("two_actions_a"));
        assert_eq!(2, total_runs.borrow().get("two_actions_b"));
        assert_eq!(ExecutionResult::Flawless, result);
    }

    #[test]
    fn two_actions_conditional() {
        println!("two actions conditional");
        let total_runs = RefCell::new(Counter::new());
        let mut model = Model::new(Counter::new);
        model.push("two_actions_conditional_a".into(), |s| {
            total_runs.borrow_mut().add("two_actions_conditional_a");
            s.add("two_actions_conditional_a");
            Joined
        });
        model.push("two_actions_conditional_b".into(), |s| {
            total_runs.borrow_mut().add("two_actions_conditional_b");
            if s.add("two_actions_conditional_b") == 0 && s.get("two_actions_conditional_a") == 0 {
                return Ran
            }
            Joined
        });
        let mut executor = Executor::new(&model);
        let result = executor.run();
        assert_eq!(3, total_runs.borrow().get("two_actions_conditional_a"));
        assert_eq!(5, total_runs.borrow().get("two_actions_conditional_b"));
        assert_eq!(ExecutionResult::Flawless, result);
    }

    #[test]
    fn two_actions_by_two() {
        println!("two actions by two");
        let total_runs = RefCell::new(Counter::new());
        let mut model = Model::new(Counter::new);
        model.push("two_actions_by_two_0".into(), |_| {
            total_runs.borrow_mut().add("two_actions_by_two_0");
            Joined
        });
        model.push("two_actions_by_two_1".into(), |s| {
            total_runs.borrow_mut().add("two_actions_by_two_1");
            match s.add("two_actions_by_two_1") {
                2 => Joined,
                _ => Ran
            }
        });
        let mut executor = Executor::new(&model);
        let result = executor.run();
        assert_eq!(3, total_runs.borrow().get("two_actions_by_two_0"));
        assert_eq!(6, total_runs.borrow().get("two_actions_by_two_1"));
        assert_eq!(ExecutionResult::Flawless, result);
    }

    #[test]
    fn three_actions() {
        println!("three actions");
        let total_runs = RefCell::new(Counter::new());
        let mut model = Model::new(Counter::new);
        model.push("three_actions_a".into(), |_| {
            total_runs.borrow_mut().add("three_actions_a");
            Joined
        });
        model.push("three_actions_b".into(), |_| {
            total_runs.borrow_mut().add("three_actions_b");
            Joined
        });
        model.push("three_actions_c".into(), |_| {
            total_runs.borrow_mut().add("three_actions_c");
            Joined
        });
        let mut executor = Executor::new(&model);
        let result = executor.run();
        assert_eq!(6, total_runs.borrow().get("three_actions_a"));
        assert_eq!(6, total_runs.borrow().get("three_actions_b"));
        assert_eq!(6, total_runs.borrow().get("three_actions_c"));
        assert_eq!(ExecutionResult::Flawless, result);
    }

    #[test]
    fn three_actions_by_two() {
        println!("three actions by two");
        let total_runs = RefCell::new(Counter::new());
        let mut model = Model::new(Counter::new);
        model.push("three_actions_by_two_a".into(), |_| {
            total_runs.borrow_mut().add("three_actions_by_two_a");
            Joined
        });
        model.push("three_actions_by_two_b".into(), |_| {
            total_runs.borrow_mut().add("three_actions_by_two_b");
            Joined
        });
        model.push("three_actions_by_two_c".into(), |s| {
            total_runs.borrow_mut().add("three_actions_by_two_c");
            match s.add("three_actions_by_two_c") {
                2 => Joined,
                _ => Ran
            }
        });
        let mut executor = Executor::new(&model);
        let result = executor.run();
        assert_eq!(12, total_runs.borrow().get("three_actions_by_two_a"));
        assert_eq!(12, total_runs.borrow().get("three_actions_by_two_b"));
        assert_eq!(24, total_runs.borrow().get("three_actions_by_two_c"));
        assert_eq!(ExecutionResult::Flawless, result);
    }

    #[test]
    fn one_shot_deadlock() {
        let run_count = Cell::new(0);
        let mut model = Model::new(Counter::new);
        model.push("one_shot_deadlock".into(), |_| {
            run_count.set(run_count.get() + 1);
            ActionResult::Blocked
        });

        let mut executor = Executor::new(&model);
        assert_eq!(ExecutionResult::Deadlocked, executor.run());
        assert_eq!(1, run_count.get());
    }

    struct Lock<'a> {
        owner: Option<&'a str>
    }

    impl <'a> Lock<'a> {
        fn new() -> Self {
            Lock { owner: None }
        }

        fn lock(&mut self, owner: &'a str) -> bool {
            match self.owner {
                None => {
                    self.owner = Some(owner);
                    true
                }
                Some(existing) if existing == owner => {
                    true
                },
                Some(_) => false
            }
        }

        fn locked(&self, owner: &str) -> bool {
            match self.owner {
                Some(existing) if existing == owner => {
                    true
                },
                _ => false
            }
        }

        fn unlock(&mut self) {
            if self.owner == None {
                panic!("no lock owner");
            }
            self.owner = None
        }
    }

    #[test]
    fn two_actions_no_deadlock() {
        let mut model = Model::new(Lock::new);
        model.push("two_actions_no_deadlock_a".into(), |s| {
            if s.locked("two_actions_no_deadlock_a") {
                s.unlock();
                Joined
            } else if s.lock("two_actions_no_deadlock_a") {
                Ran
            } else {
                Blocked
            }
        });
        model.push("two_actions_no_deadlock_b".into(), |s| {
            if s.locked("two_actions_no_deadlock_b") {
                s.unlock();
                Joined
            } else if s.lock("two_actions_no_deadlock_b") {
                Ran
            } else {
                Blocked
            }
        });

        let mut executor = Executor::new(&model);
        assert_eq!(ExecutionResult::Flawless, executor.run());
    }

    #[test]
    fn two_actions_deadlock() {
        let mut model = Model::new(|| vec![Lock::new(), Lock::new()]);
        model.push("two_actions_deadlock_a".into(), |s| {
            if s[0].locked("two_actions_deadlock_a") {
                if s[1].locked("two_actions_deadlock_a") {
                    s[1].unlock();
                    s[0].unlock();
                    Joined
                } else if s[1].lock("two_actions_deadlock_a") {
                    Ran
                } else {
                    Blocked
                }
            } else if s[0].lock("two_actions_deadlock_a") {
                Ran
            } else {
                Blocked
            }
        });
        model.push("two_actions_deadlock_b".into(), |s| {
            if s[1].locked("two_actions_deadlock_b") {
                if s[0].locked("two_actions_deadlock_b") {
                    s[0].unlock();
                    s[1].unlock();
                    Joined
                } else if s[0].lock("two_actions_deadlock_b") {
                    Ran
                } else {
                    Blocked
                }
            } else if s[1].lock("two_actions_deadlock_b") {
                Ran
            } else {
                Blocked
            }
        });

        let mut executor = Executor::new(&model);
        assert_eq!(ExecutionResult::Deadlocked, executor.run());
    }
}