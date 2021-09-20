use crate::examiner::{Candidate, AbortReason, Record};

pub mod examiner;
pub mod havoc;
pub mod suffix;

#[derive(Debug)]
pub enum Message<S> {
    Candidate(CandidateMessage<S>),
    Decision(DecisionMessage<S>),
}

impl <S> Message<S> {
    pub fn as_candidate(&self) -> Option<&CandidateMessage<S>> {
        match self {
            Message::Candidate(candidate) => Some(candidate),
            Message::Decision(_) => None
        }
    }

    pub fn as_decision(&self) -> Option<&DecisionMessage<S>> {
        match self {
            Message::Candidate(_) => None,
            Message::Decision(decision) => Some(decision)
        }
    }
}

#[derive(Debug)]
pub struct CandidateMessage<S> {
    pub rec: Record,
    pub statemap: S,
}

#[derive(Debug)]
pub enum DecisionMessage<S> {
    Commit(CommitMessage<S>),
    Abort(AbortMessage)
}

impl<S> DecisionMessage<S> {
    pub fn as_commit(&self) -> Option<&CommitMessage<S>> {
        match self {
            DecisionMessage::Commit(message) => Some(message),
            DecisionMessage::Abort(_) => None
        }
    }

    pub fn as_abort(&self) -> Option<&AbortMessage> {
        match self {
            DecisionMessage::Commit(_) => None,
            DecisionMessage::Abort(message) => Some(message)
        }
    }

    pub fn candidate(&self) -> &Candidate {
        match self {
            DecisionMessage::Commit(commit) => &commit.candidate,
            DecisionMessage::Abort(abort) => &abort.candidate
        }
    }
}

#[derive(Debug)]
pub struct CommitMessage<S> {
    pub candidate: Candidate,
    pub safepoint: u64,
    pub statemap: S
}

#[derive(Debug)]
pub struct AbortMessage {
    pub candidate: Candidate,
    pub reason: AbortReason
}

#[cfg(test)]
mod tests;
