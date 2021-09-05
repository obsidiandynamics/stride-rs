pub mod component;

use crate::havoc::CheckResult::{Deadlocked, Flawless};
use crate::havoc::Retention::Strong;
use rand::rngs::StdRng;
use rand::SeedableRng;
use rustc_hash::FxHashSet;
use std::hash::Hasher;
use crate::havoc::Trace::Off;

pub struct Model<'a, S> {
    setup: Box<dyn Fn() -> S + 'a>,
    actions: Vec<ActionEntry<'a, S>>,
}

pub enum ActionResult {
    Ran,
    Blocked,
    Joined,
    Panicked,
}

#[derive(PartialEq, Debug)]
pub enum Retention {
    Strong,
    Weak,
}

struct ActionEntry<'a, S> {
    name: String,
    retention: Retention,
    action: Box<dyn Fn(&mut S, &Context<S>) -> ActionResult + 'a>,
}

impl<'a, S> Model<'a, S> {
    pub fn new<G>(setup: G) -> Self
    where
        G: Fn() -> S + 'a,
    {
        Model {
            setup: Box::new(setup),
            actions: vec![],
        }
    }

    pub fn push<F>(&mut self, name: String, retention: Retention, action: F)
    where
        F: Fn(&mut S, &Context<S>) -> ActionResult + 'a,
    {
        self.actions.push(ActionEntry {
            name,
            retention,
            action: Box::new(action),
        });
    }
}

pub struct Checker<'a, S> {
    config: Config,
    model: &'a Model<'a, S>,
    stack: Vec<Frame>,
    depth: usize,
    live: FxHashSet<usize>,    // indexes of live (non-joined) actions
    blocked: FxHashSet<usize>, // indexes of blocked actions
    strong_count: usize,
}

#[derive(Debug)]
struct Frame {
    index: usize,
    live_snapshot: FxHashSet<usize>,
    blocked_snapshot: FxHashSet<usize>,
}

#[derive(PartialEq, Debug)]
pub enum CheckResult {
    Flawless,
    Flawed,
    Deadlocked,
}

pub struct Context<'a, S> {
    name: &'a str,
    checker: &'a Checker<'a, S>,
}

impl<S> Context<'_, S> {
    pub fn name(&self) -> &str {
        self.name
    }

    pub fn rng(&self) -> StdRng {
        rand::rngs::StdRng::seed_from_u64(self.checker.hash())
    }
}

struct Initials {
    live: FxHashSet<usize>,
    strong_count: usize,
}

#[derive(Debug)]
pub struct Stats {
    schedules: usize, // how many discrete runs were performed
    deepest: usize,   // the deepest traversal (number of stack elements)
    steps: usize,     // total number of steps undertaken (number of actions executed)
}

#[derive(Debug)]
pub struct Config {
    max_depth: usize,
    trace: Trace
}

impl Default for Config {
    fn default() -> Self {
        Config::default()
    }
}

impl Config {
    pub fn default() -> Self {
        Config { max_depth: usize::MAX, trace: Trace::Fine }
    }

    pub fn with_max_depth(mut self, max_depth: usize) -> Self {
        self.max_depth = max_depth;
        self
    }

    pub fn with_trace(mut self, trace: Trace) -> Self {
        self.trace = trace;
        self
    }
}

#[derive(Copy, Clone, Debug)]
pub enum Trace {
    Off, Fine, Finer, Finest
}

impl Trace {
    #[inline]
    fn allows(&self, other: &Trace) -> bool {
        *self as usize >= *other as usize
    }

    #[inline]
    fn conditional(&self) -> Self {
        match log::log_enabled!(log::Level::Trace) {
            true => *self,
            false => Off
        }
    }
}

#[test]
fn trace_allows() {
    assert!(!Trace::Off.allows(&Trace::Fine));
    assert!(Trace::Fine.allows(&Trace::Fine));
    assert!(!Trace::Fine.allows(&Trace::Finer));
    assert!(Trace::Finer.allows(&Trace::Finer));
    assert!(Trace::Finer.allows(&Trace::Fine));
}

impl<'a, S> Checker<'a, S> {
    pub fn new(model: &'a Model<'a, S>) -> Self {
        Checker {
            config: Default::default(),
            model,
            stack: vec![],
            depth: 0,
            live: FxHashSet::default(),
            strong_count: 0,
            blocked: FxHashSet::default(),
        }
    }

    pub fn with_config(mut self, config: Config) -> Self {
        self.config(config);
        self
    }

    pub fn config(&mut self, config: Config) {
        self.config = config;
    }

    fn reset_run(&mut self) {
        //todo live and strong_count can be cached and cloned
        let trace = self.config.trace.conditional();
        if trace.allows(&Trace::Fine) {
            log::trace!("NEW RUN---------------------");
        }

        for i in 0..self.model.actions.len() {
            self.live.insert(i);
        }
        self.depth = 0;
        self.strong_count = self
            .model
            .actions
            .iter()
            .filter(|entry| entry.retention == Strong)
            .count();
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
        let trace = self.config.trace.conditional();
        loop {
            let mut top = &mut self.stack[self.depth];
            loop {
                top.index += 1;
                if top.index == self.model.actions.len()
                    || top.live_snapshot.contains(&top.index)
                    && !top.blocked_snapshot.contains(&top.index)
                {
                    break;
                }
            }
            if trace.allows(&Trace::Finest) {
                log::trace!("    top {:?}", top);
            }
            if top.index == self.model.actions.len() {
                self.stack.remove(self.depth);
                if trace.allows(&Trace::Finest) {
                    log::trace!("    popped {}", self.depth);
                }
                if self.depth > 0 {
                    self.depth -= 1;
                } else {
                    return None;
                }
            } else {
                break;
            }
        }
        let trace = self.config.trace;
        self.reset_run();
        Some((*self.model.setup)())
    }

    pub fn check(mut self) -> CheckResult {
        let trace = self.config.trace.conditional();
        self.reset_run();

        // let mut i = 0;
        let mut state = (*self.model.setup)();
        loop {
            // i += 1;
            // if i > 70 {
            //     println!("TOO MANY RUNS");
            //     return CheckResult::Flawed
            // }

            if self.depth == self.stack.len() {
                if trace.allows(&Trace::Finest) {
                    log::trace!("pushing...");
                }
                self.stack.push(Frame {
                    index: 0,
                    live_snapshot: self.live.clone(),
                    blocked_snapshot: self.blocked.clone(),
                });
            }

            if trace.allows(&Trace::Finest) {
                log::trace!("depth: {}, stack {:?}", self.depth, self.stack);
            }
            let top = &self.stack[self.depth];
            if !self.live.contains(&top.index) {
                if trace.allows(&Trace::Finer) {
                    log::trace!("  skipping {} due to join", top.index);
                }

                if top.index + 1 == self.model.actions.len() {
                    panic!("    exhausted");
                } else {
                    let top = &mut self.stack[self.depth];
                    top.index += 1;
                    continue;
                }
            }

            if self.blocked.contains(&top.index) {
                if trace.allows(&Trace::Finer) {
                    log::trace!("  skipping {} due to block", top.index);
                }

                if top.index + 1 == self.model.actions.len() {
                    panic!("    abandoning");
                    // println!("    abandoning");
                    // match self.unwind() {
                    //     None => return Flawless,
                    //     Some(s) => state = s
                    // }
                    // self.blocked.clear();
                    // continue
                } else {
                    let top = &mut self.stack[self.depth];
                    top.index += 1;
                    continue;
                }
            }

            let top = &self.stack[self.depth];
            let action_entry = &self.model.actions[top.index];
            if trace.allows(&Trace::Fine) {
                log::trace!("  running {}", action_entry.name);
            }
            let context = Context {
                name: &action_entry.name,
                checker: &self,
            };
            let result = (*action_entry.action)(&mut state, &context);

            match result {
                ActionResult::Ran => {
                    if trace.allows(&Trace::Fine) {
                        log::trace!("    ran");
                    }
                    self.depth += 1;
                    self.blocked.clear();
                }
                ActionResult::Blocked => {
                    if trace.allows(&Trace::Fine) {
                        log::trace!("    blocked");
                    }
                    self.blocked.insert(top.index);
                    // let mut top = &mut self.stack[self.depth];
                    // top.blocked += 1;

                    if self.blocked.len() == self.live.len() {
                        if trace.allows(&Trace::Fine) {
                            log::trace!("      deadlocked");
                        }
                        return Deadlocked;
                    } else {
                        // println!("      abandoning");
                        // match self.unwind() {
                        //     None => return Flawless,
                        //     Some(s) => state = s
                        // }
                        self.depth += 1;
                    }

                    // if top.index + 1 == self.model.actions.len() {
                    //     if self.blocked.len() == self.live.len() {
                    //         println!("      deadlocked");
                    //         return Deadlocked
                    //     } else {
                    //         // println!("      abandoning");
                    //         // match self.unwind() {
                    //         //     None => return Flawless,
                    //         //     Some(s) => state = s
                    //         // }
                    //         println!("      diving");
                    //         top.blocked = 0;
                    //         self.depth += 1;
                    //     }
                    // } else {
                    //     top.index += 1;
                    // }
                    continue;
                }
                ActionResult::Joined => {
                    if trace.allows(&Trace::Fine) {
                        log::trace!("    joined");
                    }
                    self.live.remove(&top.index);
                    if self.model.actions[top.index].retention == Strong {
                        self.strong_count -= 1;
                    }

                    if self.strong_count == 0 {
                        if trace.allows(&Trace::Finest) {
                            log::trace!("    no more strong actions");
                        }
                        match self.unwind() {
                            None => return Flawless,
                            Some(s) => state = s,
                        }
                    } else {
                        self.depth += 1;
                    }
                    self.blocked.clear();
                }
                ActionResult::Panicked => {}
            }
        }
    }
}

#[cfg(test)]
mod tests;
