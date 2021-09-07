use std::hash::Hasher;

use rand::{Rng, RngCore, SeedableRng};
use rustc_hash::FxHashSet;

use crate::havoc::model::{ActionResult, Context, Model};
use crate::havoc::model::Retention::Strong;
use crate::havoc::sim::SimResult::{Deadlocked, Flawless};
use crate::havoc::Sublevel;

#[derive(PartialEq, Debug, Eq, Hash)]
pub enum SimResult {
    Flawless,
    Flawed,
    Deadlocked,
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

struct SimContext<'a> {
    name: &'a str,
    stack: &'a [usize],
    schedule: usize
}

impl Context for SimContext<'_> {
    fn name(&self) -> &str {
        self.name
    }

    fn rand(&self) -> u64 {
        let hash = hash(self.stack, self.schedule);
        rand::rngs::StdRng::seed_from_u64(hash).next_u64()
    }
}

#[inline]
fn hash(stack: &[usize], schedule: usize) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    hasher.write_usize(0x517cc1b727220a95); // K from FxHasher
    for &i in stack {
        hasher.write_usize(i);
    }
    hasher.write_usize(schedule);
    hasher.finish()
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

    pub fn with_seed(mut self, seed: u64) -> Self {
        self.seed(seed);
        self
    }

    pub fn seed(&mut self, seed: u64) {
        self.seed = seed;
    }

    pub fn with_config(mut self, config: Config) -> Self {
        self.config(config);
        self
    }

    pub fn config(&mut self, config: Config) {
        self.config = config;
    }

    pub fn check(&self) -> SimResult {
        let sublevel = self.config.sublevel.if_trace();
        if sublevel.allows(Sublevel::Fine) {
            let model_name = match &self.model.name {
                None => "untitled",
                Some(name) => &name
            };
            log::trace!("checking '{}' with {:?}", model_name, self.config);
        }
        let init_strong_count = self.model.strong_count();
        let mut live = FxHashSet::default();
        let mut blocked = FxHashSet::default();
        let mut stack = vec![];
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
                return Flawless
            }

            if sublevel.allows(Sublevel::Fine) {
                log::trace!("new schedule {} (seed: {})", stats.completed, self.seed);
            }

            for i in 0..self.model.actions.len() {
                live.insert(i);
            }
            stack.clear();
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

                stack.push(action_index);
                let action_entry = &self.model.actions[action_index];
                if sublevel.allows(Sublevel::Fine) {
                    log::trace!("  running {}", action_entry.name);
                }
                let context = SimContext {
                    name: &action_entry.name,
                    stack: &stack,
                    schedule: stats.completed
                };
                let result = (*action_entry.action)(&mut state, &context);
                match result {
                    ActionResult::Ran => {
                        if sublevel.allows(Sublevel::Fine) {
                            log::trace!("    ran");
                        }
                        blocked.clear();
                    }
                    ActionResult::Blocked => {
                        if sublevel.allows(Sublevel::Fine) {
                            log::trace!("    blocked");
                        }
                        stack.remove(stack.len() - 1);
                        blocked.insert(action_index);
                        if blocked.len() == live.len() {
                            if sublevel.allows(Sublevel::Fine) {
                                log::trace!("      deadlocked with {:?}", stats);
                            }
                            return Deadlocked;
                        }
                    }
                    ActionResult::Joined => {
                        if sublevel.allows(Sublevel::Fine) {
                            log::trace!("    joined");
                        }
                        blocked.clear();

                        if action_entry.retention == Strong {
                            strong_count -= 1;
                        }

                        if strong_count == 0 {
                            if sublevel.allows(Sublevel::Finest) {
                                log::trace!("    no more strong actions");
                            }
                            break;
                        }
                        live.remove(&action_index);
                    }
                    ActionResult::Panicked => {
                        todo!();
                    }
                }
            }

            stats.completed += 1;
            let depth = stack.len();
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
