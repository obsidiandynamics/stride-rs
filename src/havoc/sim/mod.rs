use crate::havoc::Sublevel;
use crate::havoc::model::{Model, Context, ActionResult};
use crate::havoc::sim::SimResult::Flawless;
use rustc_hash::FxHashSet;
use rand::{SeedableRng, Rng};
use std::hash::Hasher;
use rand::rngs::StdRng;
use crate::havoc::model::Retention::Strong;

#[derive(PartialEq, Debug)]
pub enum SimResult {
    Flawless,
    Flawed,
    Deadlocked,
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
}

impl Context for SimContext<'_> {
    fn name(&self) -> &str {
        self.name
    }

    fn rng(&self) -> StdRng {
        let hash = hash(self.stack);
        rand::rngs::StdRng::seed_from_u64(hash)
    }
}

#[inline]
fn hash(stack: &[usize]) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    hasher.write_usize(0x517cc1b727220a95); // K from FxHasher
    for &i in stack {
        hasher.write_usize(i);
    }
    hasher.finish()
}

pub struct Sim <'a, S> {
    config: Config,
    model: &'a Model<'a, S>,
}

impl<'a, S> Sim<'a, S> {
    pub fn new(model: &'a Model<'a, S>) -> Self {
        Sim {
            config: Default::default(),
            model
        }
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
        let mut schedule = 0;
        let mut live = FxHashSet::default();
        let mut blocked = FxHashSet::default();
        let mut stack = vec![];

        loop {
            if schedule == self.config.max_schedules {
                return Flawless
            }

            if sublevel.allows(Sublevel::Fine) {
                log::trace!("new schedule {}", schedule);
            }

            for i in 0..self.model.actions.len() {
                live.insert(i);
            }
            stack.clear();
            let mut strong_count = init_strong_count;

            let mut rng = rand::rngs::StdRng::seed_from_u64(schedule as u64);
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
                    stack: &stack
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

            schedule += 1;
        }
    }
}

#[cfg(test)]
mod tests;
