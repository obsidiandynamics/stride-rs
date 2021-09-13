use crate::Record;

#[test]
fn compress() {
    assert_eq!((vec![], 10), Record::compress(vec![3, 6, 9], 10));
    assert_eq!((vec![6, 9], 4), Record::compress(vec![3, 6, 9], 4));
    assert_eq!((vec![6, 9], 3), Record::compress(vec![3, 6, 9], 1));
}