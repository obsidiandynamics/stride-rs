use crate::fixtures::xdb::Xdb;
use stride::examiner::Outcome::{Commit, Abort};
use stride::examiner::Discord::{Assertive, Permissive};
use uuid::Uuid;
use stride::examiner::AbortReason::Staleness;
use crate::fixtures::xdb::XdbAssignmentError::Conflict;
use crate::fixtures::xdb::Redaction::{New, Existing};

#[test]
fn xdb_assign_assertive_over_assertive_without_conflict() {
    let mut xdb = Xdb::new();
    assert_eq!(Ok(New(&Commit {safepoint: 7, discord: Assertive})),
               xdb.assign(Uuid::nil(), &Commit {safepoint: 7, discord: Assertive}));
    assert_eq!(Ok(Existing(&Commit {safepoint: 7, discord: Assertive})),
               xdb.assign(Uuid::nil(), &Commit {safepoint: 9, discord: Assertive}));
    assert_eq!(Some(&Commit {safepoint: 7, discord: Assertive}), xdb.get(&Uuid::nil()));
}

#[test]
fn xdb_assign_assertive_over_assertive_with_conflict() {
    let mut xdb = Xdb::new();
    assert_eq!(Ok(New(&Commit {safepoint: 7, discord: Assertive})),
               xdb.assign(Uuid::nil(), &Commit {safepoint: 7, discord: Assertive}));
    assert_eq!(Err(Conflict { existing: &Commit {safepoint: 7, discord: Assertive}, new: &Abort {reason: Staleness, discord: Assertive}}),
               xdb.assign(Uuid::nil(), &Abort {reason: Staleness, discord: Assertive}));
    assert_eq!(Some(&Commit {safepoint: 7, discord: Assertive}), xdb.get(&Uuid::nil()));
}

#[test]
fn xdb_assign_assertive_over_permissive_without_conflict() {
    let mut xdb = Xdb::new();
    assert_eq!(Ok(New(&Commit {safepoint: 7, discord: Permissive})),
               xdb.assign(Uuid::nil(), &Commit {safepoint: 7, discord: Permissive}));
    // should upgrade to assertive
    assert_eq!(Ok(Existing(&Commit {safepoint: 9, discord: Assertive})),
               xdb.assign(Uuid::nil(), &Commit {safepoint: 9, discord: Assertive}));
    assert_eq!(Some(&Commit {safepoint: 9, discord: Assertive}), xdb.get(&Uuid::nil()));
}

#[test]
fn xdb_assign_assertive_over_permissive_with_conflict() {
    let mut xdb = Xdb::new();
    assert_eq!(Ok(New(&Commit {safepoint: 7, discord: Permissive})),
               xdb.assign(Uuid::nil(), &Commit {safepoint: 7, discord: Permissive}));
    assert_eq!(Err(Conflict { existing: &Commit {safepoint: 7, discord: Permissive}, new: &Abort {reason: Staleness, discord: Assertive}}),
               xdb.assign(Uuid::nil(), &Abort {reason: Staleness, discord: Assertive}));
    assert_eq!(Some(&Commit {safepoint: 7, discord: Permissive}), xdb.get(&Uuid::nil()));
}

#[test]
fn xdb_assign_permissive_over_assertive_without_conflict() {
    let mut xdb = Xdb::new();
    assert_eq!(Ok(New(&Commit {safepoint: 7, discord: Assertive})),
               xdb.assign(Uuid::nil(), &Commit {safepoint: 7, discord: Assertive}));
    assert_eq!(Ok(Existing(&Commit {safepoint: 7, discord: Assertive})),
               xdb.assign(Uuid::nil(), &Commit {safepoint: 9, discord: Permissive}));
    assert_eq!(Some(&Commit {safepoint: 7, discord: Assertive}), xdb.get(&Uuid::nil()));
}

#[test]
fn xdb_assign_permissive_over_assertive_with_conflict() {
    let mut xdb = Xdb::new();
    assert_eq!(Ok(New(&Commit {safepoint: 7, discord: Assertive})),
               xdb.assign(Uuid::nil(), &Commit {safepoint: 7, discord: Assertive}));
    assert_eq!(Err(Conflict { existing: &Commit {safepoint: 7, discord: Assertive}, new: &Abort {reason: Staleness, discord: Permissive}}),
               xdb.assign(Uuid::nil(), &Abort {reason: Staleness, discord: Permissive}));
    assert_eq!(Some(&Commit {safepoint: 7, discord: Assertive}), xdb.get(&Uuid::nil()));
}

#[test]
fn xdb_assign_permissive_over_permissive_without_conflict() {
    let mut xdb = Xdb::new();
    assert_eq!(Ok(New(&Commit {safepoint: 7, discord: Permissive})),
               xdb.assign(Uuid::nil(), &Commit {safepoint: 7, discord: Permissive}));
    assert_eq!(Ok(Existing(&Commit {safepoint: 7, discord: Permissive})),
               xdb.assign(Uuid::nil(), &Commit {safepoint: 9, discord: Permissive}));
    assert_eq!(Some(&Commit {safepoint: 7, discord: Permissive}), xdb.get(&Uuid::nil()));
}

#[test]
fn xdb_assign_permissive_over_permissive_with_conflict() {
    let mut xdb = Xdb::new();
    assert_eq!(Ok(New(&Commit {safepoint: 7, discord: Permissive})),
               xdb.assign(Uuid::nil(), &Commit {safepoint: 7, discord: Permissive}));
    assert_eq!(Ok(Existing(&Commit {safepoint: 7, discord: Permissive})),
               xdb.assign(Uuid::nil(), &Abort {reason: Staleness, discord: Permissive}));
    assert_eq!(Some(&Commit {safepoint: 7, discord: Permissive}), xdb.get(&Uuid::nil()));
}