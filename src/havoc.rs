use std::ops::Deref;

pub struct Model<'a> {
    // actions: Vec<ActionEntry<'a>>
    actions: Vec<Box<dyn FnMut() -> ActionResult + 'a>>
}

struct Execution {
    
}

pub enum ActionResult {
    Ran,
    Blocked,
    Joined,
    Panicked
}

// type Action = FnMut() -> ActionResult;

// struct ActionEntry<'a> {
//     name: &'a str,
//     action: Box<dyn FnMut() -> ActionResult + 'a>,
// }

impl<'a> Model<'a> {
    pub fn new() -> Self {
        Model { actions: vec![] }
    }

    // fn add(&mut self, name: &'a str, action: &'a Action) {
    //     // self.actions.push(ActionEntry { name, action });
    // }

    fn push<F>(&mut self, name: &'a str, mut f: F)
        where F: FnMut() -> ActionResult + 'a {
        // self.actions.push(ActionEntry { name, action: Box::new(f) });
    }

    pub fn append(&mut self, action: Box<dyn FnMut() -> ActionResult + 'a>) {
        self.actions.push(action);
    }

    fn run(&mut self) {
        for entry in &mut self.actions {
            entry();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::havoc::ActionResult::*;
    use std::cell::{Cell, RefCell};
    use std::borrow::BorrowMut;
    use std::rc::Rc;

    #[test]
    fn one_shot() {
        let mut model = Model::new();
        let ran = Rc::new(Cell::new(false));
        // model.add("test", & || {
        //     ran = true;
        //     Joined
        // });
        // model.push("test", || {
        //     ran.set(true);
        //     Joined
        // });
        let ran_c = Rc::clone(&ran);
        model.append(Box::new(move || {
            ran_c.set(true);
            Joined
        }));
        model.run();
        // let r = ran.borrow();
        assert!(ran.get());
    }

    #[test]
    fn two() {
        let should_end = Cell::new(false);
        let mut model = Model::new();
        model.append(Box::new(|| {
            should_end.set(true);
            ActionResult::Ran
        }));
        model.run();
        assert!(should_end.get());
    }
}