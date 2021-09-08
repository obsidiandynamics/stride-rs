use crate::havoc::Sublevel;
use crate::havoc::model::{Call, Trace};


impl Call {
    pub(crate) fn of(action: usize, rands: &[u64]) -> Self {
        Call { action, rands: rands.to_vec() }
    }
}

impl Trace {
    pub(crate) fn of(calls: &[Call]) -> Self {
        Trace { stack: calls.to_vec() }
    }
}

#[test]
fn sublevel_allows() {
    assert!(!Sublevel::Off.allows(Sublevel::Fine));
    assert!(Sublevel::Fine.allows(Sublevel::Fine));
    assert!(!Sublevel::Fine.allows(Sublevel::Finer));
    assert!(Sublevel::Finer.allows(Sublevel::Finer));
    assert!(Sublevel::Finer.allows(Sublevel::Fine));
}