//! Variables (port of `var.c`).

use std::cell::RefCell;
use std::rc::Rc;

use crate::shell::{Code, Shell};
use crate::word::{list2strcolon, Word, Words};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeFn {
    None,
    /// convert `$path` to `$PATH`
    LittlePath,
    /// convert `$PATH` to `$path`
    BigPath,
}

#[derive(Debug)]
pub struct Var {
    pub name: Word,
    pub val: Words,
    pub changed: bool,
    /// the function's code vector and the pc of its start
    pub func: Option<(Code, usize)>,
    pub fnchanged: bool,
    pub changefn: ChangeFn,
}

impl Var {
    pub fn new(name: Word) -> Var {
        Var { name, val: Words::new(), changed: false, func: None, fnchanged: false, changefn: ChangeFn::None }
    }
}

pub type VarRef = Rc<RefCell<Var>>;

#[derive(Debug)]
pub struct LocalNode {
    pub var: VarRef,
    pub next: Locals,
}

/// Persistent list of local variables; threads share their parents' lists.
pub type Locals = Option<Rc<LocalNode>>;

impl Shell {
    /// `gvlook`: look up (creating if needed) a global variable.
    pub(crate) fn gvlook(&mut self, name: &[u8]) -> VarRef {
        if let Some(v) = self.gvar.get(name) {
            return v.clone();
        }
        let v = Rc::new(RefCell::new(Var::new(name.to_vec())));
        self.gvar.insert(name.to_vec(), v.clone());
        v
    }

    /// `vlook`: look up a variable, locals first.
    pub(crate) fn vlook(&mut self, name: &[u8]) -> VarRef {
        if let Some(q) = &self.runq {
            let mut l = q.local.clone();
            while let Some(node) = l {
                if node.var.borrow().name == name {
                    return node.var.clone();
                }
                l = node.next.clone();
            }
        }
        self.gvlook(name)
    }

    /// `_setvar`
    pub(crate) fn setvar_(&mut self, name: &[u8], val: Words, callfn: bool) {
        let v = self.vlook(name);
        let cf = {
            let mut vb = v.borrow_mut();
            vb.val = val;
            vb.changed = true;
            vb.changefn
        };
        if callfn {
            self.changefn(cf, &v);
        }
    }

    /// `setvar`
    pub(crate) fn setvar(&mut self, name: &[u8], val: Words) {
        self.setvar_(name, val, true);
    }

    pub(crate) fn changefn(&mut self, cf: ChangeFn, v: &VarRef) {
        match cf {
            ChangeFn::None => {}
            ChangeFn::LittlePath => self.littlepath(v),
            ChangeFn::BigPath => self.bigpath(v),
        }
    }

    /// `bigpath`: convert `$PATH` to `$path`.
    pub(crate) fn bigpath(&mut self, v: &VarRef) {
        let first = v.borrow().val.front().cloned();
        let p = match first {
            None => {
                self.setvar_(b"path", Words::new(), false);
                return;
            }
            Some(p) => p,
        };
        let mut w = Words::new();
        // Doesn't handle escaped colon nonsense.
        if !p.is_empty() {
            for comp in p.split(|&c| c == b':') {
                w.push_back(if comp.is_empty() { b".".to_vec() } else { comp.to_vec() });
            }
        }
        self.setvar_(b"path", w, false);
    }

    /// `littlepath`: convert `$path` to `$PATH`.
    pub(crate) fn littlepath(&mut self, v: &VarRef) {
        let p = list2strcolon(&v.borrow().val);
        let mut w = Words::new();
        w.push_back(p);
        // recompute $path to expose colon problems
        self.setvar_(b"PATH", w, true);
    }

    /// `pathinit`
    pub(crate) fn pathinit(&mut self) {
        let v = self.gvlook(b"path");
        v.borrow_mut().changefn = ChangeFn::LittlePath;
        let v = self.gvlook(b"PATH");
        v.borrow_mut().changefn = ChangeFn::BigPath;
        self.bigpath(&v);
    }
}
