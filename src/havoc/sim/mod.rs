use std::hash::Hasher;

use rand::{Rng, SeedableRng};
use rustc_hash::FxHashSet;

use crate::havoc::model::{ActionResult, Context, Model, Trace, Call};
use crate::havoc::model::Retention::Strong;
use crate::havoc::sim::SimResult::{Deadlock, Pass, Fail};
use crate::havoc::Sublevel;
use ActionResult::{Breached, Joined, Blocked, Ran};
use std::borrow::Cow;

#[derive(PartialEq, Debug, Eq, Hash)]
pub enum SimResult {
    Pass,
    Fail(FailResult),
    Deadlock(DeadlockResult),
}

#[derive(PartialEq, Debug, Eq, Hash)]
pub struct FailResult {
    pub error: String,
    pub trace: Trace,
    pub schedule: usize
}

#[derive(PartialEq, Debug, Eq, Hash)]
pub struct DeadlockResult {
    pub trace: Trace,
    pub schedule: usize
}

#[derive(Debug)]
pub struct Stats {
    completed: usize, // how many schedules ran to completion
    deepest: usize,   // the deepest traversal (number of stack elements)
    steps: usize,     // total number of steps undertaken (number of actions executed)
}

#[derive(Debug)]
pub struct Config {
    max_depth: usize,
    max_schedules: usize,
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
            max_schedules: 1,
            sublevel: Sublevel::Fine,
        }
    }

    pub fn with_max_depth(mut self, max_depth: usize) -> Self {
        self.max_depth = max_depth;
        self
    }

    pub fn with_max_schedules(mut self, max_schedules: usize) -> Self {
        self.max_schedules = max_schedules;
        self
    }

    pub fn with_sublevel(mut self, sublevel: Sublevel) -> Self {
        self.sublevel = sublevel;
        self
    }
}

struct SimContext<'a, S> {
    model: &'a Model<'a, S>,
    trace: &'a mut Trace,
    schedule: usize
}

impl<'a, S> SimContext<'a, S> {
    fn hash(&self) -> u64 {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        hasher.write_usize(0x517cc1b727220a95); // K from FxHasher
        for call in &self.trace.calls {
            hasher.write_usize(call.action);
            for &rand in &call.rands {
                hasher.write_u64(rand);
            }
        }
        hasher.write_usize(self.schedule);
        hasher.finish()
    }
}

impl<S> Context for SimContext<'_, S> {
    fn name(&self) -> &str {
        &self.model.actions[self.trace.peek().action].name
    }

    fn rand(&mut self, limit:  u64) -> u64 {
        let hash = self.hash();
        let rand = rand::rngs::StdRng::seed_from_u64(hash).gen_range(0..limit);
        self.trace.push_rand(rand);
        rand
    }

    fn trace(&self) -> Cow<Trace> {
        Cow::Borrowed(&self.trace)
    }
}

pub struct Sim <'a, S> {
    config: Config,
    model: &'a Model<'a, S>,
    seed: u64,
}

impl<'a, S> Sim<'a, S> {
    pub fn new(model: &'a Model<'a, S>) -> Self {
        Sim {
            config: Default::default(),
            model,
            seed: 0
        }
    }

    pub fn seed(&self) -> u64 {
        self.seed
    }

    pub fn with_seed(mut self, seed: u64) -> Self {
        self.set_seed(seed);
        self
    }

    pub fn set_seed(&mut self, seed: u64) {
        self.seed = seed;
    }

    pub fn config(&self) -> &Config {
        &self.config
    }

    pub fn with_config(mut self, config: Config) -> Self {
        self.set_config(config);
        self
    }

    pub fn set_config(&mut self, config: Config) {
        self.config = config;
    }

    pub fn check(&self) -> SimResult {
        let sublevel = self.config.sublevel.if_trace();
        if sublevel.allows(Sublevel::Fine) {
            log::trace!("checking '{}' with {:?} (seed: {})", self.model.name().unwrap_or("untitled"), self.config, self.seed);
        }
        let init_strong_count = self.model.strong_count();
        let mut live = FxHashSet::default();
        let mut blocked = FxHashSet::default();
        let mut trace = Trace::new();
        let mut stats = Stats {
            completed: 0,
            deepest: 0,
            steps: 0
        };

        loop {
            if stats.completed == self.config.max_schedules {
                if sublevel.allows(Sublevel::Fine) {
                    log::trace!("  passed with {:?}", stats);
                }
                return Pass
            }

            if sublevel.allows(Sublevel::Fine) {
                log::trace!("new schedule {} (seed: {})", stats.completed, self.seed);
            }

            for i in 0..self.model.actions.len() {
                live.insert(i);
            }
            trace.calls.clear();
            let mut strong_count = init_strong_count;

            let mut rng = rand::rngs::StdRng::seed_from_u64(stats.completed as u64 + self.seed);
            let mut state = (*self.model.setup)();

            loop {
                let action_index = rng.gen_range(0..self.model.actions.len());

                if !live.contains(&action_index) {
                    if sublevel.allows(Sublevel::Finer) {
                        log::trace!("  skipping {} due to join", action_index);
                    }
                    continue;
                }

                if blocked.contains(&action_index) {
                    if sublevel.allows(Sublevel::Finer) {
                        log::trace!("  skipping {} due to block", action_index);
                    }
                    continue;
                }

                trace.calls.push(Call { action: action_index, rands: vec![] });
                let action_entry = &self.model.actions[action_index];
                if sublevel.allows(Sublevel::Fine) {
                    log::trace!("  running {}", action_entry.name);
                }
                let mut context = SimContext {
                    model: &self.model,
                    trace: &mut trace,
                    schedule: stats.completed
                };

                let result = (*action_entry.action)(&mut state, &mut context);
                match result {
                    Ran => {
                        if sublevel.allows(Sublevel::Fine) {
                            log::trace!("    ran");
                        }
                        blocked.clear();
                    }
                    Blocked => {
                        if sublevel.allows(Sublevel::Fine) {
                            log::trace!("    blocked");
                        }
                        trace.pop();
                        blocked.insert(action_index);
                        if blocked.len() == live.len() {
                            if sublevel.allows(Sublevel::Fine) {
                                log::trace!("      deadlocked with {:?}", stats);
                            }
                            return Deadlock(DeadlockResult {
                                trace: trace.clone(),
                                schedule: stats.completed
                            });
                        }
                    }
                    Joined => {
                        if sublevel.allows(Sublevel::Fine) {
                            log::trace!("    joined");
                        }
                        blocked.clear();

                        if action_entry.retention == Strong {
                            strong_count -= 1;
                        }

                        if strong_count == 0 {
                            if sublevel.allows(Sublevel::Finest) {
                                log::trace!("    no more strong actions, {:?}", trace);
                            }
                            break;
                        }
                        live.remove(&action_index);
                    }
                    Breached(error) => {
                        if sublevel.allows(Sublevel::Fine) {
                            log::trace!("      invariant breached: {}", error);
                        }
                        return Fail(FailResult {
                            error,
                            trace: trace.clone(),
                            schedule: stats.completed
                        })
                    }
                }
            }

            stats.completed += 1;
            let depth = trace.calls.len();
            if depth > stats.deepest {
                stats.deepest = depth;
            }
            stats.steps += depth;

            if stats.completed % 10000 == 0 {
                log::debug!("progress: {:?}, {:.6}%", stats, stats.completed as f64 / self.config.max_schedules as f64 * 100f64);
            }
        }
    }
}

#[cfg(test)]
mod tests;
