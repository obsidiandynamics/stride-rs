use crate::suffix::{Suffix, SuffixEntry};
use crate::suffix::InsertError::Nonmonotonic;
use std::slice::Iter;
use std::ops::Range;
use std::iter::Zip;

impl Suffix {
    fn enumerate(&self) -> impl Iterator<Item = (u64, &Option<SuffixEntry>)> {
        self.range().into_iter().zip(self.items.iter())
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

#[test]
fn insert_0_uninitialized() {
    let mut suffix = Suffix::default();
    assert_eq!(Err(Nonmonotonic), suffix.insert(vec![], vec![], 0));
    assert_eq!(None, suffix.lwm());
    assert_eq!(None, suffix.get(0));
    assert_eq!((0..0), suffix.range());
}

struct Z<T>(T);

impl Into<Vec<String>> for Z<&[&str]> {
    fn into(self) -> Vec<String> {
        vectorize::<str>(self.0)
    }
}

// fn vec_str(slice: &[&str]) -> Vec<String> {
//     slice.iter().map(|s| (*s).into()).collect()
// }

fn vectorize<T>(slice: &[&T]) -> Vec<<T as ToOwned>::Owned> where T: ToOwned + ?Sized {
    slice.iter().map(|&s| s.to_owned()).collect()
}

// fn vectorize<T>(slice: &[T]) -> Vec<<T as ToOwned>::Owned> where T: ToOwned {
//     slice.iter().map(|s| (*s).to_owned()).collect()
// }

impl SuffixEntry {
    fn undecided(readset: &[&str], writeset: &[&str]) -> Self {
        Self {
            readset: Z(readset).into(),
            writeset: Z(writeset).into(),
            decided: false
        }
    }
}

#[test]
fn insert_dense() {
    let mut suffix = Suffix::default();
    assert_eq!(Ok(()), suffix.insert(vec!["r1".to_owned()], vec!["w1".into()], 1));
    assert_eq!(Some(1), suffix.lwm());
    assert_eq!(Some(2), suffix.hwm());
    assert_eq!((1..2), suffix.range());
    assert_eq!(None, suffix.get(0));
    assert_eq!(Some(&SuffixEntry::undecided(&["r1"], &["w1"])), suffix.get(1));
    assert_eq!(None, suffix.get(2));
    let entries = suffix.enumerate().collect::<Vec<_>>();
    assert_eq!(vec![(1, &Some(SuffixEntry::undecided(&["r1"], &["w1"])))], entries);

    assert_eq!(Ok(()), suffix.insert(vec!["r2".to_owned()], vec!["w2".into()], 2));
    assert_eq!(Some(1), suffix.lwm());
    assert_eq!(Some(3), suffix.hwm());
    assert_eq!((1..3), suffix.range());
    assert_eq!(None, suffix.get(0));
    assert_eq!(Some(&SuffixEntry::undecided(&["r1"], &["w1"])), suffix.get(1));
    assert_eq!(Some(&SuffixEntry::undecided(&["r2"], &["w2"])), suffix.get(2));
    assert_eq!(None, suffix.get(3));
    let entries = suffix.enumerate().collect::<Vec<_>>();
    assert_eq!(vec![(1, &Some(SuffixEntry::undecided(&["r1"], &["w1"]))),
                    (2, &Some(SuffixEntry::undecided(&["r2"], &["w2"])))], entries);
}