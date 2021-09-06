use crate::havoc::Sublevel;

#[test]
fn sublevel_allows() {
    assert!(!Sublevel::Off.allows(Sublevel::Fine));
    assert!(Sublevel::Fine.allows(Sublevel::Fine));
    assert!(!Sublevel::Fine.allows(Sublevel::Finer));
    assert!(Sublevel::Finer.allows(Sublevel::Finer));
    assert!(Sublevel::Finer.allows(Sublevel::Fine));
}