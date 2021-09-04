use rustc_hash::{FxHashSet};
use crate::havoc::CheckResult::{Flawless, Deadlocked};
use crate::havoc::Retention::Strong;
use std::hash::{Hasher};
use rand::rngs::StdRng;
use rand::SeedableRng;

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

#[derive(PartialEq, Debug)]
pub enum Retention {
    Strong, Weak
}

struct ActionEntry<'a, S> {
    name: String,
    retention: Retention,
    action: Box<dyn Fn(&mut S, &Context<S>) -> ActionResult + 'a>,
}

impl<'a, S> Model<'a, S> {
    pub fn new<G>(setup: G) -> Self
        where G: Fn() -> S + 'a {
        Model { setup: Box::new(setup), actions: vec![] }
    }

    pub fn push<F>(&mut self, name: String, retention: Retention, action: F)
        where F: Fn(&mut S, &Context<S>) -> ActionResult + 'a {
        self.actions.push(ActionEntry { name, retention, action: Box::new(action) });
    }
}

pub struct Checker<'a, S> {
    model: &'a Model<'a, S>,
    stack: Vec<Frame>,
    depth: usize,
    live: FxHashSet<usize>, // indexes of live actions
    strong_count: usize
}

#[derive(Debug)]
struct Frame {
    index: usize,
    live_snapshot: FxHashSet<usize>,
    blocked: usize
}

#[derive(PartialEq, Debug)]
pub enum CheckResult {
    Flawless,
    Flawed,
    Deadlocked
}

pub struct Context<'a, S> {
    name: &'a str,
    checker: &'a Checker<'a, S>
}

impl<S> Context<'_, S> {
    pub fn name(&self) -> &str {
        self.name
    }

    pub fn rng(&self) -> StdRng {
        // let r = rand::thread_rng();
        let rng = rand::rngs::StdRng::seed_from_u64(self.checker.hash());
        rng
    }
}

impl<'a, S> Checker<'a, S> {
    pub fn new(model: &'a Model<'a, S>) -> Self {
        Checker { model, stack: vec![], depth: 0, live: FxHashSet::default(), strong_count: 0 }
    }

    fn reset_live(&mut self) {
        //todo live and strong_count can be cached and cloned
        println!("NEW RUN---------------------");
        for i in 0..self.model.actions.len() {
            self.live.insert(i);
        }
        self.depth = 0;
        self.strong_count = self.model.actions
            .iter().filter(|entry| entry.retention == Strong).count();
    }

    fn hash(&self) -> u64 {
        // let mut hasher = FxHasher::default();
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        // println!("depth {}", self.depth);
        hasher.write_usize(0x517cc1b727220a95); // K from FxHasher
        for i in 0..=self.depth {
            hasher.write_usize(self.stack[i].index);
        }
        let hash = hasher.finish();
        // println!("hash {}", hash);
        hash
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
        self.reset_live();
        Some((*self.model.setup)())
    }

    pub fn check(mut self) -> CheckResult {
        self.reset_live();

        let mut i = 0;
        let mut state = (*self.model.setup)();
        loop {
            i += 1;
            if i > 70 {
                println!("TOO MANY RUNS");
                return CheckResult::Flawed
            }

            if self.depth == self.stack.len() {
                print!("pushing...");
                self.stack.push(Frame { index: 0, live_snapshot: self.live.clone(), blocked: 0 });
            }
            println!("depth: {}, stack {:?}", self.depth, self.stack);
            let top = &self.stack[self.depth];
            if !self.live.contains(&top.index) {
                println!("  skipping {} due to join", top.index);

                if top.index + 1 == self.model.actions.len() {
                    panic!("    exhausted");
                } else {
                    let top = &mut self.stack[self.depth];
                    top.index += 1;
                    continue
                }
            }

            let top = &self.stack[self.depth];
            let action_entry = &self.model.actions[top.index];
            println!("  running {}", action_entry.name);
            let context = Context { name: &action_entry.name, checker: &self };
            let result = (*action_entry.action)(&mut state, &context);

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
                    if self.model.actions[top.index].retention == Strong {
                        self.strong_count -= 1;
                    }

                    if self.strong_count == 0 {
                        println!("    no more strong actions");
                        match self.unwind() {
                            None => return Flawless,
                            Some(s) => state = s
                        }
                    } else {
                        self.depth += 1;
                    }
                }
                ActionResult::Panicked => {}
            }
        }
    }
}

#[cfg(test)]
mod tests;