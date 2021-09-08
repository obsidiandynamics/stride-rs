use std::cell::{Cell, RefCell};

use rustc_hash::FxHashSet;

use crate::havoc::checker::{Checker, Config};
use crate::havoc::component::*;
use crate::havoc::model::{ActionResult, Model, name_of};
use crate::havoc::model::ActionResult::{Blocked, Joined, Ran};
use crate::havoc::model::Retention::{Strong, Weak};

use super::*;

fn init_log() {
    let _ = env_logger::builder()
        .filter_level(log::LevelFilter::Trace)
        .is_test(true)
        .try_init();
}

fn default_config() -> Config {
    Config::default().with_sublevel(Sublevel::Finest)
}

#[test]
fn dfs_one_shot() {
    init_log();
    let run_count = Cell::new(0);
    let model = Model::new(Counter::new)
        .with_name(name_of(&dfs_one_shot).into())
        .with_action("test".into(), Strong, |_, _| {
            run_count.set(run_count.get() + 1);
            ActionResult::Joined
        });

    let checker = Checker::new(&model).with_config(default_config());
    assert_eq!(Flawless, checker.check());
    assert_eq!(1, run_count.get());
}

#[test]
fn dfs_two_shots() {
    init_log();
    let run_count = Cell::new(0);
    let model = Model::new(Counter::new)
        .with_name(name_of(&dfs_two_shots).into())
        .with_action("test".into(), Strong, |s, c| {
            run_count.set(run_count.get() + 1);
            match s.inc(c.name().into()) {
                2 => ActionResult::Joined,
                _ => ActionResult::Ran,
            }
        });

    let checker = Checker::new(&model).with_config(default_config());
    assert_eq!(Flawless, checker.check());
    assert_eq!(2, run_count.get());
}

#[test]
fn dfs_two_actions() {
    init_log();
    let total_runs = RefCell::new(Counter::new());
    let model = Model::new(Counter::new)
        .with_name(name_of(&dfs_two_actions).into())
        .with_action("a".into(), Strong, |_, c| {
            total_runs.borrow_mut().inc(c.name().into());
            Joined
        })
        .with_action("b".into(), Strong, |_, c| {
            total_runs.borrow_mut().inc(c.name().into());
            Joined
        });

    let checker = Checker::new(&model).with_config(default_config());
    let result = checker.check();
    assert_eq!(Flawless, result);
    assert_eq!(2, total_runs.borrow().get("a"));
    assert_eq!(2, total_runs.borrow().get("b"));
}

#[test]
fn dfs_two_actions_conditional() {
    init_log();
    let total_runs = RefCell::new(Counter::new());
    let model = Model::new(Counter::new)
        .with_name(name_of(&dfs_two_actions_conditional).into())
        .with_action("a".into(), Strong, |s, c| {
            total_runs.borrow_mut().inc(c.name().into());
            s.inc(c.name().into());
            Joined
        })
        .with_action("b".into(), Strong, |s, c| {
            if s.inc(c.name().into()) == 0 && s.get("a") == 0 {
                return Blocked;
            }
            total_runs.borrow_mut().inc(c.name().into());
            Joined
        });

    let checker = Checker::new(&model).with_config(default_config());
    let result = checker.check();
    assert_eq!(Flawless, result);
    assert_eq!(1, total_runs.borrow().get("a"));
    assert_eq!(1, total_runs.borrow().get("b"));
}

#[test]
fn dfs_two_actions_by_two() {
    init_log();
    let total_runs = RefCell::new(Counter::new());
    let model = Model::new(Counter::new)
        .with_name(name_of(&dfs_two_actions_by_two).into())
        .with_action("a".into(), Strong, |_, c| {
            total_runs.borrow_mut().inc(c.name().into());
            Joined
        })
        .with_action("b".into(), Strong, |s, c| {
            total_runs.borrow_mut().inc(c.name().into());
            match s.inc(c.name().into()) {
                2 => Joined,
                _ => Ran,
            }
        });

    let checker = Checker::new(&model).with_config(default_config());
    let result = checker.check();
    assert_eq!(Flawless, result);
    assert_eq!(3, total_runs.borrow().get("a"));
    assert_eq!(6, total_runs.borrow().get("b"));
}

#[test]
fn dfs_three_actions() {
    init_log();
    let total_runs = RefCell::new(Counter::new());
    let model = Model::new(Counter::new)
        .with_name(name_of(&dfs_three_actions).into())
        .with_action("a".into(), Strong, |_, c| {
            total_runs.borrow_mut().inc(c.name().into());
            Joined
        })
        .with_action("b".into(), Strong, |_, c| {
            total_runs.borrow_mut().inc(c.name().into());
            Joined
        })
        .with_action("c".into(), Strong, |_, c| {
            total_runs.borrow_mut().inc(c.name().into());
            Joined
        });

    let checker = Checker::new(&model).with_config(default_config());
    let result = checker.check();
    assert_eq!(Flawless, result);
    assert_eq!(6, total_runs.borrow().get("a"));
    assert_eq!(6, total_runs.borrow().get("b"));
    assert_eq!(6, total_runs.borrow().get("c"));
}

#[test]
fn dfs_three_actions_by_two() {
    init_log();
    let total_runs = RefCell::new(Counter::new());
    let model = Model::new(Counter::new)
        .with_name(name_of(&dfs_three_actions_by_two).into())
        .with_action("a".into(), Strong, |_, c| {
            total_runs.borrow_mut().inc(c.name().into());
            Joined
        })
        .with_action("b".into(), Strong, |_, c| {
            total_runs.borrow_mut().inc(c.name().into());
            Joined
        })
        .with_action("c".into(), Strong, |s, c| {
            total_runs.borrow_mut().inc(c.name().into());
            match s.inc(c.name().into()) {
                2 => Joined,
                _ => Ran,
            }
        });

    let checker = Checker::new(&model).with_config(default_config());
    let result = checker.check();
    assert_eq!(12, total_runs.borrow().get("a"));
    assert_eq!(12, total_runs.borrow().get("b"));
    assert_eq!(24, total_runs.borrow().get("c"));
    assert_eq!(Flawless, result);
}

#[test]
fn dfs_one_shot_deadlock() {
    init_log();
    let run_count = Cell::new(0);
    let model = Model::new(Counter::new)
        .with_name(name_of(&dfs_one_shot_deadlock).into())
        .with_action("test".into(), Strong, |_, _| {
            run_count.set(run_count.get() + 1);
            ActionResult::Blocked
        });

    let checker = Checker::new(&model).with_config(default_config());
    assert_eq!(Deadlocked, checker.check());
    assert_eq!(1, run_count.get());
}

#[test]
fn dfs_two_actions_no_deadlock() {
    init_log();
    let mut model = Model::new(Lock::new).with_name(name_of(&dfs_two_actions_no_deadlock).into());
    for c in ["a", "b"] {
        model.action(
            String::from("test-".to_owned() + c),
            Strong,
            |s, c| {
                if s.held(c.name()) {
                    s.unlock();
                    Joined
                } else if s.lock(c.name().into()) {
                    Ran
                } else {
                    Blocked
                }
            },
        );
    }

    let checker = Checker::new(&model).with_config(default_config());
    assert_eq!(Flawless, checker.check());
}

#[test]
fn dfs_two_actions_deadlock() {
    init_log();
    let model = Model::new(|| [Lock::new(), Lock::new()])
        .with_name(name_of(&dfs_two_actions_deadlock).into())
        .with_action("a".into(), Strong, |s, c| {
            if s[0].held(c.name()) {
                if s[1].held(c.name()) {
                    s[1].unlock();
                    s[0].unlock();
                    Joined
                } else if s[1].lock(c.name().into()) {
                    Ran
                } else {
                    Blocked
                }
            } else if s[0].lock(c.name().into()) {
                Ran
            } else {
                Blocked
            }
        })
        .with_action("b".into(), Strong, |s, c| {
            if s[1].held(c.name()) {
                if s[0].held(c.name()) {
                    s[0].unlock();
                    s[1].unlock();
                    Joined
                } else if s[0].lock(c.name().into()) {
                    Ran
                } else {
                    Blocked
                }
            } else if s[1].lock(c.name().into()) {
                Ran
            } else {
                Blocked
            }
        });

    let checker = Checker::new(&model).with_config(default_config());
    assert_eq!(Deadlocked, checker.check());
}

#[test]
fn dfs_two_actions_one_weak_blocked() {
    init_log();
    let total_runs = RefCell::new(Counter::new());
    let model = Model::new(Counter::new)
        .with_name(name_of(&dfs_two_actions_one_weak_blocked).into())
        .with_action("a".into(), Strong, |s, c| {
            total_runs.borrow_mut().inc(c.name().into());
            s.inc(c.name().into());
            Joined
        })
        .with_action("b".into(), Weak, |s, c| {
            assert_eq!(0, s.get("a"), "b should not run after a's join");
            total_runs.borrow_mut().inc(c.name().into());
            Blocked
        });

    let checker = Checker::new(&model).with_config(default_config());
    let result = checker.check();
    assert_eq!(Flawless, result);
    assert_eq!(1, total_runs.borrow().get("a"));
    assert_eq!(1, total_runs.borrow().get("b"));
}

#[test]
fn dfs_two_actions_one_weak_two_runs() {
    init_log();
    let total_runs = RefCell::new(Counter::new());
    let model = Model::new(Counter::new)
        .with_name(name_of(&dfs_two_actions_one_weak_two_runs).into())
        .with_action("a".into(), Strong, |s, c| {
            total_runs.borrow_mut().inc(c.name().into());
            s.inc(c.name().into());
            Joined
        })
        .with_action("b".into(), Weak, |s, c| {
            assert_eq!(0, s.get("a"), "b should not run after a's join");
            total_runs.borrow_mut().inc(c.name().into());
            match s.inc(c.name().into()) {
                2 => Joined,
                _ => Ran,
            }
        });

    let checker = Checker::new(&model).with_config(default_config());
    let result = checker.check();
    assert_eq!(Flawless, result);
    assert_eq!(3, total_runs.borrow().get("a"));
    assert_eq!(3, total_runs.borrow().get("b"));
}

#[test]
fn dfs_rand() {
    init_log();
    let generated = RefCell::new(FxHashSet::default());
    const NUM_RUNS: i64 = 3;
    let model = Model::new(Counter::new)
        .with_name(name_of(&dfs_rand).into())
        .with_action("test".into(), Strong, |s, c| {
            let random_number = c.rand(u64::MAX);
            generated.borrow_mut().insert(random_number);
            match s.inc(c.name().into()) {
                NUM_RUNS => Joined,
                _ => Ran,
            }
        });
    assert_eq!(Flawless, Checker::new(&model).with_config(default_config()).check());
    assert_eq!(NUM_RUNS, generated.borrow().len() as i64);

    // repeat run should yield the same random numbers
    assert_eq!(Flawless, Checker::new(&model).with_config(default_config()).check());
    assert_eq!(NUM_RUNS, generated.borrow().len() as i64);
}
