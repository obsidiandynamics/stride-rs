use stride::havoc::model::Model;
use crate::utils::{timed, seed, scale};
use stride::havoc::{Sublevel, checker, sim};
use stride::havoc::checker::{CheckResult, Checker};
use stride::havoc::sim::{Sim, SimResult};
use std::ops::Div;

fn init_log() {
    let _ = env_logger::builder().is_test(true).try_init();
}

pub fn dfs<S>(model: &Model<S>) {
    init_log();
    let (result, elapsed) = timed(|| {
        let config = checker::Config::default().with_sublevel(Sublevel::Fine);
        log::debug!(
            "checking model '{}' with {:?}",
            model.name().unwrap_or("untitled"),
            config
        );
        Checker::new(&model).with_config(config).check()
    });
    let stats = result.stats();
    let per_schedule = elapsed.div(stats.executed as u32);
    let rate_s = 1_000_000_000 as f64 / per_schedule.as_nanos() as f64;
    log::debug!(
        "took {:?} ({:?}/schedule, {:.3} schedules/sec) {:?}",
        elapsed,
        per_schedule,
        rate_s,
        stats
    );
    if let CheckResult::Fail(fail) = &result {
        log::error!("fail trace:\n{}", fail.trace.prettify(&model));
    } else if let CheckResult::Deadlock(deadlock) = &result {
        log::error!("deadlock trace:\n{}", deadlock.trace.prettify(&model));
    }
    assert!(matches!(result, CheckResult::Pass(_)), "{:?}", result);
}

pub fn sim<S>(model: &Model<S>, max_schedules: usize) {
    init_log();
    let seed = seed();
    let max_schedules = max_schedules * scale();
    let sim = Sim::new(&model)
        .with_config(
            sim::Config::default()
                .with_sublevel(Sublevel::Fine)
                .with_max_schedules(max_schedules),
        )
        .with_seed(seed);
    log::debug!(
        "simulating model '{}' with {:?} (seed: {})",
        model.name().unwrap_or("untitled"),
        sim.config(),
        seed
    );
    let (result, elapsed) = timed(|| sim.check());
    let per_schedule = elapsed.div(max_schedules as u32);
    let rate_s = 1_000_000_000 as f64 / per_schedule.as_nanos() as f64;
    log::debug!(
        "took {:?} ({:?}/schedule, {:.3} schedules/sec)",
        elapsed,
        per_schedule,
        rate_s
    );
    if let SimResult::Fail(fail) = &result {
        log::error!("fail trace:\n{}", fail.trace.prettify(&model));
    } else if let SimResult::Deadlock(deadlock) = &result {
        log::error!("deadlock trace:\n{}", deadlock.trace.prettify(&model));
    }
    assert_eq!(SimResult::Pass, result, "{:?} (seed: {})", result, seed);
}