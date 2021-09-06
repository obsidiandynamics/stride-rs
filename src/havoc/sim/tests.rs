use super::*;
use crate::havoc::model::name_of;
use std::cell::{Cell, RefCell};
use crate::havoc::component::Counter;
use crate::havoc::model::ActionResult::{Joined, Ran, Blocked};
use more_asserts::assert_le;

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
    assert_eq!(SimResult::Flawless, sim.check());
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
    assert_eq!(SimResult::Flawless, sim.check());
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
    assert_eq!(SimResult::Flawless, result);
    assert_le!(2, total_runs.borrow().get("a"));
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
            if s.inc(c.name().into()) == 0 && s.get("a") == 0 {
                return Blocked;
            }
            total_runs.borrow_mut().inc(c.name().into());
            Joined
        });

    let sim = Sim::new(&model).with_config(default_config().with_max_schedules(5));
    let result = sim.check();
    assert_eq!(SimResult::Flawless, result);
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
    assert_eq!(SimResult::Flawless, result);
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
    assert_eq!(SimResult::Flawless, result);
    assert_eq!(3, total_runs.borrow().get("a"));
    assert_eq!(3, total_runs.borrow().get("b"));
    assert_eq!(3, total_runs.borrow().get("c"));
}