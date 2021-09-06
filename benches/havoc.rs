use criterion::{BatchSize, Criterion, criterion_group, criterion_main};

use stride::havoc::checker::{Checker, CheckResult, Config};
use stride::havoc::component::Lock;
use stride::havoc::model::{Model, name_of};
use stride::havoc::model::ActionResult::{Blocked, Joined, Ran};
use stride::havoc::model::Retention::Strong;
use stride::havoc::Trace;

fn criterion_benchmark(c: &mut Criterion) {
    let _ = env_logger::builder().is_test(true).try_init();

    let model = Model::new(|| [Lock::new(), Lock::new()])
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
        });

    c.bench_function("deadlock", |b| {
        b.iter_batched(
            || {
                Checker::new(&model).with_config(
                    Config::default()
                        .with_trace(Trace::Fine)
                        .with_max_depth(usize::MAX),
                )
            },
            |checker| {
                assert_eq!(CheckResult::Deadlocked, checker.check());
            },
            BatchSize::SmallInput,
        )
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
