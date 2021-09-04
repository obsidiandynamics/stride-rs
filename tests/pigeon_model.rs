use stride::*;
use stride::havoc::*;
use rustc_hash::FxHashMap;
use uuid::Uuid;

struct State {
    cohorts: Vec<Cohort>,
    examiner: Examiner,
    candidates: Vec<Message<Statemap>>,
    decisions: Vec<Message<Statemap>>
}

struct Statemap {
    changes: Vec<(usize, i32)>
}

impl Statemap {
    fn new(changes: Vec<(usize, i32)>) -> Self {
        Statemap { changes }
    }
}

struct Cohort {
    pending: Vec<Uuid>,
    replica: Replica
}

struct Replica {
    holes: Vec<(i32, u64)>,
    ver: u64
}

impl Replica {
    fn install_items(&mut self, statemap: &Statemap, ver: u64) {
        for &(change_item, change_value) in &statemap.changes {
            let existing = &mut self.holes[change_item];
            if ver > existing.1 {
                *existing = (change_value, ver);
            }
        }
    }

    fn install_ooo(&mut self, statemap: &Statemap, safepoint: u64, ver: u64) {
        if self.ver >= safepoint && ver > self.ver {
            self.install_items(statemap, ver);
        }
    }

    fn install_ser(&mut self, statemap: &Statemap, ver: u64) {
        if ver > self.ver {
            self.install_items(statemap, ver);
            self.ver = ver;
        }
    }
}

#[test]
fn replica_install_items() {
    let mut replica = Replica { holes: vec![(10, 5), (20, 5), (30, 5)], ver: 5 };

    // empty statemap at a newer version -- no change expected
    replica.install_items(&Statemap::new(vec![]), 6);
    assert_eq!(vec![(10, 5), (20, 5), (30, 5)], replica.holes);
    assert_eq!(5, replica.ver);

    // non-empty statemap at the same version -- no change expected
    replica.install_items(&Statemap::new(vec![(0, 11)]), 5);
    assert_eq!(vec![(10, 5), (20, 5), (30, 5)], replica.holes);
    assert_eq!(5, replica.ver);

    // non-empty statemap at a higher version -- expect changes
    replica.install_items(&Statemap::new(vec![(0, 11)]), 6);
    assert_eq!(vec![(11, 6), (20, 5), (30, 5)], replica.holes);
    assert_eq!(5, replica.ver);
}

#[test]
fn replica_install_ooo() {
    let mut replica = Replica { holes: vec![(10, 5), (20, 5), (30, 5)], ver: 5 };

    // non-empty statemap at the same safepoint and same version -- no change expected
    replica.install_ooo(&Statemap::new(vec![(0, 11)]), 5, 5);
    assert_eq!(vec![(10, 5), (20, 5), (30, 5)], replica.holes);
    assert_eq!(5, replica.ver);

    // non-empty statemap at the greater safepoint and greater version -- no change expected
    replica.install_ooo(&Statemap::new(vec![(0, 11)]), 6, 6);
    assert_eq!(vec![(10, 5), (20, 5), (30, 5)], replica.holes);
    assert_eq!(5, replica.ver);

    // non-empty statemap at the same safepoint and greater version -- expect changes
    replica.install_ooo(&Statemap::new(vec![(0, 11)]), 5, 6);
    assert_eq!(vec![(11, 6), (20, 5), (30, 5)], replica.holes);
    assert_eq!(5, replica.ver);
}

#[test]
fn replica_install_ser() {
    let mut replica = Replica { holes: vec![(10, 5), (20, 5), (30, 5)], ver: 5 };

    // non-empty statemap at the same version -- no change expected
    replica.install_ser(&Statemap::new(vec![(0, 11)]), 5);
    assert_eq!(vec![(10, 5), (20, 5), (30, 5)], replica.holes);
    assert_eq!(5, replica.ver);

    // non-empty statemap at a higher version -- expect changes
    replica.install_ser(&Statemap::new(vec![(0, 11)]), 6);
    assert_eq!(vec![(11, 6), (20, 5), (30, 5)], replica.holes);
    assert_eq!(6, replica.ver);
}

#[test]
fn test() {
}