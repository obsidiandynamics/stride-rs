use crate::sortedvec::SortedVec;

#[test]
fn from_unsorted_vec() {
    assert_eq!(&[1, 2, 3, 4, 5], SortedVec::from(vec![2, 4, 3, 5, 1]).as_slice());
}

#[test]
fn insert_maintains_order() {
    let mut vec = SortedVec::default();
    assert_eq!(&[] as &[i32], vec.as_slice());
    assert_eq!(0, vec.len());
    vec.insert(2);
    assert_eq!(&[2], vec.as_slice());
    vec.insert(4);
    assert_eq!(&[2, 4], vec.as_slice());
    vec.insert(3);
    assert_eq!(&[2, 3, 4], vec.as_slice());
    vec.insert(5);
    assert_eq!(&[2, 3, 4, 5], vec.as_slice());
    vec.insert(1);
    assert_eq!(&[1, 2, 3, 4, 5], vec.as_slice());
    vec.insert(1);
    assert_eq!(&[1, 1, 2, 3, 4, 5], vec.as_slice());
    assert_eq!(6, vec.len());

    vec.clear();
    assert_eq!(0, vec.len());
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

#[test]
fn remove() {
    let mut vec = SortedVec::new(3);
    vec.insert(20);
    vec.insert(40);
    vec.insert(30);
    vec.insert(50);
    vec.insert(10);
    assert!(!vec.remove(&9));
    assert_eq!(&[10, 20, 30, 40, 50], vec.as_slice());
    assert!(!vec.remove(&11));
    assert_eq!(&[10, 20, 30, 40, 50], vec.as_slice());
    assert!(vec.remove(&10));
    assert_eq!(&[20, 30, 40, 50], vec.as_slice());
    assert!(vec.remove(&50));
    assert_eq!(&[20, 30, 40], vec.as_slice());
    assert!(vec.remove(&30));
    assert_eq!(&[20, 40], vec.as_slice());
    assert!(vec.remove(&20));
    assert_eq!(&[40], vec.as_slice());
    assert!(vec.remove(&40));
    assert_eq!(&[] as &[u64], vec.as_slice());
    assert_eq!(0, vec.len());
}