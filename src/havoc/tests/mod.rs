mod common;

use super::*;
use crate::havoc::ActionResult::*;
use std::cell::{Cell, RefCell};
use crate::havoc::tests::common::{Counter, Lock};

#[test]
fn one_shot() {
    let run_count = Cell::new(0);
    let mut model = Model::new(Counter::new);
    model.push("one_shot".into(), |_, _| {
        run_count.set(run_count.get() + 1);
        ActionResult::Joined
    });

    let checker = Checker::new(&model);
    assert_eq!(CheckResult::Flawless, checker.check());
    assert_eq!(1, run_count.get());
}

#[test]
fn two_shot() {
    let run_count = Cell::new(0);
    let mut model = Model::new(Counter::new);
    model.push("two_shot".into(), |s, c| {
        run_count.set(run_count.get() + 1);
        match s.inc(c.name().into()) {
            2 => ActionResult::Joined,
            _ => ActionResult::Ran
        }
    });

    let checker = Checker::new(&model);
    assert_eq!(CheckResult::Flawless, checker.check());
    assert_eq!(2, run_count.get());
}

#[test]
fn two_actions() {
    let total_runs = RefCell::new(Counter::new());
    let mut model = Model::new(Counter::new);
    model.push("two_actions_a".into(), |_, c| {
        total_runs.borrow_mut().inc(c.name().into());
        Joined
    });
    model.push("two_actions_b".into(), |_, c| {
        total_runs.borrow_mut().inc(c.name().into());
        Joined
    });
    let checker = Checker::new(&model);
    let result = checker.check();
    assert_eq!(2, total_runs.borrow().get("two_actions_a"));
    assert_eq!(2, total_runs.borrow().get("two_actions_b"));
    assert_eq!(CheckResult::Flawless, result);
}

#[test]
fn two_actions_conditional() {
    let total_runs = RefCell::new(Counter::new());
    let mut model = Model::new(Counter::new);
    model.push("two_actions_conditional_a".into(), |s, c| {
        total_runs.borrow_mut().inc(c.name().into());
        s.inc(c.name().into());
        Joined
    });
    model.push("two_actions_conditional_b".into(), |s, c| {
        total_runs.borrow_mut().inc(c.name().into());
        if s.inc(c.name().into()) == 0 && s.get("two_actions_conditional_a") == 0 {
            return Ran
        }
        Joined
    });
    let checker = Checker::new(&model);
    let result = checker.check();
    assert_eq!(3, total_runs.borrow().get("two_actions_conditional_a"));
    assert_eq!(5, total_runs.borrow().get("two_actions_conditional_b"));
    assert_eq!(CheckResult::Flawless, result);
}

#[test]
fn two_actions_by_two() {
    let total_runs = RefCell::new(Counter::new());
    let mut model = Model::new(Counter::new);
    model.push("two_actions_by_two_0".into(), |_, c| {
        total_runs.borrow_mut().inc(c.name().into());
        Joined
    });
    model.push("two_actions_by_two_1".into(), |s, c| {
        total_runs.borrow_mut().inc(c.name().into());
        match s.inc(c.name().into()) {
            2 => Joined,
            _ => Ran
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
    let total_runs = RefCell::new(Counter::new());
    let mut model = Model::new(Counter::new);
    model.push("three_actions_a".into(), |_, c| {
        total_runs.borrow_mut().inc(c.name().into());
        Joined
    });
    model.push("three_actions_b".into(), |_, c| {
        total_runs.borrow_mut().inc(c.name().into());
        Joined
    });
    model.push("three_actions_c".into(), |_, c| {
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
    let total_runs = RefCell::new(Counter::new());
    let mut model = Model::new(Counter::new);
    model.push("three_actions_by_two_a".into(), |_, c| {
        total_runs.borrow_mut().inc(c.name().into());
        Joined
    });
    model.push("three_actions_by_two_b".into(), |_, c| {
        total_runs.borrow_mut().inc(c.name().into());
        Joined
    });
    model.push("three_actions_by_two_c".into(), |s, c| {
        total_runs.borrow_mut().inc(c.name().into());
        match s.inc(c.name().into()) {
            2 => Joined,
            _ => Ran
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
    let run_count = Cell::new(0);
    let mut model = Model::new(Counter::new);
    model.push("one_shot_deadlock".into(), |_, _| {
        run_count.set(run_count.get() + 1);
        ActionResult::Blocked
    });

    let checker = Checker::new(&model);
    assert_eq!(CheckResult::Deadlocked, checker.check());
    assert_eq!(1, run_count.get());
}

#[test]
fn two_actions_no_deadlock() {
    let mut model = Model::new(Lock::new);
    for c in ["a", "b"] {
        model.push(String::from("two_actions_no_deadlock_".to_owned() + c),
                   |s, c| {
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

    let checker = Checker::new(&model);
    assert_eq!(CheckResult::Flawless, checker.check());
}

#[test]
fn two_actions_deadlock() {
    let mut model = Model::new(|| vec![Lock::new(), Lock::new()]);
    model.push("two_actions_deadlock_a".into(), |s, c| {
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
    });
    model.push("two_actions_deadlock_b".into(), |s, c| {
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

    let checker = Checker::new(&model);
    assert_eq!(CheckResult::Deadlocked, checker.check());
}