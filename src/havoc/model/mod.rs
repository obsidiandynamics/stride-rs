use crate::havoc::model::Retention::Strong;
use std::fmt::{Display, Formatter};
use core::fmt;
use std::borrow::Cow;

pub enum ActionResult {
    Ran,
    Blocked,
    Joined,
    Breached(String),
}

pub struct Model<'a, S> {
    pub(crate) setup: Box<dyn Fn() -> S + 'a>,
    pub(crate) actions: Vec<ActionEntry<'a, S>>,
    pub(crate) name: Option<String>,
}

#[derive(PartialEq, Debug)]
pub enum Retention {
    Strong,
    Weak,
}

pub fn rand_element<'a, T>(c: &mut dyn Context, slice: &'a [T]) -> &'a T {
    let rand = c.rand(slice.len() as u64);
    &slice[rand as usize]
}

pub trait Context {
    fn name(&self) -> &str;

    fn rand(&mut self, limit: u64) -> u64;

    fn trace(&self) -> Cow<Trace>;
}

pub(crate) struct ActionEntry<'a, S> {
    pub(crate) name: String,
    pub(crate) retention: Retention,
    pub(crate) action: Box<dyn Fn(&mut S, &mut dyn Context) -> ActionResult + 'a>,
}

pub fn name_of<T>(_: &T) -> &'static str {
    std::any::type_name::<T>()
}

impl<'a, S> Model<'a, S> {
    pub fn new<G>(setup: G) -> Self
    where
        G: Fn() -> S + 'a,
    {
        Model {
            setup: Box::new(setup),
            actions: vec![],
            name: Option::None,
        }
    }

    pub fn add_action<F>(&mut self, name: String, retention: Retention, action: F)
    where
        F: Fn(&mut S, &mut dyn Context) -> ActionResult + 'a,
    {
        self.actions.push(ActionEntry {
            name,
            retention,
            action: Box::new(action),
        });
    }

    pub fn with_action<F>(mut self, name: String, retention: Retention, action: F) -> Self
    where
        F: Fn(&mut S, &mut dyn Context) -> ActionResult + 'a,
    {
        self.add_action(name, retention, action);
        self
    }

    pub fn name(&self) -> Option<&str> {
        self.name.as_ref().map(|name| name.as_str())
    }

    pub fn set_name(&mut self, name: String) {
        self.name = Some(name);
    }

    pub fn with_name(mut self, name: String) -> Self {
        self.set_name(name);
        self
    }

    pub(crate) fn strong_count(&self) -> usize {
        self.actions
            .iter()
            .filter(|entry| entry.retention == Strong)
            .count()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Call {
    pub action: usize,
    pub rands: Vec<u64>
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Trace {
    pub calls: Vec<Call>
}

impl Trace {
    pub(crate) fn new() -> Self {
        Trace { calls: vec![] }
    }

    pub(crate) fn peek(&self) -> &Call {
        self.calls.last().unwrap()
    }

    pub(crate) fn peek_mut(&mut self) -> &mut Call {
        self.calls.last_mut().unwrap()
    }

    pub(crate) fn push_rand(&mut self, rand: u64) {
        self.peek_mut().rands.push(rand);
    }

    pub(crate) fn pop(&mut self) -> Call {
        self.calls.remove(self.calls.len() - 1)
    }

    pub fn prettify<'a, S>(&'a self, model: &'a Model<'a, S>) -> PrettyTrace<'a, S> {
        PrettyTrace { trace: self, model }
    }
}

pub struct PrettyTrace<'a, S> {
    pub trace: &'a Trace,
    pub model: &'a Model<'a, S>
}

impl<S> Display for PrettyTrace<'_, S> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        for (stack_index, call) in self.trace.calls.iter().enumerate() {
            let action_entry = self.model.actions.get(call.action).ok_or(fmt::Error)?;
            writeln!(f, "{: >3}: {}", stack_index, &action_entry.name)?;
            if call.rands.is_empty() {
                writeln!(f, "      -")?;
            } else {
                writeln!(f, "      rands: {:?}", call.rands)?;
            }
        }
        Ok(())
    }
}