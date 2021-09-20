use std::cell::RefCell;
use std::env;
use std::ops::{Deref, Div};
use std::rc::Rc;
use std::str::FromStr;
use std::time::{Duration, SystemTime};

use rand::RngCore;
use uuid::Uuid;

use std::convert::{TryFrom, TryInto};
use std::fmt::Debug;
use stride::examiner::{Examiner, Outcome, Discord, Candidate};
use stride::havoc::checker::{CheckResult, Checker};
use stride::havoc::model::ActionResult::{Blocked, Breached, Joined, Ran};
use stride::havoc::model::{rand_element, ActionResult, Context, Model};
use stride::havoc::sim::{Sim, SimResult};
use stride::havoc::{checker, sim, Sublevel};
use stride::suffix::{Suffix};
use stride::Message::Decision;
use stride::{AbortMessage, CommitMessage, DecisionMessage, Message};
use rustc_hash::FxHashMap;
use std::collections::hash_map::Entry;
use crate::fixtures::XdbAssignmentError::Conflict;

#[derive(Debug)]
pub struct SystemState {
    pub cohorts: Vec<Cohort>,
    pub certifier: Certifier,
}

impl SystemState {
    pub fn new(num_cohorts: usize, values: &[i32]) -> Self {
        let broker = Broker::new(1);
        let cohorts = (0..num_cohorts)
            .into_iter()
            .map(|_| Cohort {
                replica: Replica::new(&values),
                stream: broker.stream(),
            })
            .collect();
        let certifier = Certifier {
            suffix: Suffix::default(),
            examiner: Examiner::default(),
            stream: broker.stream(),
        };

        SystemState { cohorts, certifier }
    }

    pub fn total_txns(&self) -> usize {
        self.certifier
            .stream
            .count(|msg| msg.as_candidate().is_some())
    }

    pub fn cohort_txns(&self, cohort_index: usize) -> usize {
        self.certifier.stream.count(|msg| match msg {
            Message::Candidate(candidate) => {
                let (pid, _) = deuuid::<usize, usize>(candidate.rec.xid);
                pid == cohort_index
            }
            Message::Decision(_) => false,
        })
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

#[derive(Debug, Clone, Copy)]
pub enum Op {
    Set(i32),
    Add(i32),
    Mpy(i32),
}

impl Op {
    fn eval(&self, existing: i32) -> i32 {
        match self {
            Op::Set(new) => *new,
            Op::Add(addend) => existing + *addend,
            Op::Mpy(multiplicand) => existing * *multiplicand,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Statemap {
    pub changes: Vec<(usize, Op)>,
}

impl Statemap {
    pub fn new(changes: Vec<(usize, Op)>) -> Self {
        Statemap { changes }
    }

    pub fn map<M>(changes: &[(usize, i32)], mapper: M) -> Self
    where
        M: Fn(i32) -> Op,
    {
        Self::new(
            changes
                .iter()
                .map(|&(item, val)| (item, mapper(val)))
                .collect(),
        )
    }
}

#[derive(Debug)]
pub struct Cohort {
    pub replica: Replica,
    pub stream: Stream<Message<Statemap>>,
}

#[derive(Debug)]
pub struct Certifier {
    pub suffix: Suffix,
    pub examiner: Examiner,
    pub stream: Stream<Message<Statemap>>,
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
        for (change_item, change_value) in &statemap.changes {
            let existing = &mut self.items[*change_item];
            if ver > existing.1 {
                *existing = (change_value.eval(existing.0), ver);
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
pub struct Xdb {
    pub txns: FxHashMap<Uuid, Outcome>
}

#[derive(Debug, PartialEq)]
pub enum XdbAssignmentError {
    Conflict { existing: Outcome, new: Outcome }
}

impl Xdb {
    pub fn new() -> Self {
        Self {
            txns: FxHashMap::default()
        }
    }

    pub fn assign(&mut self, xid: Uuid, outcome: &Outcome) -> Result<Outcome, XdbAssignmentError> {
        match self.txns.entry(xid) {
            Entry::Occupied(entry) => {
                let existing = entry.get();
                if *outcome.discord() == Discord::Assertive &&
                    (existing.is_abort() && outcome.is_commit() || existing.is_commit() && outcome.is_abort()) {
                    return Err(Conflict { existing: existing.clone(), new: outcome.clone() })
                }
                Ok(existing.clone())
            }
            Entry::Vacant(entry) => {
                entry.insert(outcome.clone());
                Ok(outcome.clone())
            }
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

    pub fn count<P>(&self, predicate: P) -> usize
    where
        P: Fn(&M) -> bool,
    {
        let internals = &self.internals.borrow();
        let messages = &internals.messages;
        messages
            .iter()
            .enumerate()
            .filter(|&(_, m)| predicate(m.deref()))
            .count()
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

pub fn uuidify<P, R>(pid: P, run: R) -> Uuid
where
    P: TryInto<u64>,
    R: TryInto<u64>,
    <P as TryInto<u64>>::Error: Debug,
    <R as TryInto<u64>>::Error: Debug,
{
    try_uuidify(pid, run).unwrap()
}

#[derive(Debug)]
pub enum Bimorphic<U, V> {
    A(U),
    B(V),
}

pub fn try_uuidify<P, R>(
    pid: P,
    run: R,
) -> Result<Uuid, Bimorphic<<P as TryInto<u64>>::Error, <R as TryInto<u64>>::Error>>
where
    P: TryInto<u64>,
    R: TryInto<u64>,
{
    let pid = pid.try_into().map_err(|err| Bimorphic::A(err))? as u128;
    let run = run.try_into().map_err(|err| Bimorphic::B(err))? as u128;
    Ok(Uuid::from_u128(pid << 64 | run))
}

pub fn deuuid<P, R>(uuid: Uuid) -> (P, R)
where
    P: TryFrom<u64>,
    <P as TryFrom<u64>>::Error: Debug,
    R: TryFrom<u64>,
    <R as TryFrom<u64>>::Error: Debug,
{
    try_deuuid(uuid).unwrap()
}

pub fn try_deuuid<P, R>(
    uuid: Uuid,
) -> Result<(P, R), Bimorphic<<P as TryFrom<u64>>::Error, <R as TryFrom<u64>>::Error>>
where
    P: TryFrom<u64>,
    R: TryFrom<u64>,
{
    let val = uuid.as_u128();
    let pid = <P>::try_from((val >> 64) as u64).map_err(|err| Bimorphic::A(err))?;
    let run = <R>::try_from(val as u64).map_err(|err| Bimorphic::B(err))?;
    Ok((pid, run))
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
    A: Fn(&[Cohort]) -> Box<dyn Fn(&[Cohort]) -> Option<String>>,
{
    move |s, c| {
        let installable_commits = {
            let cohort = &mut s.cohorts()[cohort_index];
            cohort.stream.find(|message| match message {
                Message::Candidate(_) => false,
                Message::Decision(decision) => match decision {
                    DecisionMessage::Commit(commit) => cohort.replica.can_install_ooo(
                        &commit.statemap,
                        commit.safepoint,
                        commit.candidate.ver,
                    ),
                    DecisionMessage::Abort(_) => false,
                },
            })
        };

        if !installable_commits.is_empty() {
            log::trace!("Installable {:?}", installable_commits);
            let (_, commit) = rand_element(c, &installable_commits);
            let after_check = asserter(&s.cohorts());
            let cohort = &mut s.cohorts()[cohort_index];
            let commit = commit.as_decision().unwrap().as_commit().unwrap();
            cohort
                .replica
                .install_ooo(&commit.statemap, commit.safepoint, commit.candidate.ver);
            if let Some(error) = after_check(&s.cohorts()) {
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
    A: Fn(&[Cohort]) -> Box<dyn Fn(&[Cohort]) -> Option<String>>,
{
    move |s, _| {
        let mut at_least_one_decision_consumed = false;
        loop {
            // looping lets us skip over candidates that appear ahead of the next decision in the
            // stream, reducing the model diameter
            match s.cohorts()[cohort_index].stream.consume() {
                None => {
                    return if at_least_one_decision_consumed {
                        Ran
                    } else {
                        Blocked
                    }
                }
                Some((_, message)) => {
                    match message.deref() {
                        Message::Candidate(_) => {}
                        Message::Decision(decision) => {
                            at_least_one_decision_consumed = true;
                            match decision {
                                DecisionMessage::Commit(commit) => {
                                    let after_check = asserter(&s.cohorts());
                                    let cohort = &mut s.cohorts()[cohort_index];
                                    cohort
                                        .replica
                                        .install_ser(&commit.statemap, commit.candidate.ver);
                                    if let Some(error) = after_check(&s.cohorts()) {
                                        return Breached(error);
                                    }
                                }
                                DecisionMessage::Abort(_) => {}
                            }
                        }
                    }
                    if at_least_one_decision_consumed {
                        return Ran;
                    }
                }
            };
        }
    }
}

pub fn certifier_action<S>(extent: usize) -> impl Fn(&mut S, &mut dyn Context) -> ActionResult
where
    S: CertifierState,
{
    move |s, _| {
        let certifier = s.certifier();
        match certifier.stream.consume() {
            None => Blocked,
            Some((offset, message)) => {
                match message.deref() {
                    Message::Candidate(candidate_message) => {
                        let result = certifier.suffix.insert(
                            candidate_message.rec.readset.clone(),
                            candidate_message.rec.writeset.clone(),
                            offset as u64,
                        );
                        if let Err(error) = result {
                            return Breached(format!("suffix insertion error: {:?}", error));
                        }

                        let candidate = Candidate {
                            rec: candidate_message.rec.clone(),
                            ver: offset as u64,
                        };
                        let outcome = certifier.examiner.assess(candidate.clone());
                        log::trace!(
                            "Certified {:?} {:?} with {:?}",
                            candidate,
                            &candidate_message.statemap,
                            outcome
                        );
                        let decision_message = match outcome {
                            Outcome::Commit(safepoint, _) => {
                                DecisionMessage::Commit(CommitMessage {
                                    candidate,
                                    safepoint,
                                    statemap: candidate_message.statemap.clone(),
                                })
                            }
                            Outcome::Abort(reason, _) => {
                                DecisionMessage::Abort(AbortMessage { candidate, reason })
                            }
                        };
                        certifier
                            .stream
                            .produce(Rc::new(Decision(decision_message)));
                    }
                    Message::Decision(decision) => {
                        log::trace!("decision {:?}", decision.candidate());
                        let result = certifier.suffix.decide(decision.candidate().ver);
                        if let Err(error) = result {
                            return Breached(format!("suffix decision error: {:?}", error));
                        }

                        if {
                            let truncated = certifier.suffix.truncate(extent, extent);
                            match truncated {
                                None => false,
                                Some(truncated_entries) => {
                                    for truncated_entry in truncated_entries {
                                        log::trace!("  truncating {:?}", truncated_entry);
                                        certifier.examiner.discard(truncated_entry);
                                    }
                                    true
                                }
                            }
                        } {
                            log::trace!("    range {:?}", certifier.suffix.range());
                        }
                    }
                }
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
            .filter(|&cohort| cohort.stream.offset() == expected_txns * 2 + 1)
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
