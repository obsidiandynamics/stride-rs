use crate::havoc::model::Retention::Strong;

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

pub trait Context {
    fn name(&self) -> &str;

    fn rand(&mut self) -> u64;
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

    pub fn action<F>(&mut self, name: String, retention: Retention, action: F)
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
        self.action(name, retention, action);
        self
    }

    pub fn name(&mut self, name: String) {
        self.name = Some(name);
    }

    pub fn with_name(mut self, name: String) -> Self {
        self.name(name);
        self
    }

    pub(crate) fn strong_count(&self) -> usize {
        self.actions
            .iter()
            .filter(|entry| entry.retention == Strong)
            .count()
    }
}

#[derive(Debug)]
pub struct Trace {
    pub stack: Vec<Call>
}

#[derive(Debug)]
pub struct Call {
    pub action: usize,
    pub rands: Vec<u64>
}

impl Trace {
    pub(crate) fn new() -> Self {
        Trace { stack: vec![] }
    }

    pub (crate) fn peek(&self) -> &Call {
        self.stack.last().unwrap()
    }

    pub(crate) fn peek_mut(&mut self) -> &mut Call {
        self.stack.last_mut().unwrap()
    }

    pub(crate) fn push_rand(&mut self, rand: u64) {
        self.peek_mut().rands.push(rand);
    }

    pub(crate) fn pop(&mut self) -> Call {
        self.stack.remove(self.stack.len() - 1)
    }
}