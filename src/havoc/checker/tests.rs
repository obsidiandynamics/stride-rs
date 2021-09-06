use std::cell::{Cell, RefCell};

use rand::Rng;
use rustc_hash::FxHashSet;

use crate::havoc::checker::{Checker, CheckResult, Config};
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

#[test]
fn one_shot() {
    init_log();
    let run_count = Cell::new(0);
    let model = Model::new(Counter::new)
        .with_name(name_of(&one_shot).into())
        .with_action("one_shot".into(), Strong, |_, _| {
            run_count.set(run_count.get() + 1);
            ActionResult::Joined
        });

    let checker = Checker::new(&model).with_config(Config::default().with_trace(Trace::Finest));
    assert_eq!(CheckResult::Flawless, checker.check());
    assert_eq!(1, run_count.get());
}

#[test]
fn two_shot() {
    init_log();
    let run_count = Cell::new(0);
    let model = Model::new(Counter::new)
        .with_name(name_of(&two_shot).into())
        .with_action("two_shot".into(), Strong, |s, c| {
            run_count.set(run_count.get() + 1);
            match s.inc(c.name().into()) {
                2 => ActionResult::Joined,
                _ => ActionResult::Ran,
            }
        });

    let checker = Checker::new(&model).with_config(Config::default().with_trace(Trace::Finest));
    assert_eq!(CheckResult::Flawless, checker.check());
    assert_eq!(2, run_count.get());
}

#[test]
fn two_actions() {
    init_log();
    let total_runs = RefCell::new(Counter::new());
    let model = Model::new(Counter::new)
        .with_name(name_of(&two_actions).into())
        .with_action("a".into(), Strong, |_, c| {
            total_runs.borrow_mut().inc(c.name().into());
            Joined
        })
        .with_action("b".into(), Strong, |_, c| {
            total_runs.borrow_mut().inc(c.name().into());
            Joined
        });
    let checker = Checker::new(&model).with_config(Config::default().with_trace(Trace::Finest));
    let result = checker.check();
    assert_eq!(2, total_runs.borrow().get("a"));
    assert_eq!(2, total_runs.borrow().get("b"));
    assert_eq!(CheckResult::Flawless, result);
}

#[test]
fn two_actions_conditional() {
    init_log();
    let total_runs = RefCell::new(Counter::new());
    let model = Model::new(Counter::new)
        .with_action("two_actions_conditional_a".into(), Strong, |s, c| {
            total_runs.borrow_mut().inc(c.name().into());
            s.inc(c.name().into());
            Joined
        })
        .with_action("two_actions_conditional_b".into(), Strong, |s, c| {
            total_runs.borrow_mut().inc(c.name().into());
            if s.inc(c.name().into()) == 0 && s.get("two_actions_conditional_a") == 0 {
                return Ran;
            }
            Joined
        });
    let checker = Checker::new(&model).with_config(Config::default().with_trace(Trace::Finest));
    let result = checker.check();
    assert_eq!(3, total_runs.borrow().get("two_actions_conditional_a"));
    assert_eq!(5, total_runs.borrow().get("two_actions_conditional_b"));
    assert_eq!(CheckResult::Flawless, result);
}

#[test]
fn two_actions_by_two() {
    init_log();
    let total_runs = RefCell::new(Counter::new());
    let model = Model::new(Counter::new)
        .with_action("two_actions_by_two_0".into(), Strong, |_, c| {
            total_runs.borrow_mut().inc(c.name().into());
            Joined
        })
        .with_action("two_actions_by_two_1".into(), Strong, |s, c| {
            total_runs.borrow_mut().inc(c.name().into());
            match s.inc(c.name().into()) {
                2 => Joined,
                _ => Ran,
            }
        });
    let checker = Checker::new(&model);
    let result = checker.check();
    assert_eq!(3, total_runs.borrow().get("two_actions_by_two_0"));
    assert_eq!(6, total_runs.borrow().get("two_actions_by_two_1"));
    assert_eq!(CheckResult::Flawless, result);
}

#[test]
fn three_actions() {
    init_log();
    let total_runs = RefCell::new(Counter::new());
    let model = Model::new(Counter::new)
        .with_action("three_actions_a".into(), Strong, |_, c| {
            total_runs.borrow_mut().inc(c.name().into());
            Joined
        })
        .with_action("three_actions_b".into(), Strong, |_, c| {
            total_runs.borrow_mut().inc(c.name().into());
            Joined
        })
        .with_action("three_actions_c".into(), Strong, |_, c| {
            total_runs.borrow_mut().inc(c.name().into());
            Joined
        });
    let checker = Checker::new(&model);
    let result = checker.check();
    assert_eq!(6, total_runs.borrow().get("three_actions_a"));
    assert_eq!(6, total_runs.borrow().get("three_actions_b"));
    assert_eq!(6, total_runs.borrow().get("three_actions_c"));
    assert_eq!(CheckResult::Flawless, result);
}

#[test]
fn three_actions_by_two() {
    init_log();
    let total_runs = RefCell::new(Counter::new());
    let model = Model::new(Counter::new)
        .with_action("three_actions_by_two_a".into(), Strong, |_, c| {
            total_runs.borrow_mut().inc(c.name().into());
            Joined
        })
        .with_action("three_actions_by_two_b".into(), Strong, |_, c| {
            total_runs.borrow_mut().inc(c.name().into());
            Joined
        })
        .with_action("three_actions_by_two_c".into(), Strong, |s, c| {
            total_runs.borrow_mut().inc(c.name().into());
            match s.inc(c.name().into()) {
                2 => Joined,
                _ => Ran,
            }
        });
    let checker = Checker::new(&model);
    let result = checker.check();
    assert_eq!(12, total_runs.borrow().get("three_actions_by_two_a"));
    assert_eq!(12, total_runs.borrow().get("three_actions_by_two_b"));
    assert_eq!(24, total_runs.borrow().get("three_actions_by_two_c"));
    assert_eq!(CheckResult::Flawless, result);
}

#[test]
fn one_shot_deadlock() {
    init_log();
    let run_count = Cell::new(0);
    let model = Model::new(Counter::new).with_action("one_shot_deadlock".into(), Strong, |_, _| {
        run_count.set(run_count.get() + 1);
        ActionResult::Blocked
    });

    let checker = Checker::new(&model);
    assert_eq!(CheckResult::Deadlocked, checker.check());
    assert_eq!(1, run_count.get());
}

#[test]
fn two_actions_no_deadlock() {
    init_log();
    let mut model = Model::new(Lock::new);
    for c in ["a", "b"] {
        model.action(
            String::from("two_actions_no_deadlock_".to_owned() + c),
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

    let checker = Checker::new(&model);
    assert_eq!(CheckResult::Flawless, checker.check());
}

#[test]
fn two_actions_deadlock() {
    init_log();
    let model = Model::new(|| [Lock::new(), Lock::new()])
        .with_name(name_of(&two_actions_deadlock).into())
        .with_action("deadlock-a".into(), Strong, |s, c| {
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
        .with_action("deadlock-b".into(), Strong, |s, c| {
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

    let checker = Checker::new(&model).with_config(Config::default().with_trace(Trace::Finest));
    assert_eq!(CheckResult::Deadlocked, checker.check());
}

#[test]
fn two_actions_one_weak_blocked() {
    init_log();
    let total_runs = RefCell::new(Counter::new());
    let model = Model::new(Counter::new)
        .with_action("two_actions_one_weak_a".into(), Strong, |s, c| {
            total_runs.borrow_mut().inc(c.name().into());
            s.inc(c.name().into());
            Joined
        })
        .with_action("two_actions_one_weak_b".into(), Weak, |s, c| {
            assert_eq!(
                0,
                s.get("two_actions_one_weak_a"),
                "b should not run after a's join"
            );
            total_runs.borrow_mut().inc(c.name().into());
            Blocked
        });
    let checker = Checker::new(&model);
    let result = checker.check();
    // assert_eq!(2, total_runs.borrow().get("two_actions_one_weak_a"));
    assert_eq!(1, total_runs.borrow().get("two_actions_one_weak_a"));
    assert_eq!(1, total_runs.borrow().get("two_actions_one_weak_b"));
    assert_eq!(CheckResult::Flawless, result);
}

#[test]
fn two_actions_one_weak_two_runs() {
    init_log();
    let total_runs = RefCell::new(Counter::new());
    let model = Model::new(Counter::new)
        .with_action("two_actions_one_weak_a".into(), Strong, |s, c| {
            total_runs.borrow_mut().inc(c.name().into());
            s.inc(c.name().into());
            Joined
        })
        .with_action("two_actions_one_weak_b".into(), Weak, |s, c| {
            assert_eq!(
                0,
                s.get("two_actions_one_weak_a"),
                "b should not run after a's join"
            );
            total_runs.borrow_mut().inc(c.name().into());
            match s.inc(c.name().into()) {
                2 => Joined,
                _ => Ran,
            }
        });
    let checker = Checker::new(&model);
    let result = checker.check();
    assert_eq!(3, total_runs.borrow().get("two_actions_one_weak_a"));
    assert_eq!(3, total_runs.borrow().get("two_actions_one_weak_b"));
    assert_eq!(CheckResult::Flawless, result);
}

#[test]
fn rand() {
    init_log();
    let generated = RefCell::new(FxHashSet::default());
    const NUMS: i64 = 3;
    let model = Model::new(Counter::new).with_action("rand".into(), Strong, |s, c| {
        let random_number = c.rng().gen::<i64>();
        generated.borrow_mut().insert(random_number);
        match s.inc(c.name().into()) {
            NUMS => Joined,
            _ => Ran,
        }
    });
    assert_eq!(CheckResult::Flawless, Checker::new(&model).check());
    assert_eq!(NUMS, generated.borrow().len() as i64);

    // repeat run should yield the same random numbers
    assert_eq!(CheckResult::Flawless, Checker::new(&model).check());
    assert_eq!(NUMS, generated.borrow().len() as i64);
}
