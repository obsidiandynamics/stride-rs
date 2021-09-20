use crate::fixtures::xdb::Xdb;
use stride::examiner::Outcome::{Commit, Abort};
use stride::examiner::Discord::{Assertive, Permissive};
use uuid::Uuid;
use stride::examiner::AbortReason::Staleness;
use crate::fixtures::xdb::XdbAssignmentError::Conflict;

#[test]
fn xdb_assign_assertive_over_assertive_without_conflict() {
    let mut xdb = Xdb::new();
    assert_eq!(Ok(&Commit(7, Assertive)),
               xdb.assign(Uuid::from_u128(0u128), &Commit(7, Assertive)));
    assert_eq!(Ok(&Commit(7, Assertive)),
               xdb.assign(Uuid::from_u128(0u128), &Commit(9, Assertive)));
}

#[test]
fn xdb_assign_assertive_over_assertive_with_conflict() {
    let mut xdb = Xdb::new();
    assert_eq!(Ok(&Commit(7, Assertive)),
               xdb.assign(Uuid::from_u128(0u128), &Commit(7, Assertive)));
    assert_eq!(Err(Conflict { existing: &Commit(7, Assertive), new: &Abort(Staleness, Assertive)}),
               xdb.assign(Uuid::from_u128(0u128), &Abort(Staleness, Assertive)));
}

#[test]
fn xdb_assign_assertive_over_permissive_without_conflict() {
    let mut xdb = Xdb::new();
    assert_eq!(Ok(&Commit(7, Permissive)),
               xdb.assign(Uuid::from_u128(0u128), &Commit(7, Permissive)));
    assert_eq!(Ok(&Commit(7, Permissive)),
               xdb.assign(Uuid::from_u128(0u128), &Commit(9, Assertive)));
}

#[test]
fn xdb_assign_assertive_over_permissive_with_conflict() {
    let mut xdb = Xdb::new();
    assert_eq!(Ok(&Commit(7, Permissive)),
               xdb.assign(Uuid::from_u128(0u128), &Commit(7, Permissive)));
    assert_eq!(Err(Conflict { existing: &Commit(7, Permissive), new: &Abort(Staleness, Assertive)}),
               xdb.assign(Uuid::from_u128(0u128), &Abort(Staleness, Assertive)));
}

#[test]
fn xdb_assign_permissive_over_assertive_without_conflict() {
    let mut xdb = Xdb::new();
    assert_eq!(Ok(&Commit(7, Assertive)),
               xdb.assign(Uuid::from_u128(0u128), &Commit(7, Assertive)));
    assert_eq!(Ok(&Commit(7, Assertive)),
               xdb.assign(Uuid::from_u128(0u128), &Commit(9, Permissive)));
}

#[test]
fn xdb_assign_permissive_over_assertive_with_conflict() {
    let mut xdb = Xdb::new();
    assert_eq!(Ok(&Commit(7, Assertive)),
               xdb.assign(Uuid::from_u128(0u128), &Commit(7, Assertive)));
    assert_eq!(Err(Conflict { existing: &Commit(7, Assertive), new: &Abort(Staleness, Permissive)}),
               xdb.assign(Uuid::from_u128(0u128), &Abort(Staleness, Permissive)));
}

#[test]
fn xdb_assign_permissive_over_permissive_without_conflict() {
    let mut xdb = Xdb::new();
    assert_eq!(Ok(&Commit(7, Permissive)),
               xdb.assign(Uuid::from_u128(0u128), &Commit(7, Permissive)));
    assert_eq!(Ok(&Commit(7, Permissive)),
               xdb.assign(Uuid::from_u128(0u128), &Commit(9, Permissive)));
}

#[test]
fn xdb_assign_permissive_over_permissive_with_conflict() {
    let mut xdb = Xdb::new();
    assert_eq!(Ok(&Commit(7, Permissive)),
               xdb.assign(Uuid::from_u128(0u128), &Commit(7, Permissive)));
    assert_eq!(Ok(&Commit(7, Permissive)),
               xdb.assign(Uuid::from_u128(0u128), &Abort(Staleness, Permissive)));
}