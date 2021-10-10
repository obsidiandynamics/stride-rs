use crate::sortedvec::SortedVec;

#[test]
fn from_unsorted_vec() {
    assert_eq!(vec![1, 2, 3, 4, 5], SortedVec::from(vec![2, 4, 3, 5, 1]).items);
}

#[test]
fn insert_maintains_order() {
    let mut vec = SortedVec::default();
    vec.insert(2);
    assert_eq!(vec![2], vec.items);
    vec.insert(4);
    assert_eq!(vec![2, 4], vec.items);
    vec.insert(3);
    assert_eq!(vec![2, 3, 4], vec.items);
    vec.insert(5);
    assert_eq!(vec![2, 3, 4, 5], vec.items);
    vec.insert(1);
    assert_eq!(vec![1, 2, 3, 4, 5], vec.items);
}

#[test]
fn contains() {
    let mut vec = SortedVec::new(3);
    vec.insert(20);
    vec.insert(40);
    vec.insert(30);
    vec.insert(50);
    vec.insert(10);
    assert!(vec.contains(&10));
    assert!(vec.contains(&20));
    assert!(vec.contains(&30));
    assert!(vec.contains(&40));
    assert!(vec.contains(&50));
    assert!(!vec.contains(&9));
    assert!(!vec.contains(&11));
}