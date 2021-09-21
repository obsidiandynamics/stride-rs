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
    assert_eq!(Ok(New(&Commit(7, Assertive))),
               xdb.assign(Uuid::nil(), &Commit(7, Assertive)));
    assert_eq!(Ok(Existing(&Commit(7, Assertive))),
               xdb.assign(Uuid::nil(), &Commit(9, Assertive)));
    assert_eq!(Some(&Commit(7, Assertive)), xdb.get(&Uuid::nil()));
}

#[test]
fn xdb_assign_assertive_over_assertive_with_conflict() {
    let mut xdb = Xdb::new();
    assert_eq!(Ok(New(&Commit(7, Assertive))),
               xdb.assign(Uuid::nil(), &Commit(7, Assertive)));
    assert_eq!(Err(Conflict { existing: &Commit(7, Assertive), new: &Abort(Staleness, Assertive)}),
               xdb.assign(Uuid::nil(), &Abort(Staleness, Assertive)));
    assert_eq!(Some(&Commit(7, Assertive)), xdb.get(&Uuid::nil()));
}

#[test]
fn xdb_assign_assertive_over_permissive_without_conflict() {
    let mut xdb = Xdb::new();
    assert_eq!(Ok(New(&Commit(7, Permissive))),
               xdb.assign(Uuid::nil(), &Commit(7, Permissive)));
    // should upgrade to assertive
    assert_eq!(Ok(Existing(&Commit(9, Assertive))),
               xdb.assign(Uuid::nil(), &Commit(9, Assertive)));
    assert_eq!(Some(&Commit(9, Assertive)), xdb.get(&Uuid::nil()));
}

#[test]
fn xdb_assign_assertive_over_permissive_with_conflict() {
    let mut xdb = Xdb::new();
    assert_eq!(Ok(New(&Commit(7, Permissive))),
               xdb.assign(Uuid::nil(), &Commit(7, Permissive)));
    assert_eq!(Err(Conflict { existing: &Commit(7, Permissive), new: &Abort(Staleness, Assertive)}),
               xdb.assign(Uuid::nil(), &Abort(Staleness, Assertive)));
    assert_eq!(Some(&Commit(7, Permissive)), xdb.get(&Uuid::nil()));
}

#[test]
fn xdb_assign_permissive_over_assertive_without_conflict() {
    let mut xdb = Xdb::new();
    assert_eq!(Ok(New(&Commit(7, Assertive))),
               xdb.assign(Uuid::nil(), &Commit(7, Assertive)));
    assert_eq!(Ok(Existing(&Commit(7, Assertive))),
               xdb.assign(Uuid::nil(), &Commit(9, Permissive)));
    assert_eq!(Some(&Commit(7, Assertive)), xdb.get(&Uuid::nil()));
}

#[test]
fn xdb_assign_permissive_over_assertive_with_conflict() {
    let mut xdb = Xdb::new();
    assert_eq!(Ok(New(&Commit(7, Assertive))),
               xdb.assign(Uuid::nil(), &Commit(7, Assertive)));
    assert_eq!(Err(Conflict { existing: &Commit(7, Assertive), new: &Abort(Staleness, Permissive)}),
               xdb.assign(Uuid::nil(), &Abort(Staleness, Permissive)));
    assert_eq!(Some(&Commit(7, Assertive)), xdb.get(&Uuid::nil()));
}

#[test]
fn xdb_assign_permissive_over_permissive_without_conflict() {
    let mut xdb = Xdb::new();
    assert_eq!(Ok(New(&Commit(7, Permissive))),
               xdb.assign(Uuid::nil(), &Commit(7, Permissive)));
    assert_eq!(Ok(Existing(&Commit(7, Permissive))),
               xdb.assign(Uuid::nil(), &Commit(9, Permissive)));
    assert_eq!(Some(&Commit(7, Permissive)), xdb.get(&Uuid::nil()));
}

#[test]
fn xdb_assign_permissive_over_permissive_with_conflict() {
    let mut xdb = Xdb::new();
    assert_eq!(Ok(New(&Commit(7, Permissive))),
               xdb.assign(Uuid::nil(), &Commit(7, Permissive)));
    assert_eq!(Ok(Existing(&Commit(7, Permissive))),
               xdb.assign(Uuid::nil(), &Abort(Staleness, Permissive)));
    assert_eq!(Some(&Commit(7, Permissive)), xdb.get(&Uuid::nil()));
}