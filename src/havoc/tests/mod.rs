mod common;

use super::*;
use crate::havoc::ActionResult::*;
use std::cell::{Cell, RefCell};
use rustc_hash::FxHashMap;
use std::collections::hash_map::{Entry};
use crate::havoc::tests::common::{Counter, Lock};

#[test]
fn one_shot() {
    let run_count = Cell::new(0);
    let mut model = Model::new(Counter::new);
    model.push("one_shot".into(), |_| {
        run_count.set(run_count.get() + 1);
        ActionResult::Joined
    });

    let mut executor = Executor::new(&model);
    assert_eq!(ExecutionResult::Flawless, executor.run());
    assert_eq!(1, run_count.get());
}

#[test]
fn two_shot() {
    let run_count = Cell::new(0);
    let mut model = Model::new(Counter::new);
    model.push("two_shot".into(), |s| {
        run_count.set(run_count.get() + 1);
        match s.inc("two_shot") {
            2 => ActionResult::Joined,
            _ => ActionResult::Ran
        }
    });

    let mut executor = Executor::new(&model);
    assert_eq!(ExecutionResult::Flawless, executor.run());
    assert_eq!(2, run_count.get());
}

#[test]
fn two_actions() {
    let total_runs = RefCell::new(Counter::new());
    let mut model = Model::new(Counter::new);
    model.push("two_actions_a".into(), |_| {
        total_runs.borrow_mut().inc("two_actions_a");
        Joined
    });
    model.push("two_actions_b".into(), |_| {
        total_runs.borrow_mut().inc("two_actions_b");
        Joined
    });
    let mut executor = Executor::new(&model);
    let result = executor.run();
    assert_eq!(2, total_runs.borrow().get("two_actions_a"));
    assert_eq!(2, total_runs.borrow().get("two_actions_b"));
    assert_eq!(ExecutionResult::Flawless, result);
}

#[test]
fn two_actions_conditional() {
    let total_runs = RefCell::new(Counter::new());
    let mut model = Model::new(Counter::new);
    model.push("two_actions_conditional_a".into(), |s| {
        total_runs.borrow_mut().inc("two_actions_conditional_a");
        s.inc("two_actions_conditional_a");
        Joined
    });
    model.push("two_actions_conditional_b".into(), |s| {
        total_runs.borrow_mut().inc("two_actions_conditional_b");
        if s.inc("two_actions_conditional_b") == 0 && s.get("two_actions_conditional_a") == 0 {
            return Ran
        }
        Joined
    });
    let mut executor = Executor::new(&model);
    let result = executor.run();
    assert_eq!(3, total_runs.borrow().get("two_actions_conditional_a"));
    assert_eq!(5, total_runs.borrow().get("two_actions_conditional_b"));
    assert_eq!(ExecutionResult::Flawless, result);
}

#[test]
fn two_actions_by_two() {
    let total_runs = RefCell::new(Counter::new());
    let mut model = Model::new(Counter::new);
    model.push("two_actions_by_two_0".into(), |_| {
        total_runs.borrow_mut().inc("two_actions_by_two_0");
        Joined
    });
    model.push("two_actions_by_two_1".into(), |s| {
        total_runs.borrow_mut().inc("two_actions_by_two_1");
        match s.inc("two_actions_by_two_1") {
            2 => Joined,
            _ => Ran
        }
    });
    let mut executor = Executor::new(&model);
    let result = executor.run();
    assert_eq!(3, total_runs.borrow().get("two_actions_by_two_0"));
    assert_eq!(6, total_runs.borrow().get("two_actions_by_two_1"));
    assert_eq!(ExecutionResult::Flawless, result);
}

#[test]
fn three_actions() {
    let total_runs = RefCell::new(Counter::new());
    let mut model = Model::new(Counter::new);
    model.push("three_actions_a".into(), |_| {
        total_runs.borrow_mut().inc("three_actions_a");
        Joined
    });
    model.push("three_actions_b".into(), |_| {
        total_runs.borrow_mut().inc("three_actions_b");
        Joined
    });
    model.push("three_actions_c".into(), |_| {
        total_runs.borrow_mut().inc("three_actions_c");
        Joined
    });
    let mut executor = Executor::new(&model);
    let result = executor.run();
    assert_eq!(6, total_runs.borrow().get("three_actions_a"));
    assert_eq!(6, total_runs.borrow().get("three_actions_b"));
    assert_eq!(6, total_runs.borrow().get("three_actions_c"));
    assert_eq!(ExecutionResult::Flawless, result);
}

#[test]
fn three_actions_by_two() {
    let total_runs = RefCell::new(Counter::new());
    let mut model = Model::new(Counter::new);
    model.push("three_actions_by_two_a".into(), |_| {
        total_runs.borrow_mut().inc("three_actions_by_two_a");
        Joined
    });
    model.push("three_actions_by_two_b".into(), |_| {
        total_runs.borrow_mut().inc("three_actions_by_two_b");
        Joined
    });
    model.push("three_actions_by_two_c".into(), |s| {
        total_runs.borrow_mut().inc("three_actions_by_two_c");
        match s.inc("three_actions_by_two_c") {
            2 => Joined,
            _ => Ran
        }
    });
    let mut executor = Executor::new(&model);
    let result = executor.run();
    assert_eq!(12, total_runs.borrow().get("three_actions_by_two_a"));
    assert_eq!(12, total_runs.borrow().get("three_actions_by_two_b"));
    assert_eq!(24, total_runs.borrow().get("three_actions_by_two_c"));
    assert_eq!(ExecutionResult::Flawless, result);
}

#[test]
fn one_shot_deadlock() {
    let run_count = Cell::new(0);
    let mut model = Model::new(Counter::new);
    model.push("one_shot_deadlock".into(), |_| {
        run_count.set(run_count.get() + 1);
        ActionResult::Blocked
    });

    let mut executor = Executor::new(&model);
    assert_eq!(ExecutionResult::Deadlocked, executor.run());
    assert_eq!(1, run_count.get());
}

#[test]
fn two_actions_no_deadlock() {
    let mut model = Model::new(Lock::new);
    model.push("two_actions_no_deadlock_a".into(), |s| {
        if s.held("two_actions_no_deadlock_a") {
            s.unlock();
            Joined
        } else if s.lock("two_actions_no_deadlock_a") {
            Ran
        } else {
            Blocked
        }
    });
    model.push("two_actions_no_deadlock_b".into(), |s| {
        if s.held("two_actions_no_deadlock_b") {
            s.unlock();
            Joined
        } else if s.lock("two_actions_no_deadlock_b") {
            Ran
        } else {
            Blocked
        }
    });

    let mut executor = Executor::new(&model);
    assert_eq!(ExecutionResult::Flawless, executor.run());
}

#[test]
fn two_actions_deadlock() {
    let mut model = Model::new(|| vec![Lock::new(), Lock::new()]);
    model.push("two_actions_deadlock_a".into(), |s| {
        if s[0].held("two_actions_deadlock_a") {
            if s[1].held("two_actions_deadlock_a") {
                s[1].unlock();
                s[0].unlock();
                Joined
            } else if s[1].lock("two_actions_deadlock_a") {
                Ran
            } else {
                Blocked
            }
        } else if s[0].lock("two_actions_deadlock_a") {
            Ran
        } else {
            Blocked
        }
    });
    model.push("two_actions_deadlock_b".into(), |s| {
        if s[1].held("two_actions_deadlock_b") {
            if s[0].held("two_actions_deadlock_b") {
                s[0].unlock();
                s[1].unlock();
                Joined
            } else if s[0].lock("two_actions_deadlock_b") {
                Ran
            } else {
                Blocked
            }
        } else if s[1].lock("two_actions_deadlock_b") {
            Ran
        } else {
            Blocked
        }
    });

    let mut executor = Executor::new(&model);
    assert_eq!(ExecutionResult::Deadlocked, executor.run());
}