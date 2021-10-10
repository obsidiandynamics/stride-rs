use crate::suffix::{Suffix, RetainedEntry, TruncatedEntry, AppendResult, CompleteResult, CompleteSkipReason};
use crate::suffix::AppendSkipReason::Nonmonotonic;

impl Suffix {
    fn enumerate(&self) -> impl Iterator<Item = (u64, &Option<RetainedEntry>)> {
        self.range().into_iter().zip(self.entries.iter())
    }
}

#[test]
fn lwm_hwm_get_uninitialized() {
    let suffix = Suffix::default();
    assert_eq!(None, suffix.lwm());
    assert_eq!(None, suffix.hwm());
    assert_eq!((0..0), suffix.range());
    assert_eq!(None, suffix.get(0));
    let mut it = suffix.enumerate();
    assert_eq!(None, it.next());
}

struct Z<T>(T);

impl Into<Vec<String>> for Z<&[&str]> {
    fn into(self) -> Vec<String> {
        vectorize::<str>(self.0)
    }
}

fn vectorize<T>(slice: &[&T]) -> Vec<<T as ToOwned>::Owned> where T: ToOwned + ?Sized {
    slice.iter().map(|&s| s.to_owned()).collect()
}

impl RetainedEntry {
    fn pending(readset: &[&str], writeset: &[&str]) -> Self {
        Self::new(readset, writeset, false)
    }

    fn completed(readset: &[&str], writeset: &[&str]) -> Self {
        Self::new(readset, writeset, true)
    }

    fn new(readset: &[&str], writeset: &[&str], completed: bool) -> Self {
        Self {
            readset: Z(readset).into(),
            writeset: Z(writeset).into(),
            completed
        }
    }
}

impl TruncatedEntry {
    fn new(ver: u64, readset: &[&str], writeset: &[&str]) -> Self {
        Self {
            ver,
            readset: Z(readset).into(),
            writeset: Z(writeset).into(),
        }
    }
}

#[test] #[should_panic(expected = "unsupported version 0")]
fn insert_unsupported_ver() {
    let _ = Suffix::default().append(vec![], vec![], 0);
}

#[test]
fn insert_dense() {
    let mut suffix = Suffix::default();
    assert_eq!(AppendResult::Appended, suffix.append(vec!["r1".into()], vec!["w1".into()], 1));
    assert_eq!(Some(1), suffix.lwm());
    assert_eq!(Some(2), suffix.hwm());
    assert_eq!((1..2), suffix.range());
    assert_eq!(None, suffix.get(0));
    assert_eq!(Some(&RetainedEntry::pending(&["r1"], &["w1"])), suffix.get(1));
    assert_eq!(None, suffix.get(2));
    assert_eq!(vec![(1, &Some(RetainedEntry::pending(&["r1"], &["w1"])))],
               suffix.enumerate().collect::<Vec<_>>());

    assert_eq!(AppendResult::Appended, suffix.append(vec!["r2".into()], vec!["w2".into()], 2));
    assert_eq!(Some(1), suffix.lwm());
    assert_eq!(Some(3), suffix.hwm());
    assert_eq!((1..3), suffix.range());
    assert_eq!(None, suffix.get(0));
    assert_eq!(Some(&RetainedEntry::pending(&["r1"], &["w1"])), suffix.get(1));
    assert_eq!(Some(&RetainedEntry::pending(&["r2"], &["w2"])), suffix.get(2));
    assert_eq!(None, suffix.get(3));
    assert_eq!(vec![(1, &Some(RetainedEntry::pending(&["r1"], &["w1"]))),
                    (2, &Some(RetainedEntry::pending(&["r2"], &["w2"])))],
               suffix.enumerate().collect::<Vec<_>>());

    // cannot insert below the high-water mark
    assert_eq!(AppendResult::Skipped(Nonmonotonic), suffix.append(vec![], vec![], 1));
    assert_eq!(AppendResult::Skipped(Nonmonotonic), suffix.append(vec![], vec![], 2));
    assert_eq!((1..3), suffix.range());
}


#[test]
fn insert_sparse() {
    let mut suffix = Suffix::default();
    assert_eq!(AppendResult::Appended, suffix.append(vec!["r3".into()], vec!["w3".into()], 3));
    assert_eq!(Some(3), suffix.lwm());
    assert_eq!(Some(4), suffix.hwm());
    assert_eq!((3..4), suffix.range());
    assert_eq!(None, suffix.get(0));
    assert_eq!(None, suffix.get(1));
    assert_eq!(None, suffix.get(2));
    assert_eq!(Some(&RetainedEntry::pending(&["r3"], &["w3"])), suffix.get(3));
    assert_eq!(None, suffix.get(4));
    assert_eq!(vec![(3, &Some(RetainedEntry::pending(&["r3"], &["w3"])))],
               suffix.enumerate().collect::<Vec<_>>());

    assert_eq!(AppendResult::Appended, suffix.append(vec!["r7".into()], vec!["w7".into()], 7));
    assert_eq!(Some(3), suffix.lwm());
    assert_eq!(Some(8), suffix.hwm());
    assert_eq!((3..8), suffix.range());
    assert_eq!(None, suffix.get(0));
    assert_eq!(None, suffix.get(2));
    assert_eq!(Some(&RetainedEntry::pending(&["r3"], &["w3"])), suffix.get(3));
    assert_eq!(None, suffix.get(4));
    assert_eq!(None, suffix.get(6));
    assert_eq!(Some(&RetainedEntry::pending(&["r7"], &["w7"])), suffix.get(7));
    assert_eq!(vec![(3, &Some(RetainedEntry::pending(&["r3"], &["w3"]))),
                    (4, &None),
                    (5, &None),
                    (6, &None),
                    (7, &Some(RetainedEntry::pending(&["r7"], &["w7"])))],
               suffix.enumerate().collect::<Vec<_>>());

    // cannot insert below the high-water mark
    assert_eq!(AppendResult::Skipped(Nonmonotonic), suffix.append(vec![], vec![], 2));
    assert_eq!(AppendResult::Skipped(Nonmonotonic), suffix.append(vec![], vec![], 3));
    assert_eq!(AppendResult::Skipped(Nonmonotonic), suffix.append(vec![], vec![], 7));
    assert_eq!((3..8), suffix.range());
}

#[test]
fn complete_uninitialized() {
    let mut suffix = Suffix::default();
    assert_eq!(None, suffix.highest_completed());
    assert_eq!(CompleteResult::Skipped(CompleteSkipReason::Uninitialized), suffix.complete(3));
    assert_eq!(None, suffix.highest_completed());
}

#[test]
fn complete_sparse_forward() {
    let mut suffix = Suffix::default();
    assert_eq!(AppendResult::Appended, suffix.append(vec!["r3".into()], vec![], 3));
    assert_eq!(AppendResult::Appended, suffix.append(vec!["r4".into()], vec![], 4));
    assert_eq!(AppendResult::Appended, suffix.append(vec!["r7".into()], vec![], 7));
    assert_eq!(AppendResult::Appended, suffix.append(vec!["r8".into()], vec![], 8));
    assert_eq!(Some(2), suffix.highest_completed());

    assert_eq!(CompleteResult::Skipped(CompleteSkipReason::Lapsed(3)), suffix.complete(2));
    assert_eq!(CompleteResult::Completed(3), suffix.complete(3));
    assert_eq!(CompleteResult::Completed(3), suffix.complete(3)); // complete is idempotent
    assert_eq!(Some(3), suffix.highest_completed());
    assert_eq!(CompleteResult::Completed(6), suffix.complete(4));
    assert_eq!(CompleteResult::Completed(6), suffix.complete(3)); // complete is idempotent
    assert_eq!(Some(6), suffix.highest_completed());
    assert_eq!(CompleteResult::Skipped(CompleteSkipReason::NoSuchCandidate), suffix.complete(5));
    assert_eq!(Some(6), suffix.highest_completed());
    assert_eq!(CompleteResult::Skipped(CompleteSkipReason::NoSuchCandidate), suffix.complete(6));
    assert_eq!(CompleteResult::Completed(7), suffix.complete(7));
    assert_eq!(Some(7), suffix.highest_completed());
    assert_eq!(CompleteResult::Completed(8), suffix.complete(8));
    assert_eq!(Some(8), suffix.highest_completed());
}

#[test]
fn complete_sparse_out_of_order() {
    let mut suffix = Suffix::default();
    assert_eq!(AppendResult::Appended, suffix.append(vec!["r3".into()], vec![], 3));
    assert_eq!(AppendResult::Appended, suffix.append(vec!["r4".into()], vec![], 4));
    assert_eq!(AppendResult::Appended, suffix.append(vec!["r7".into()], vec![], 7));
    assert_eq!(AppendResult::Appended, suffix.append(vec!["r8".into()], vec![], 8));
    assert_eq!(Some(2), suffix.highest_completed());

    assert_eq!(CompleteResult::Completed(2), suffix.complete(7));
    assert_eq!(Some(2), suffix.highest_completed());

    assert_eq!(CompleteResult::Completed(3), suffix.complete(3));
    assert_eq!(Some(3), suffix.highest_completed());
    assert_eq!(CompleteResult::Completed(3), suffix.complete(3)); // complete is idempotent

    assert_eq!(CompleteResult::Completed(7), suffix.complete(4));
    assert_eq!(Some(7), suffix.highest_completed());

    assert_eq!(CompleteResult::Completed(8), suffix.complete(8));
    assert_eq!(Some(8), suffix.highest_completed());
}

#[test] #[should_panic(expected = "uninitialized")]
fn truncate_uninitialized() {
    Suffix::default().truncate(1, 2);
}

#[test] #[should_panic(expected = "invalid min_extent (2), max_extent (1)")]
fn truncate_invalid_args() {
    let mut suffix = Suffix::default();
    assert_eq!(AppendResult::Appended, suffix.append(vec![], vec![], 3));
    suffix.truncate(2, 1);
}

fn collect<I>(opt: Option<I>) -> Option<Vec<TruncatedEntry>> where I: Iterator<Item = TruncatedEntry> {
    opt.map(|it| it.collect())
}

#[test]
fn truncate_none_completed() {
    let mut suffix = Suffix::default();
    assert_eq!(AppendResult::Appended, suffix.append(vec![], vec![], 3));
    assert_eq!(None, collect(suffix.truncate(1, 1)));
}

#[test]
fn truncate_one_completed_min_1_max_1() {
    let mut suffix = Suffix::default();
    assert_eq!(AppendResult::Appended, suffix.append(vec![], vec![], 3));
    assert_eq!(CompleteResult::Completed(3), suffix.complete(3));
    assert_eq!(None, collect(suffix.truncate(1, 1)));
    assert_eq!((3..4), suffix.range());
}

#[test]
fn truncate_two_completed_min_1_max_1() {
    let mut suffix = Suffix::default();
    assert_eq!(AppendResult::Appended, suffix.append(vec!["r3".into()], vec!["w3".into()], 3));
    assert_eq!(AppendResult::Appended, suffix.append(vec!["r4".into()], vec!["w4".into()], 4));
    assert_eq!(CompleteResult::Completed(3), suffix.complete(3));
    assert_eq!(CompleteResult::Completed(4), suffix.complete(4));
    assert_eq!((3..5), suffix.range());
    assert_eq!(Some(vec![TruncatedEntry::new(3, &["r3"], &["w3"])]),
               collect(suffix.truncate(1, 1)));
    assert_eq!((4..5), suffix.range());
}

#[test]
fn truncate_two_completed_min_1_max_2() {
    let mut suffix = Suffix::default();
    assert_eq!(AppendResult::Appended, suffix.append(vec!["r3".into()], vec!["w3".into()], 3));
    assert_eq!(AppendResult::Appended, suffix.append(vec!["r4".into()], vec!["w4".into()], 4));
    assert_eq!(CompleteResult::Completed(3), suffix.complete(3));
    assert_eq!(CompleteResult::Completed(4), suffix.complete(4));
    assert_eq!((3..5), suffix.range());
    assert_eq!(None, collect(suffix.truncate(1, 2)));
    assert_eq!((3..5), suffix.range());
}

#[test]
fn truncate_two_completed_one_pending_min_1_max_1() {
    let mut suffix = Suffix::default();
    assert_eq!(AppendResult::Appended, suffix.append(vec!["r3".into()], vec!["w3".into()], 3));
    assert_eq!(AppendResult::Appended, suffix.append(vec!["r4".into()], vec!["w4".into()], 4));
    assert_eq!(AppendResult::Appended, suffix.append(vec!["r5".into()], vec!["w5".into()], 5));
    assert_eq!(CompleteResult::Completed(3), suffix.complete(3));
    assert_eq!(CompleteResult::Completed(4), suffix.complete(4));
    assert_eq!((3..6), suffix.range());
    assert_eq!(Some(vec![TruncatedEntry::new(3, &["r3"], &["w3"]),
                         TruncatedEntry::new(4, &["r4"], &["w4"])]),
               collect(suffix.truncate(1, 1)));
    assert_eq!((5..6), suffix.range());
}

#[test]
fn truncate_two_completed_one_pending_min_1_max_2() {
    let mut suffix = Suffix::default();
    assert_eq!(AppendResult::Appended, suffix.append(vec!["r3".into()], vec!["w3".into()], 3));
    assert_eq!(AppendResult::Appended, suffix.append(vec!["r4".into()], vec!["w4".into()], 4));
    assert_eq!(AppendResult::Appended, suffix.append(vec!["r5".into()], vec!["w5".into()], 5));
    assert_eq!(CompleteResult::Completed(3), suffix.complete(3));
    assert_eq!(CompleteResult::Completed(4), suffix.complete(4));
    assert_eq!((3..6), suffix.range());
    assert_eq!(Some(vec![TruncatedEntry::new(3, &["r3"], &["w3"]),
                         TruncatedEntry::new(4, &["r4"], &["w4"])]),
               collect(suffix.truncate(1, 2)));
    assert_eq!((5..6), suffix.range());
}

#[test]
fn truncate_two_completed_one_pending_min_1_max_3() {
    let mut suffix = Suffix::default();
    assert_eq!(AppendResult::Appended, suffix.append(vec!["r3".into()], vec!["w3".into()], 3));
    assert_eq!(AppendResult::Appended, suffix.append(vec!["r4".into()], vec!["w4".into()], 4));
    assert_eq!(AppendResult::Appended, suffix.append(vec!["r5".into()], vec!["w5".into()], 5));
    assert_eq!(CompleteResult::Completed(3), suffix.complete(3));
    assert_eq!(CompleteResult::Completed(4), suffix.complete(4));
    assert_eq!((3..6), suffix.range());
    assert_eq!(None, collect(suffix.truncate(1, 3)));
    assert_eq!((3..6), suffix.range());
}

#[test]
fn truncate_two_completed_one_pending_min_2_max_2() {
    let mut suffix = Suffix::default();
    assert_eq!(AppendResult::Appended, suffix.append(vec!["r3".into()], vec!["w3".into()], 3));
    assert_eq!(AppendResult::Appended, suffix.append(vec!["r4".into()], vec!["w4".into()], 4));
    assert_eq!(AppendResult::Appended, suffix.append(vec!["r5".into()], vec!["w5".into()], 5));
    assert_eq!(CompleteResult::Completed(3), suffix.complete(3));
    assert_eq!(CompleteResult::Completed(4), suffix.complete(4));
    assert_eq!((3..6), suffix.range());
    assert_eq!(Some(vec![TruncatedEntry::new(3, &["r3"], &["w3"])]),
               collect(suffix.truncate(2, 2)));
    assert_eq!((4..6), suffix.range());
}

#[test]
fn truncate_two_completed_one_pending_min_2_max_3() {
    let mut suffix = Suffix::default();
    assert_eq!(AppendResult::Appended, suffix.append(vec!["r3".into()], vec!["w3".into()], 3));
    assert_eq!(AppendResult::Appended, suffix.append(vec!["r4".into()], vec!["w4".into()], 4));
    assert_eq!(AppendResult::Appended, suffix.append(vec!["r5".into()], vec!["w5".into()], 5));
    assert_eq!(CompleteResult::Completed(3), suffix.complete(3));
    assert_eq!(CompleteResult::Completed(4), suffix.complete(4));
    assert_eq!((3..6), suffix.range());
    assert_eq!(None,
               collect(suffix.truncate(2, 3)));
    assert_eq!((3..6), suffix.range());
}

#[test]
fn truncate_two_completed_one_pending_min_3_max_3() {
    let mut suffix = Suffix::default();
    assert_eq!(AppendResult::Appended, suffix.append(vec!["r3".into()], vec!["w3".into()], 3));
    assert_eq!(AppendResult::Appended, suffix.append(vec!["r4".into()], vec!["w4".into()], 4));
    assert_eq!(AppendResult::Appended, suffix.append(vec!["r5".into()], vec!["w5".into()], 5));
    assert_eq!(CompleteResult::Completed(3), suffix.complete(3));
    assert_eq!(CompleteResult::Completed(4), suffix.complete(4));
    assert_eq!((3..6), suffix.range());
    assert_eq!(None,
               collect(suffix.truncate(3, 3)));
    assert_eq!((3..6), suffix.range());
}

#[test]
fn truncate_three_completed_min_2_max_2_dense() {
    let mut suffix = Suffix::default();
    assert_eq!(AppendResult::Appended, suffix.append(vec!["r3".into()], vec!["w3".into()], 3));
    assert_eq!(AppendResult::Appended, suffix.append(vec!["r4".into()], vec!["w4".into()], 4));
    assert_eq!(AppendResult::Appended, suffix.append(vec!["r5".into()], vec!["w5".into()], 5));
    assert_eq!(CompleteResult::Completed(3), suffix.complete(3));
    assert_eq!(CompleteResult::Completed(4), suffix.complete(4));
    assert_eq!(CompleteResult::Completed(5), suffix.complete(5));
    assert_eq!((3..6), suffix.range());
    assert_eq!(Some(vec![TruncatedEntry::new(3, &["r3"], &["w3"])]),
               collect(suffix.truncate(2, 2)));
    assert_eq!((4..6), suffix.range());

    // truncate the remainder
    assert_eq!(Some(vec![TruncatedEntry::new(4, &["r4"], &["w4"])]),
               collect(suffix.truncate(1, 1)));
    assert_eq!((5..6), suffix.range());

    // truncate the remainder
    assert_eq!(None, collect(suffix.truncate(1, 1)));
    assert_eq!((5..6), suffix.range());

    // check leftovers
    assert_eq!(vec![(5, &Some(RetainedEntry::completed(&["r5"], &["w5"])))],
               suffix.enumerate().collect::<Vec<_>>());
}

#[test]
fn truncate_three_completed_min_2_max_2_sparse() {
    let mut suffix = Suffix::default();
    assert_eq!(AppendResult::Appended, suffix.append(vec!["r3".into()], vec!["w3".into()], 3));
    assert_eq!(AppendResult::Appended, suffix.append(vec!["r5".into()], vec!["w5".into()], 5));
    assert_eq!(CompleteResult::Completed(4), suffix.complete(3));
    assert_eq!(CompleteResult::Completed(5), suffix.complete(5));
    assert_eq!((3..6), suffix.range());
    assert_eq!(Some(vec![TruncatedEntry::new(3, &["r3"], &["w3"])]),
               collect(suffix.truncate(2, 2)));
    assert_eq!((4..6), suffix.range());

    // truncate the remainder
    assert_eq!(Some(vec![]), collect(suffix.truncate(1, 1)));
    assert_eq!((5..6), suffix.range());

    // truncate the remainder
    assert_eq!(None, collect(suffix.truncate(1, 1)));
    assert_eq!((5..6), suffix.range());

    // check leftovers
    assert_eq!(vec![(5, &Some(RetainedEntry::completed(&["r5"], &["w5"])))],
               suffix.enumerate().collect::<Vec<_>>());
}

#[test]
fn truncate_three_completed_min_1_max_1() {
    let mut suffix = Suffix::default();
    assert_eq!(AppendResult::Appended, suffix.append(vec!["r3".into()], vec!["w3".into()], 3));
    assert_eq!(AppendResult::Appended, suffix.append(vec!["r4".into()], vec!["w4".into()], 4));
    assert_eq!(AppendResult::Appended, suffix.append(vec!["r5".into()], vec!["w5".into()], 5));
    assert_eq!(AppendResult::Appended, suffix.append(vec!["r6".into()], vec!["w6".into()], 6));
    assert_eq!(CompleteResult::Completed(3), suffix.complete(3));
    assert_eq!(CompleteResult::Completed(4), suffix.complete(4));
    assert_eq!(CompleteResult::Completed(5), suffix.complete(5));
    assert_eq!((3..7), suffix.range());
    assert_eq!(Some(vec![TruncatedEntry::new(3, &["r3"], &["w3"]),
                         TruncatedEntry::new(4, &["r4"], &["w4"]),
                         TruncatedEntry::new(5, &["r5"], &["w5"])]),
               collect(suffix.truncate(1, 2)));
    assert_eq!((6..7), suffix.range());

    // truncate the remainder
    assert_eq!(None, collect(suffix.truncate(1, 1)));
    assert_eq!((6..7), suffix.range());

    // check leftovers
    assert_eq!(vec![(6, &Some(RetainedEntry::pending(&["r6"], &["w6"])))],
               suffix.enumerate().collect::<Vec<_>>());
}