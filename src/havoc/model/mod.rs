use rand::prelude::StdRng;

pub enum ActionResult {
    Ran,
    Blocked,
    Joined,
    Panicked,
}

pub struct Model<'a, S> {
    pub(crate) setup: Box<dyn Fn() -> S + 'a>,
    pub(crate) actions: Vec<ActionEntry<'a, S>>,
    pub(crate) name: Option<String>
}

#[derive(PartialEq, Debug)]
pub enum Retention {
    Strong,
    Weak,
}

pub trait Context {
    fn name(&self) -> &str;

    fn rng(&self) -> StdRng;
}

pub(crate) struct ActionEntry<'a, S> {
    pub(crate) name: String,
    pub(crate) retention: Retention,
    pub(crate) action: Box<dyn Fn(&mut S, &dyn Context) -> ActionResult + 'a>,
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
            name: Option::None
        }
    }

    pub fn action<F>(&mut self, name: String, retention: Retention, action: F)
        where
            F: Fn(&mut S, &dyn Context) -> ActionResult + 'a,
    {
        self.actions.push(ActionEntry {
            name,
            retention,
            action: Box::new(action),
        });
    }

    pub fn with_action<F>(mut self, name: String, retention: Retention, action: F) -> Self
        where
            F: Fn(&mut S, &dyn Context) -> ActionResult + 'a,
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
}