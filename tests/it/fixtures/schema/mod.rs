use stride::examiner::{Candidate, AbortReason, Record};

#[derive(Debug)]
pub enum MessageKind<S> {
    CandidateMessage(CandidateData<S>),
    DecisionMessage(DecisionMessageKind<S>),
}

impl <S> MessageKind<S> {
    pub fn as_candidate(&self) -> Option<&CandidateData<S>> {
        match self {
            MessageKind::CandidateMessage(candidate) => Some(candidate),
            MessageKind::DecisionMessage(_) => None
        }
    }

    pub fn as_decision(&self) -> Option<&DecisionMessageKind<S>> {
        match self {
            MessageKind::CandidateMessage(_) => None,
            MessageKind::DecisionMessage(decision) => Some(decision)
        }
    }
}

#[derive(Debug)]
pub struct CandidateData<S> {
    pub rec: Record,
    pub statemap: S,
}

#[derive(Debug)]
pub enum DecisionMessageKind<S> {
    CommitMessage(CommitData<S>),
    AbortMessage(AbortData)
}

impl<S> DecisionMessageKind<S> {
    pub fn as_commit(&self) -> Option<&CommitData<S>> {
        match self {
            DecisionMessageKind::CommitMessage(message) => Some(message),
            DecisionMessageKind::AbortMessage(_) => None
        }
    }

    pub fn candidate(&self) -> &Candidate {
        match self {
            DecisionMessageKind::CommitMessage(commit) => &commit.candidate,
            DecisionMessageKind::AbortMessage(abort) => &abort.candidate
        }
    }
}

#[derive(Debug)]
pub struct CommitData<S> {
    pub candidate: Candidate,
    pub safepoint: u64,
    pub statemap: S
}

#[derive(Debug)]
pub struct AbortData {
    pub candidate: Candidate,
    pub reason: AbortReason
}
