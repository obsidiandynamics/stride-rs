use std::hash::Hasher;

use rand::{Rng, SeedableRng};
use rustc_hash::FxHashSet;

use crate::havoc::checker::CheckResult::{Deadlock, Fail, Pass};
use crate::havoc::model::Retention::Strong;
use crate::havoc::model::{ActionResult, Call, Context, Model, Trace};
use crate::havoc::Sublevel;
use std::borrow::Cow;

#[derive(PartialEq, Debug, Eq, Hash)]
pub enum CheckResult {
    Pass(PassResult),
    Fail(FailResult),
    Deadlock(DeadlockResult),
}

impl CheckResult {
    pub fn stats(&self) -> &Stats {
        match self {
            Pass(pass) => &pass.stats,
            Fail(fail) => &fail.stats,
            Deadlock(deadlock) => &deadlock.stats
        }
    }
}

#[derive(PartialEq, Debug, Eq, Hash)]
pub struct PassResult {
    pub stats: Stats
}

#[derive(PartialEq, Debug, Eq, Hash)]
pub struct FailResult {
    pub stats: Stats,
    pub error: String,
    pub trace: Trace
}

#[derive(PartialEq, Debug, Eq, Hash)]
pub struct DeadlockResult {
    pub stats: Stats,
    pub trace: Trace
}

struct CheckContext<'a, S> {
    model: &'a Model<'a, S>,
    stack: &'a mut Vec<Frame>,
    depth: usize,
}

impl<'a, S> CheckContext<'a, S> {
    fn hash(&self) -> u64 {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        hasher.write_usize(0x517cc1b727220a95); // K from FxHasher
        for stack_index in 0..=self.depth {
            let frame = &self.stack[stack_index];
            hasher.write_usize(frame.index);
            for &rand in &frame.rands {
                hasher.write_u64(rand);
            }
        }
        hasher.finish()
    }
}

impl<S> Context for CheckContext<'_, S> {
    fn name(&self) -> &str {
        let frame = &self.stack[self.depth];
        &self.model.actions[frame.index].name
    }

    fn rand(&mut self, limit: u64) -> u64 {
        let rand =
            rand::rngs::StdRng::seed_from_u64(self.hash()).gen_range(0..limit);
        self.stack[self.depth].rands.push(rand);
        rand
    }

    fn trace(&self) -> Cow<Trace> {
        Cow::Owned(build_trace(self.stack, self.depth))
    }
}

fn build_trace(check_stack: &[Frame], depth: usize) -> Trace {
    let mut stack = Vec::with_capacity(depth);
    for stack_index in 0..=depth {
        let frame = &check_stack[stack_index];
        stack.push(Call { action: frame.index, rands: frame.rands.clone() })
    }
    Trace { calls: stack }
}

#[derive(PartialEq, Debug, Eq, Hash)]
pub struct Stats {
    pub executed: usize,  // how many schedules were executed
    pub completed: usize, // how many schedules ran to completion
    pub deepest: usize,   // the deepest traversal (number of stack elements)
    pub steps: usize,     // total number of steps undertaken (number of actions executed)
}

#[derive(Debug)]
pub struct Config {
    max_depth: usize,
    sublevel: Sublevel,
}

impl Default for Config {
    fn default() -> Self {
        Config::default()
    }
}

impl Config {
    pub fn default() -> Self {
        Config {
            max_depth: usize::MAX,
            sublevel: Sublevel::Fine,
        }
    }

    pub fn with_max_depth(mut self, max_depth: usize) -> Self {
        self.max_depth = max_depth;
        self
    }

    pub fn with_sublevel(mut self, sublevel: Sublevel) -> Self {
        self.sublevel = sublevel;
        self
    }
}

pub struct Checker<'a, S> {
    config: Config,
    stats: Stats,
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
    rands: Vec<u64>,
}

impl<'a, S> Checker<'a, S> {
    pub fn new(model: &'a Model<'a, S>) -> Self {
        Checker {
            config: Default::default(),
            stats: Stats {
                executed: 0,
                completed: 0,
                deepest: 0,
                steps: 0,
            },
            model,
            stack: Vec::with_capacity(8),
            depth: 0,
            live: Default::default(),
            strong_count: 0,
            blocked: Default::default(),
        }
    }

    pub fn config (&self) -> &Config {
        &self.config
    }

    pub fn with_config(mut self, config: Config) -> Self {
        self.set_config(config);
        self
    }

    pub fn set_config(&mut self, config: Config) {
        self.config = config;
    }

    #[inline]
    fn reset_run(&mut self) {
        let sublevel = self.config.sublevel.if_trace();
        if sublevel.allows(Sublevel::Fine) {
            log::trace!("new schedule {}", self.stats.executed);
        }
        self.stats.executed += 1;
        if self.stats.executed % 100000 == 0 {
            let num_actions = self.model.actions.len();
            let (mut sum, mut frac, divisor) = (0f64, 1f64, num_actions as f64);
            for frame in self.stack.iter() {
                frac /= divisor;
                sum += frame.index as f64 * frac;
            }
            log::debug!("progress: {:?}, {:.6}%", self.stats, sum * 100f64);
        }

        self.depth = 0;
        for i in 0..self.model.actions.len() {
            self.live.insert(i);
        }
        self.strong_count = self.model.strong_count();
    }

    #[inline]
    fn unwind(&mut self) -> Option<S> {
        let sublevel = self.config.sublevel.if_trace();

        loop {
            let top = &mut self.stack[self.depth];
            loop {
                top.index += 1;
                if top.index == self.model.actions.len()
                    || top.live_snapshot.contains(&top.index)
                        && !top.blocked_snapshot.contains(&top.index)
                {
                    break;
                }
            }
            if sublevel.allows(Sublevel::Finest) {
                log::trace!("    top {:?}", top);
            }
            if top.index == self.model.actions.len() {
                self.stack.remove(self.depth);
                if sublevel.allows(Sublevel::Finest) {
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
        self.reset_run();
        Some((*self.model.setup)())
    }

    pub fn check(mut self) -> CheckResult {
        let sublevel = self.config.sublevel.if_trace();
        if sublevel.allows(Sublevel::Fine) {
            log::trace!(
                "checking '{}' with {:?}",
                self.model.name().unwrap_or("untitled"),
                self.config
            );
        }
        self.reset_run();

        let mut state = (*self.model.setup)();
        loop {
            if self.depth == self.stack.len() {
                if sublevel.allows(Sublevel::Finest) {
                    log::trace!("pushing...");
                }
                self.stack.push(Frame {
                    index: 0,
                    live_snapshot: self.live.clone(),
                    blocked_snapshot: self.blocked.clone(),
                    rands: vec![],
                });
            }

            let top = &self.stack[self.depth];
            if sublevel.allows(Sublevel::Finest) {
                log::trace!("depth: {}, stack {:?}", self.depth, self.stack);
            }
            if !self.live.contains(&top.index) {
                if sublevel.allows(Sublevel::Finer) {
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
                if sublevel.allows(Sublevel::Finer) {
                    log::trace!("  skipping {} due to block", top.index);
                }

                if top.index + 1 == self.model.actions.len() {
                    panic!("    abandoning");
                } else {
                    let top = &mut self.stack[self.depth];
                    top.index += 1;
                    continue;
                }
            }

            let action_entry = {
                let top = &mut self.stack[self.depth];
                top.rands = vec![];
                &self.model.actions[top.index]
            };

            if sublevel.allows(Sublevel::Fine) {
                log::trace!("  running {}", action_entry.name);
            }

            let mut context = CheckContext {
                model: self.model,
                stack: &mut self.stack,
                depth: self.depth,
            };
            let result = (*action_entry.action)(&mut state, &mut context);
            let top = &mut self.stack[self.depth];

            match result {
                ActionResult::Ran => {
                    if sublevel.allows(Sublevel::Fine) {
                        log::trace!("    ran");
                    }
                    self.depth += 1;
                    self.blocked.clear();
                }
                ActionResult::Blocked => {
                    if sublevel.allows(Sublevel::Fine) {
                        log::trace!("    blocked");
                    }
                    self.blocked.insert(top.index);

                    if self.blocked.len() == self.live.len() {
                        self.stats.completed += 1;
                        self.stats.steps += self.depth;
                        if self.depth > self.stats.deepest {
                            self.stats.deepest = self.depth;
                        }
                        if sublevel.allows(Sublevel::Fine) {
                            log::trace!("      deadlocked with {:?}", self.stats);
                        }
                        return Deadlock(DeadlockResult {
                            stats: self.stats,
                            trace: match self.depth {
                                0 => Trace { calls: vec![] },
                                _ => build_trace(&self.stack, self.depth - 1)
                            }
                        });
                    } else {
                        if top.index + 1 == self.model.actions.len() {
                            if sublevel.allows(Sublevel::Finest) {
                                log::trace!("      discarded run");
                            }
                            self.stats.steps += self.depth;
                            if self.depth > self.stats.deepest {
                                self.stats.deepest = self.depth;
                            }
                            match self.unwind() {
                                None => {
                                    if sublevel.allows(Sublevel::Fine) {
                                        log::trace!(
                                            "  passed with {:?} (last run discarded)",
                                            self.stats
                                        );
                                    }
                                    return Pass(PassResult { stats: self.stats });
                                }
                                Some(s) => state = s,
                            }
                            self.blocked.clear();
                        } else {
                            top.index += 1;
                        }
                    }
                    continue;
                }
                ActionResult::Joined => {
                    if sublevel.allows(Sublevel::Fine) {
                        log::trace!("    joined");
                    }
                    self.live.remove(&top.index);
                    if action_entry.retention == Strong {
                        self.strong_count -= 1;
                    }

                    if self.strong_count == 0 {
                        if sublevel.allows(Sublevel::Finest) {
                            log::trace!("    no more strong actions");
                        }
                        self.stats.completed += 1;
                        self.stats.steps += self.depth + 1;
                        if self.depth + 1 > self.stats.deepest {
                            self.stats.deepest = self.depth + 1;
                        }
                        match self.unwind() {
                            None => {
                                if sublevel.allows(Sublevel::Fine) {
                                    log::trace!(
                                        "  passed with {:?} (last strong action joined)",
                                        self.stats
                                    );
                                }
                                return Pass(PassResult { stats: self.stats });
                            }
                            Some(s) => state = s,
                        }
                    } else {
                        self.depth += 1;
                    }
                    self.blocked.clear();
                }
                ActionResult::Breached(error) => {
                    self.stats.completed += 1;
                    self.stats.steps += self.depth + 1;
                    if self.depth + 1 > self.stats.deepest {
                        self.stats.deepest = self.depth + 1;
                    }
                    return Fail(FailResult {
                        stats: self.stats, error,
                        trace: build_trace(&self.stack, self.depth)
                    });
                }
            }
        }
    }
}

#[cfg(test)]
mod tests;
