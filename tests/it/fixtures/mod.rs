use std::fmt::Debug;
use std::ops::Deref;
use std::rc::Rc;

use stride::examiner::{Candidate, Examiner, Outcome};
use stride::havoc::model::{ActionResult, Context, rand_element};
use stride::havoc::model::ActionResult::{Blocked, Breached, Joined, Ran};
use stride::suffix::Suffix;

use crate::fixtures::broker::{Broker, Stream};
use crate::fixtures::xdb::Redaction::{Existing, New};
use crate::fixtures::xdb::Xdb;
use crate::utils::deuuid;
use crate::fixtures::schema::{MessageKind, DecisionMessageKind, AbortData, CommitData};
use crate::fixtures::schema::MessageKind::DecisionMessage;

mod broker;
pub mod schema;
mod xdb;

#[derive(Debug)]
pub struct SystemState {
    pub cohorts: Vec<Cohort>,
    pub certifiers: Vec<Certifier>,
    pub xdb: Xdb,
}

impl SystemState {
    pub fn new(num_cohorts: usize, init_values: &[i32], num_certifiers: usize) -> Self {
        let broker = Broker::new(1);
        let cohorts = (0..num_cohorts)
            .map(|_| Cohort {
                replica: Replica::new(&init_values),
                stream: broker.stream(),
            })
            .collect();
        let certifiers = (0..num_certifiers)
            .map(|_| Certifier {
                suffix: Suffix::default(),
                examiner: Examiner::default(),
                stream: broker.stream(),
            })
            .collect();

        SystemState { cohorts, certifiers, xdb: Xdb::default() }
    }

    pub fn total_txns(&self) -> usize {
        self.certifiers[0]
            .stream
            .count(|msg| msg.as_candidate().is_some())
    }

    pub fn cohort_txns(&self, cohort_index: usize) -> usize {
        self.certifiers[0].stream.count(|msg| match msg {
            MessageKind::CandidateMessage(candidate) => {
                let (pid, _) = deuuid::<usize, usize>(candidate.rec.xid);
                pid == cohort_index
            }
            MessageKind::DecisionMessage(_) => false,
        })
    }
}

impl CertifierState for SystemState {
    fn certifiers(&mut self) -> &mut [Certifier] {
        &mut self.certifiers
    }
}

impl CohortState for SystemState {
    fn cohorts(&mut self) -> &mut [Cohort] {
        &mut self.cohorts
    }
}

impl XdbState for SystemState {
    fn xdb(&mut self) -> &mut Xdb {
        &mut self.xdb
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
    pub stream: Stream<MessageKind<Statemap>>,
}

#[derive(Debug)]
pub struct Certifier {
    pub suffix: Suffix,
    pub examiner: Examiner,
    pub stream: Stream<MessageKind<Statemap>>,
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

pub trait CohortState {
    fn cohorts(&mut self) -> &mut [Cohort];
}

pub trait CertifierState {
    fn certifiers(&mut self) -> &mut [Certifier];
}

pub trait XdbState {
    fn xdb(&mut self) -> &mut Xdb;
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
                MessageKind::CandidateMessage(_) => false,
                MessageKind::DecisionMessage(decision) => match decision {
                    DecisionMessageKind::CommitMessage(commit) => cohort.replica.can_install_ooo(
                        &commit.statemap,
                        commit.safepoint,
                        commit.candidate.ver,
                    ),
                    DecisionMessageKind::AbortMessage(_) => false,
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
                        MessageKind::CandidateMessage(_) => {}
                        MessageKind::DecisionMessage(decision) => {
                            at_least_one_decision_consumed = true;
                            match decision {
                                DecisionMessageKind::CommitMessage(commit) => {
                                    let after_check = asserter(&s.cohorts());
                                    let cohort = &mut s.cohorts()[cohort_index];
                                    cohort
                                        .replica
                                        .install_ser(&commit.statemap, commit.candidate.ver);
                                    if let Some(error) = after_check(&s.cohorts()) {
                                        return Breached(error);
                                    }
                                }
                                DecisionMessageKind::AbortMessage(_) => {}
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

pub fn certifier_action<S>(certifier_index: usize, extent: usize) -> impl Fn(&mut S, &mut dyn Context) -> ActionResult
where
    S: CertifierState + XdbState,
{
    move |s, _| {
        let message = s.certifiers()[certifier_index].stream.consume();
        match message {
            None => Blocked,
            Some((offset, message)) => {
                match message.deref() {
                    MessageKind::CandidateMessage(candidate_message) => {
                        let certifier = &mut s.certifiers()[certifier_index];
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
                        let result = s.xdb().assign(candidate.rec.xid, &outcome);
                        let new_redaction = match result {
                            Ok(New(_)) => {
                                true
                            }
                            Ok(Existing(_)) => {
                                log::trace!("  duplicate");
                                false
                            },
                            Err(error) => {
                                return Breached(format!("XDB assignment error: {:?}", error));
                            }
                        };

                        if new_redaction {
                            let decision_message = match outcome {
                                Outcome::Commit(safepoint, _) => {
                                    DecisionMessageKind::CommitMessage(CommitData {
                                        candidate,
                                        safepoint,
                                        statemap: candidate_message.statemap.clone(),
                                    })
                                }
                                Outcome::Abort(reason, _) => {
                                    DecisionMessageKind::AbortMessage(AbortData { candidate, reason })
                                }
                            };

                            let certifier = &s.certifiers()[certifier_index];
                            certifier
                                .stream
                                .produce(Rc::new(DecisionMessage(decision_message)));
                        }
                    }
                    MessageKind::DecisionMessage(decision) => {
                        log::trace!("decision {:?}", decision.candidate());
                        let certifier = &mut s.certifiers()[certifier_index];
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
