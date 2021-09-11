use std::cell::RefCell;
use std::env;
use std::ops::{Deref, Div};
use std::rc::Rc;
use std::str::FromStr;
use std::time::{Duration, SystemTime};

use rand::RngCore;
use uuid::Uuid;

use std::convert::{TryFrom, TryInto};
use stride::havoc::checker::{CheckResult, Checker};
use stride::havoc::model::ActionResult::{Blocked, Breached, Joined, Ran};
use stride::havoc::model::{rand_element, ActionResult, Context, Model};
use stride::havoc::sim::{Sim, SimResult};
use stride::havoc::{checker, sim, Sublevel};
use stride::{
    AbortMessage, Candidate, CandidateMessage, CommitMessage, DecisionMessage, Examiner, Outcome,
};
use std::fmt::Debug;

#[derive(Debug)]
pub struct SystemState {
    pub cohorts: Vec<Cohort>,
    pub certifier: Certifier,
    pub run: usize,
}

impl SystemState {
    pub fn new(num_cohorts: usize, values: &[i32]) -> Self {
        let candidates_broker = Broker::new(1);
        let decisions_broker = Broker::new(1);
        let cohorts = (0..num_cohorts)
            .into_iter()
            .map(|_| Cohort {
                run: 0,
                replica: Replica::new(&values),
                candidates: candidates_broker.stream(),
                decisions: decisions_broker.stream(),
            })
            .collect();
        let certifier = Certifier {
            examiner: Examiner::new(),
            candidates: candidates_broker.stream(),
            decisions: decisions_broker.stream(),
        };

        SystemState {
            cohorts,
            certifier,
            run: 0,
        }
    }
}

impl CertifierState for SystemState {
    fn certifier(&mut self) -> &mut Certifier {
        &mut self.certifier
    }
}

impl CohortState for SystemState {
    fn cohorts(&mut self) -> &mut [Cohort] {
        &mut self.cohorts
    }
}

#[derive(Debug, Clone)]
pub struct Statemap {
    pub changes: Vec<(usize, i32)>,
}

impl Statemap {
    pub fn new(changes: Vec<(usize, i32)>) -> Self {
        Statemap { changes }
    }
}

#[derive(Debug)]
pub struct Cohort {
    pub run: usize,
    pub replica: Replica,
    pub candidates: Stream<CandidateMessage<Statemap>>,
    pub decisions: Stream<DecisionMessage<Statemap>>,
}

impl Cohort {
    // pub fn total_txns() {
    //     candidates.
    // }
}

#[derive(Debug)]
pub struct Certifier {
    pub examiner: Examiner,
    pub candidates: Stream<CandidateMessage<Statemap>>,
    pub decisions: Stream<DecisionMessage<Statemap>>,
}

#[derive(Debug)]
pub struct Replica {
    pub items: Vec<(i32, u64)>,
    pub ver: u64,
}

impl Replica {
    pub fn new(values: &[i32]) -> Self {
        Replica {
            items: values.iter().map(|&i| (i, 0)).collect(),
            ver: 0,
        }
    }

    pub fn can_install_ooo(&self, statemap: &Statemap, safepoint: u64, ver: u64) -> bool {
        if self.ver >= safepoint && ver > self.ver {
            for &(change_item, _) in &statemap.changes {
                let &(_, existing_ver) = &self.items[change_item];
                if ver > existing_ver {
                    return true;
                }
            }
        }
        false
    }

    fn install_items(&mut self, statemap: &Statemap, ver: u64) {
        for &(change_item, change_value) in &statemap.changes {
            let existing = &mut self.items[change_item];
            if ver > existing.1 {
                *existing = (change_value, ver);
            }
        }
    }

    pub fn install_ooo(&mut self, statemap: &Statemap, safepoint: u64, ver: u64) {
        if self.ver >= safepoint && ver > self.ver {
            self.install_items(statemap, ver);
        }
    }

    pub fn install_ser(&mut self, statemap: &Statemap, ver: u64) {
        if ver > self.ver {
            self.install_items(statemap, ver);
            self.ver = ver;
        }
    }
}

#[derive(Debug)]
pub struct Broker<M> {
    internals: Rc<RefCell<BrokerInternals<M>>>,
}

#[derive(Debug)]
struct BrokerInternals<M> {
    messages: Vec<Rc<M>>,
    base: usize,
}

impl<M> Broker<M> {
    pub fn new(base: usize) -> Self {
        Broker {
            internals: Rc::new(RefCell::new(BrokerInternals {
                messages: vec![],
                base,
            })),
        }
    }

    pub fn stream(&self) -> Stream<M> {
        let internals = Rc::clone(&self.internals);
        let offset = internals.borrow().base;
        Stream { internals, offset }
    }
}

#[derive(Debug)]
pub struct Stream<M> {
    internals: Rc<RefCell<BrokerInternals<M>>>,
    offset: usize,
}

impl<M> Stream<M> {
    pub fn produce(&self, message: Rc<M>) {
        self.internals
            .borrow_mut()
            .messages
            .push(Rc::clone(&message));
    }

    pub fn consume(&mut self) -> Option<(usize, Rc<M>)> {
        let internals = self.internals.borrow();
        let offset = self.offset;
        match internals.messages.get(offset - internals.base) {
            None => None,
            Some(message) => {
                self.offset += 1;
                Some((offset, Rc::clone(message)))
            }
        }
    }

    pub fn find<P>(&self, predicate: P) -> Vec<(usize, Rc<M>)>
    where
        P: Fn(&M) -> bool,
    {
        let internals = &self.internals.borrow();
        let messages = &internals.messages;
        let base = internals.base;
        messages
            .iter()
            .enumerate()
            .filter(|&(_, m)| predicate(m.deref()))
            .map(|(i, m)| (i + base, Rc::clone(&m)))
            .collect()
    }

    pub fn offset(&self) -> usize {
        self.offset
    }

    pub fn low_watermark(&self) -> usize {
        self.internals.borrow().base
    }

    pub fn high_watermark(&self) -> usize {
        let internals = self.internals.borrow();
        internals.base + internals.messages.len()
    }

    pub fn len(&self) -> usize {
        self.internals.borrow().messages.len()
    }
}

pub fn uuidify<T>(pid: T, run: T) -> Uuid
    where T: TryInto<u64>, <T as TryInto<u64>>::Error: Debug {
    try_uuidify(pid, run).unwrap()
}

pub fn try_uuidify<T>(pid: T, run: T) -> Result<Uuid, <T as TryInto<u64>>::Error> where T: TryInto<u64> {
    Ok(Uuid::from_u128((pid.try_into()? as u128) << 64 | run.try_into()? as u128))
}

// pub trait TruncateU64 {
//     fn trunc(self) -> u64;
// }
//
// impl TruncateU64 for usize {
//     fn trunc(self) -> u64 {
//         self as u64
//     }
// }
//
// pub fn uuidify<P, R>(pid: P, run: R) -> Uuid
// where
//     P: TruncateU64,
//     R: TruncateU64,
// {
//     Uuid::from_u128((pid.trunc() as u128) << 64 | run.trunc() as u128)
// }

// pub fn try_uuidify<T>(pid: T, run: T) -> Result<Uuid, <T as TryInto<u64>>::Error> where T: TryInto<u64> {
//     Ok(Uuid::from_u128((pid.try_into()? as u128) << 64 | run.try_into()? as u128))
// }

pub fn deuuid(uuid: Uuid) -> (usize, usize) {
    let val = uuid.as_u128();
    let pid = (val >> 64) as usize;
    let run = val as usize;
    (pid, run)
}

pub fn timed<F, R>(f: F) -> (R, Duration)
where
    F: Fn() -> R,
{
    let start = SystemTime::now();
    (
        f(),
        SystemTime::now()
            .duration_since(start)
            .unwrap_or(Duration::new(0, 0)),
    )
}

pub fn scale() -> usize {
    get_env::<usize, _>("SCALE", || 1)
}

pub fn seed() -> u64 {
    get_env("SEED", || rand::thread_rng().next_u64())
}

pub fn get_env<T, D>(key: &str, def: D) -> T
where
    T: FromStr,
    T::Err: std::fmt::Debug,
    D: Fn() -> T,
{
    match env::var(key) {
        Ok(str) => T::from_str(&str).expect(&format!("invalid {} value '{}'", key, str)),
        Err(_) => def(),
    }
}

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
        "took {:?} ({:?}/schedule, {:.3} schedules/sec)",
        elapsed,
        per_schedule,
        rate_s
    );
    if let CheckResult::Fail(fail) = &result {
        log::error!("trace:\n{}", fail.trace.prettify(&model));
    } else if let CheckResult::Deadlock(deadlock) = &result {
        log::error!("trace:\n{}", deadlock.trace.prettify(&model));
    }
    assert!(matches!(result, CheckResult::Pass(_)));
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
        log::error!("trace:\n{}", fail.trace.prettify(&model));
    } else if let SimResult::Deadlock(deadlock) = &result {
        log::error!("trace:\n{}", deadlock.trace.prettify(&model));
    }
    assert_eq!(SimResult::Pass, result);
}

pub trait CohortState {
    fn cohorts(&mut self) -> &mut [Cohort];
}

pub trait CertifierState {
    fn certifier(&mut self) -> &mut Certifier;
}

pub fn updater_action<S, A>(
    cohort_index: usize,
    asserter: A,
) -> impl Fn(&mut S, &mut dyn Context) -> ActionResult
where
    S: CohortState,
    A: Fn(&Replica) -> Option<String>,
{
    move |s, c| {
        let cohort = &mut s.cohorts()[cohort_index];
        let installable_commits = cohort.decisions.find(|decision| match decision {
            DecisionMessage::Commit(commit) => cohort.replica.can_install_ooo(
                &commit.statemap,
                commit.safepoint,
                commit.candidate.ver,
            ),
            DecisionMessage::Abort(_) => false,
        });

        if !installable_commits.is_empty() {
            log::trace!("Installable {:?}", installable_commits);
            let (_, commit) = rand_element(c, &installable_commits);
            let commit = commit.as_commit().unwrap();
            cohort
                .replica
                .install_ooo(&commit.statemap, commit.safepoint, commit.candidate.ver);
            if let Some(error) = asserter(&cohort.replica) {
                return Breached(error);
            }
            Ran
        } else {
            Blocked
        }
    }
}

pub fn replicator_action<S, A>(
    cohort_index: usize,
    asserter: A,
) -> impl Fn(&mut S, &mut dyn Context) -> ActionResult
where
    S: CohortState,
    A: Fn(&Replica) -> Option<String>,
{
    move |s, _| {
        let cohort = &mut s.cohorts()[cohort_index];
        match cohort.decisions.consume() {
            None => Blocked,
            Some((_, decision)) => {
                match decision.deref() {
                    DecisionMessage::Commit(commit) => {
                        cohort
                            .replica
                            .install_ser(&commit.statemap, commit.candidate.ver);
                        if let Some(error) = asserter(&cohort.replica) {
                            return Breached(error);
                        }
                    }
                    DecisionMessage::Abort(abort) => {
                        log::trace!("ABORTED {:?}", abort.reason);
                    }
                }
                Ran
            }
        }
    }
}

pub fn certifier_action<S>() -> impl Fn(&mut S, &mut dyn Context) -> ActionResult
where
    S: CertifierState,
{
    |s, _| {
        let certifier = s.certifier();
        match certifier.candidates.consume() {
            None => Blocked,
            Some((offset, candidate_message)) => {
                let candidate = Candidate {
                    rec: candidate_message.rec.clone(),
                    ver: offset as u64,
                };
                let outcome = certifier.examiner.assess(&candidate);
                log::trace!("Certified {:?} with {:?}", candidate, outcome);
                let decision_message = match outcome {
                    Outcome::Commit(safepoint, _) => DecisionMessage::Commit(CommitMessage {
                        candidate,
                        safepoint,
                        statemap: candidate_message.statemap.clone(),
                    }),
                    Outcome::Abort(reason, _) => {
                        DecisionMessage::Abort(AbortMessage { candidate, reason })
                    }
                };
                certifier.decisions.produce(Rc::new(decision_message));
                Ran
            }
        }
    }
}

pub fn supervisor_action<S>(
    expected_txns: usize,
) -> impl Fn(&mut S, &mut dyn Context) -> ActionResult
where
    S: CohortState,
{
    move |s, _| {
        let cohorts = s.cohorts();
        let finished_cohorts = cohorts
            .iter()
            .filter(|&cohort| cohort.decisions.offset() == expected_txns + 1)
            .count();
        if finished_cohorts == cohorts.len() {
            Joined
        } else {
            Blocked
        }
    }
}

#[cfg(test)]
mod tests;
