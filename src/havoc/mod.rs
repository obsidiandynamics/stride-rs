use crate::havoc::Trace::Off;

pub mod model;
pub mod checker;
pub mod component;

#[derive(Copy, Clone, Debug)]
pub enum Trace {
    Off,
    Fine,
    Finer,
    Finest,
}

impl Trace {
    #[inline]
    fn allows(self, other: Trace) -> bool {
        self as usize >= other as usize
    }

    #[inline]
    fn conditional(self) -> Self {
        match log::log_enabled!(log::Level::Trace) {
            true => self,
            false => Off,
        }
    }
}

#[test]
fn trace_allows() {
    assert!(!Trace::Off.allows(Trace::Fine));
    assert!(Trace::Fine.allows(Trace::Fine));
    assert!(!Trace::Fine.allows(Trace::Finer));
    assert!(Trace::Finer.allows(Trace::Finer));
    assert!(Trace::Finer.allows(Trace::Fine));
}

#[cfg(test)]
mod tests;
