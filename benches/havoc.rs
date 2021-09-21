use criterion::{BatchSize, Criterion, criterion_group, criterion_main};

use stride::havoc::checker::{Checker, CheckResult};
use stride::havoc::components::Lock;
use stride::havoc::model::{Model, name_of};
use stride::havoc::model::ActionResult::{Blocked, Joined, Ran};
use stride::havoc::model::Retention::Strong;
use stride::havoc::{Sublevel, checker, sim};
use stride::havoc::sim::{Sim, SimResult};

fn build_model() -> Model<'static, [Lock; 2]> {
    Model::new(|| [Lock::new(), Lock::new()])
        .with_name(name_of(&criterion_benchmark).into())
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
        })
}

fn criterion_benchmark(c: &mut Criterion) {
    let _ = env_logger::builder().is_test(true).try_init();

    let model = build_model();

    c.bench_function("dfs_deadlock", |b| {
        b.iter_batched(
            || {
                Checker::new(&model).with_config(
                    checker::Config::default()
                        .with_sublevel(Sublevel::Fine)
                        .with_max_depth(usize::MAX),
                )
            },
            |checker| {
                assert!(matches!(checker.check(), CheckResult::Deadlock(_)));
            },
            BatchSize::SmallInput,
        )
    });

    c.bench_function("sim_deadlock", |b| {
        let mut i = 0;
        b.iter_batched(
            || {
                let sim = Sim::new(&model).with_config(
                    sim::Config::default()
                        .with_sublevel(Sublevel::Fine)
                        .with_max_schedules(usize::MAX)
                        .with_max_depth(usize::MAX),
                    )
                    .with_seed(i);
                i += 1;
                sim
            },
            |sim| {
                assert!(matches!(sim.check(), SimResult::Deadlock(_)));
            },
            BatchSize::SmallInput,
        )
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
