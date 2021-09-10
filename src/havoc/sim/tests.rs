use super::*;
use crate::havoc::component::{Counter, Lock};
use crate::havoc::model::name_of;
use crate::havoc::model::ActionResult::{Blocked, Joined, Ran};
use crate::havoc::model::Retention::Weak;
use rustc_hash::FxHashSet;
use std::cell::{Cell, RefCell};
use std::iter::FromIterator;

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
fn sim_one_shot() {
    init_log();
    let run_count = Cell::new(0);
    let model = Model::new(Counter::new)
        .with_name(name_of(&sim_one_shot).into())
        .with_action("action".into(), Strong, |_, _| {
            run_count.set(run_count.get() + 1);
            ActionResult::Joined
        });

    let sim = Sim::new(&model).with_config(default_config().with_max_schedules(3));
    assert_eq!(Pass, sim.check());
    assert_eq!(3, run_count.get());
}

#[test]
fn sim_two_shots() {
    init_log();
    let run_count = Cell::new(0);
    let model = Model::new(Counter::new)
        .with_name(name_of(&sim_two_shots).into())
        .with_action("test".into(), Strong, |s, c| {
            run_count.set(run_count.get() + 1);
            match s.inc(c.name().into()) {
                2 => ActionResult::Joined,
                _ => ActionResult::Ran,
            }
        });

    let sim = Sim::new(&model).with_config(default_config().with_max_schedules(3));
    assert_eq!(Pass, sim.check());
    assert_eq!(6, run_count.get());
}

#[test]
fn sim_two_actions() {
    init_log();
    let total_runs = RefCell::new(Counter::new());
    let model = Model::new(Counter::new)
        .with_name(name_of(&sim_two_actions).into())
        .with_action("a".into(), Strong, |_, c| {
            total_runs.borrow_mut().inc(c.name().into());
            Joined
        })
        .with_action("b".into(), Strong, |_, c| {
            total_runs.borrow_mut().inc(c.name().into());
            Joined
        });

    let sim = Sim::new(&model).with_config(default_config().with_max_schedules(2));
    let result = sim.check();
    assert_eq!(Pass, result);
    assert_eq!(2, total_runs.borrow().get("a"));
    assert_eq!(2, total_runs.borrow().get("b"));
}

#[test]
fn sim_two_actions_conditional() {
    init_log();
    let total_runs = RefCell::new(Counter::new());
    let model = Model::new(Counter::new)
        .with_name(name_of(&sim_two_actions_conditional).into())
        .with_action("a".into(), Strong, |s, c| {
            total_runs.borrow_mut().inc(c.name().into());
            s.inc(c.name().into());
            Joined
        })
        .with_action("b".into(), Strong, |s, c| {
            if s.inc(c.name().into()) == 1 && s.get("a") == 0 {
                return Blocked;
            }
            total_runs.borrow_mut().inc(c.name().into());
            Joined
        });

    let sim = Sim::new(&model).with_config(default_config().with_max_schedules(5));
    let result = sim.check();
    assert_eq!(Pass, result);
    assert_eq!(5, total_runs.borrow().get("a"));
    assert_eq!(5, total_runs.borrow().get("b"));
}

#[test]
fn sim_two_actions_by_two() {
    init_log();
    let total_runs = RefCell::new(Counter::new());
    let model = Model::new(Counter::new)
        .with_name(name_of(&sim_two_actions_by_two).into())
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

    let sim = Sim::new(&model).with_config(default_config().with_max_schedules(3));
    let result = sim.check();
    assert_eq!(Pass, result);
    assert_eq!(3, total_runs.borrow().get("a"));
    assert_eq!(6, total_runs.borrow().get("b"));
}

#[test]
fn sim_three_actions() {
    init_log();
    let total_runs = RefCell::new(Counter::new());
    let model = Model::new(Counter::new)
        .with_name(name_of(&sim_three_actions).into())
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

    let sim = Sim::new(&model).with_config(default_config().with_max_schedules(3));
    let result = sim.check();
    assert_eq!(Pass, result);
    assert_eq!(3, total_runs.borrow().get("a"));
    assert_eq!(3, total_runs.borrow().get("b"));
    assert_eq!(3, total_runs.borrow().get("c"));
}

#[test]
fn sim_three_actions_by_two() {
    init_log();
    let total_runs = RefCell::new(Counter::new());
    let model = Model::new(Counter::new)
        .with_name(name_of(&sim_three_actions_by_two).into())
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

    let sim = Sim::new(&model).with_config(default_config().with_max_schedules(3));
    let result = sim.check();
    assert_eq!(3, total_runs.borrow().get("a"));
    assert_eq!(3, total_runs.borrow().get("b"));
    assert_eq!(6, total_runs.borrow().get("c"));
    assert_eq!(Pass, result);
}

#[test]
fn sim_one_shot_deadlock() {
    init_log();
    let run_count = Cell::new(0);
    let model = Model::new(Counter::new)
        .with_name(name_of(&sim_one_shot_deadlock).into())
        .with_action("test".into(), Strong, |_, _| {
            run_count.set(run_count.get() + 1);
            ActionResult::Blocked
        });

    let sim = Sim::new(&model).with_config(default_config());
    assert_eq!(Deadlock, sim.check());
    assert_eq!(1, run_count.get());
}

#[test]
fn sim_two_actions_no_deadlock() {
    init_log();
    let mut model = Model::new(Lock::new).with_name(name_of(&sim_two_actions_no_deadlock).into());
    for c in ["a", "b"] {
        model.add_action(String::from("test-".to_owned() + c), Strong, |s, c| {
            if s.held(c.name()) {
                s.unlock();
                Joined
            } else if s.lock(c.name().into()) {
                Ran
            } else {
                Blocked
            }
        });
    }

    let sim = Sim::new(&model).with_config(default_config().with_max_schedules(10));
    assert_eq!(Pass, sim.check());
}

#[test]
fn sim_two_actions_deadlock() {
    init_log();
    let model = Model::new(|| [Lock::new(), Lock::new()])
        .with_name(name_of(&sim_two_actions_deadlock).into())
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

    let mut sim = Sim::new(&model).with_config(default_config());
    let mut results = FxHashSet::default();
    for seed in 0..999 {
        sim.set_seed(seed);
        results.insert(sim.check());
        if results.len() == 2 {
            break;
        }
    }
    // some runs will result in a deadlock, while others will pass
    assert_eq!(FxHashSet::from_iter([Deadlock, Pass]), results);
}

#[test]
fn sim_two_actions_one_weak_blocked() {
    init_log();
    let total_runs = RefCell::new(Counter::new());
    let model = Model::new(Counter::new)
        .with_name(name_of(&sim_two_actions_one_weak_blocked).into())
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

    let mut sim = Sim::new(&model).with_config(default_config());
    let mut run_counts = FxHashSet::default();
    let mut seed = 0;
    for _ in 0..999 {
        sim.set_seed(seed);
        seed += 1;
        assert_eq!(Pass, sim.check());
        let mut total_runs = total_runs.borrow_mut();
        run_counts.insert(total_runs.get("b"));
        total_runs.reset("b");
        if run_counts.len() == 2 {
            break;
        }
    }
    assert_eq!(seed, total_runs.borrow().get("a") as u64);
    assert_eq!(FxHashSet::from_iter([0, 1]), run_counts);
}

#[test]
fn sim_two_actions_one_weak_two_runs() {
    init_log();
    let total_runs = RefCell::new(Counter::new());
    let model = Model::new(Counter::new)
        .with_name(name_of(&sim_two_actions_one_weak_two_runs).into())
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

    let mut sim = Sim::new(&model).with_config(default_config());
    let mut run_counts = FxHashSet::default();
    let mut seed = 0;
    for _ in 0..999 {
        sim.set_seed(seed);
        seed += 1;
        assert_eq!(Pass, sim.check());
        let mut total_runs = total_runs.borrow_mut();
        run_counts.insert(total_runs.get("b"));
        total_runs.reset("b");
        if run_counts.len() == 3 {
            break;
        }
    }
    assert_eq!(seed, total_runs.borrow().get("a") as u64);
    assert_eq!(FxHashSet::from_iter([0, 1, 2]), run_counts);
}

#[test]
fn sim_rand() {
    init_log();
    let generated = RefCell::new(FxHashSet::default());
    const NUM_RUNS: i64 = 3;
    struct State {
        counter: Counter,
        rands: Vec<Vec<u64>>,
    }
    let model = Model::new(|| State {
        counter: Counter::new(),
        rands: vec![],
    })
    .with_name(name_of(&sim_rand).into())
    .with_action("test".into(), Strong, |s, c| {
        let current_rands = vec![c.rand(u64::MAX), c.rand(u64::MAX)];
        for &rand in &current_rands {
            generated.borrow_mut().insert(rand);
        }

        s.rands.push(current_rands);

        let trace = c.trace();
        let completed = s.counter.inc(c.name().into());
        assert_eq!(completed, trace.stack.len() as i64);
        let rands_from_trace: Vec<Vec<u64>> =
            trace.stack.iter().map(|call| call.rands.clone()).collect();
        assert_eq!(s.rands, rands_from_trace);
        match completed {
            NUM_RUNS => Joined,
            _ => Ran,
        }
    });
    assert_eq!(Pass, Sim::new(&model).with_config(default_config()).check());
    assert_eq!(NUM_RUNS * 2, generated.borrow().len() as i64);

    // repeat run should yield the same random numbers
    assert_eq!(Pass, Sim::new(&model).with_config(default_config()).check());
    assert_eq!(NUM_RUNS * 2, generated.borrow().len() as i64);
}

#[test]
fn sim_one_shot_breach() {
    init_log();
    let run_count = Cell::new(0);
    let model = Model::new(Counter::new)
        .with_name(name_of(&sim_one_shot_breach).into())
        .with_action("action".into(), Strong, |_, _| {
            run_count.set(run_count.get() + 1);
            Breached("some invariant".into())
        });

    let sim = Sim::new(&model).with_config(default_config().with_max_schedules(3));
    assert_eq!(
        Fail(
            SimFail {
                error: "some invariant".to_string(),
                trace: Trace::of(&[Call::of(0, &[])]),
                schedule: 0
            }
            .into()
        ),
        sim.check()
    );
    assert_eq!(1, run_count.get());
}
