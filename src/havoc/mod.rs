use crate::havoc::Sublevel::Off;

pub mod components;
pub mod model;
pub mod checker;
pub mod sim;

#[derive(Copy, Clone, Debug)]
pub enum Sublevel {
    Off,
    Fine,
    Finer,
    Finest,
}

impl Sublevel {
    #[inline]
    fn allows(self, other: Sublevel) -> bool {
        self as usize >= other as usize
    }

    #[inline]
    fn if_trace(self) -> Self {
        match log::log_enabled!(log::Level::Trace) {
            true => self,
            false => Off,
        }
    }
}

#[cfg(test)]
mod tests;