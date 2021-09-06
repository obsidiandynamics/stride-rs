use crate::havoc::Trace;

#[test]
fn trace_allows() {
    assert!(!Trace::Off.allows(Trace::Fine));
    assert!(Trace::Fine.allows(Trace::Fine));
    assert!(!Trace::Fine.allows(Trace::Finer));
    assert!(Trace::Finer.allows(Trace::Finer));
    assert!(Trace::Finer.allows(Trace::Fine));
}