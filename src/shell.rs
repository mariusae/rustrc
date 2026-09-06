//! The interpreter state: everything that is a global in the C sources
//! lives in [`Shell`] (port of the globals in `exec.c`, `lex.c`, ...).

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::io::Io;
use crate::tree::Tree;
use crate::var::{LocalNode, Locals, Var, VarRef};
use crate::word::{Word, Words};

pub const NSTATUS: usize = 128; // ERRMAX
pub const NFLAG: usize = 128;
pub const NSIG: usize = 32;

/// Interpreter opcodes (the `X*` functions of `exec.c`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Op {
    Append,
    Async,
    Backq,
    Bang,
    Close,
    Conc,
    Count,
    Delfn,
    Dol,
    Qdol,
    Dup,
    Exit,
    False,
    Fn,
    For,
    Glob,
    Jump,
    Mark,
    Match,
    Pipe,
    Read,
    Rdwr,
    Rdfn,
    Return,
    Subshell,
    True,
    Word,
    Write,
    Pipefd,
    Case,
    Local,
    Unlocal,
    Assign,
    Simple,
    Popm,
    Rdcmds,
    Wastrue,
    If,
    Ifnot,
    Pipewait,
    Delhere,
    Popredir,
    Sub,
    Eflag,
    Settrue,
}

impl Op {
    pub fn name(self) -> &'static str {
        match self {
            Op::Append => "Xappend",
            Op::Async => "Xasync",
            Op::Backq => "Xbackq",
            Op::Bang => "Xbang",
            Op::Close => "Xclose",
            Op::Conc => "Xconc",
            Op::Count => "Xcount",
            Op::Delfn => "Xdelfn",
            Op::Dol => "Xdol",
            Op::Qdol => "Xqdol",
            Op::Dup => "Xdup",
            Op::Exit => "Xexit",
            Op::False => "Xfalse",
            Op::Fn => "Xfn",
            Op::For => "Xfor",
            Op::Glob => "Xglob",
            Op::Jump => "Xjump",
            Op::Mark => "Xmark",
            Op::Match => "Xmatch",
            Op::Pipe => "Xpipe",
            Op::Read => "Xread",
            Op::Rdwr => "Xrdwr",
            Op::Rdfn => "Xrdfn",
            Op::Return => "Xreturn",
            Op::Subshell => "Xsubshell",
            Op::True => "Xtrue",
            Op::Word => "Xword",
            Op::Write => "Xwrite",
            Op::Pipefd => "Xpipefd",
            Op::Case => "Xcase",
            Op::Local => "Xlocal",
            Op::Unlocal => "Xunlocal",
            Op::Assign => "Xassign",
            Op::Simple => "Xsimple",
            Op::Popm => "Xpopm",
            Op::Rdcmds => "Xrdcmds",
            Op::Wastrue => "Xwastrue",
            Op::If => "Xif",
            Op::Ifnot => "Xifnot",
            Op::Pipewait => "Xpipewait",
            Op::Delhere => "Xdelhere",
            Op::Popredir => "Xpopredir",
            Op::Sub => "Xsub",
            Op::Eflag => "Xeflag",
            Op::Settrue => "Xsettrue",
        }
    }
}

/// One slot of a code vector (`union code`).
#[derive(Debug, Clone)]
pub enum Instr {
    F(Op),
    I(i32),
    S(Word),
}

/// A code vector.  Slot 0 is the reference count in C; it is kept (as
/// `I(0)`) so that program counters are numbered identically.
pub type Code = Rc<Vec<Instr>>;

pub fn code_i(code: &Code, pc: usize) -> i32 {
    match &code[pc] {
        Instr::I(i) => *i,
        other => panic!("rc: expected integer at pc {}, found {:?}", pc, other),
    }
}

pub fn code_s(code: &Code, pc: usize) -> &Word {
    match &code[pc] {
        Instr::S(s) => s,
        other => panic!("rc: expected string at pc {}, found {:?}", pc, other),
    }
}

/// Redirection kinds (`redir.type`).
pub const ROPEN: i32 = 1; // dup2(from, to); close(from);
pub const RDUP: i32 = 2; // dup2(from, to);
pub const RCLOSE: i32 = 3; // close(from);

#[derive(Debug)]
pub struct Redir {
    pub ty: i32,
    pub from: i32,
    pub to: i32,
    pub next: RedirList,
}

/// The redirection stack is a persistent linked list so that a thread can
/// share its parent's list and push on top of it.
pub type RedirList = Option<Rc<Redir>>;

pub fn redir_ptr_eq(a: &RedirList, b: &RedirList) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(x), Some(y)) => Rc::ptr_eq(x, y),
        _ => false,
    }
}

pub type IoRef = Rc<RefCell<Io>>;

/// Port of `struct thread`.
#[derive(Debug)]
pub struct Thread {
    pub code: Code,
    /// `code[pc]` is the next instruction
    pub pc: usize,
    /// argument stack; the last element is the top (`runq->argv`)
    pub argv: Vec<Words>,
    pub redir: RedirList,
    /// redir inheritance point
    pub startredir: RedirList,
    pub local: Locals,
    /// file name in Xrdcmd
    pub cmdfile: Option<Word>,
    /// file descriptor for Xrdcmd
    pub cmdfd: Option<IoRef>,
    /// static `if not` checking
    pub iflast: bool,
    /// is cmdfd at eof?
    pub eof: i32,
    /// interactive?
    pub iflag: bool,
    pub lineno: i32,
    /// process for Xpipewait to wait for
    pub pid: i32,
    /// status for Xpipewait
    pub status: Word,
    /// who continues when this finishes
    pub ret: Option<Box<Thread>>,
}

/// A pending here document (`struct here`).
#[derive(Debug)]
pub struct Here {
    pub tag: Box<Tree>,
    pub name: Word,
}

/// How `Exit` behaves in the main process.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitMode {
    /// Terminate the process (the `rc` binary).
    Process,
    /// Unwind back to the embedding caller.
    Unwind,
}

/// Payload used to unwind out of the interpreter when an embedded shell
/// executes `exit` (see [`ExitMode::Unwind`]).
#[derive(Debug, Clone)]
pub struct ExitUnwind {
    pub status: Word,
}

/// The rc interpreter.  See the crate documentation for the public API.
pub struct Shell {
    // exec.c
    pub(crate) runq: Option<Box<Thread>>,
    pub(crate) gvar: HashMap<Word, VarRef>,
    pub(crate) mypid: i32,
    pub(crate) nerror: i32,
    pub(crate) ndot: i32,
    pub(crate) lastc: i32,
    pub(crate) err: Io,
    pub(crate) promptstr: Word,
    pub(crate) eflagok: bool,
    pub(crate) ifnot: bool,
    pub(crate) argv0: Word,
    pub(crate) exit_mode: ExitMode,
    pub(crate) exit_beenhere: bool,

    // flags (getflags.c)
    pub(crate) flag: Vec<Option<Vec<Word>>>,

    // lex.c
    pub(crate) tok: Vec<u8>,
    pub(crate) tok_overflow: bool,
    pub(crate) future: i32,
    pub(crate) peekc: i32,
    pub(crate) doprompt: bool,
    pub(crate) inquote: bool,
    pub(crate) incomm: bool,
    pub(crate) lastdol: bool,
    pub(crate) lastword: bool,
    pub(crate) yylval: Option<Box<Tree>>,

    // here.c
    pub(crate) here: Vec<Here>,
    pub(crate) here_ser: i32,

    // code.c
    pub(crate) codebuf: Vec<Instr>,

    // plan9ish.c
    pub(crate) waitpids: Vec<i32>,
    pub(crate) environ: Vec<Word>,
    pub(crate) envp: usize,
    pub(crate) wdirfd: i32,
    pub(crate) dot_first: bool,
    pub(crate) rcmain_path: Word,
    pub(crate) rdcmds_code: Code,
    pub(crate) dotcmds_code: Code,
    pub(crate) rdfns_code: Code,
}

impl Shell {
    /// `start`: begin executing `code` at `pc` on a new thread stacked on
    /// top of the current one.
    pub(crate) fn start(&mut self, code: Code, pc: usize, local: Locals) {
        let redir = self.runq.as_ref().and_then(|q| q.redir.clone());
        let ret = self.runq.take();
        let p = Thread {
            code,
            pc,
            argv: Vec::new(),
            redir: redir.clone(),
            startredir: redir,
            local,
            cmdfile: None,
            cmdfd: None,
            iflast: false,
            eof: 0,
            iflag: false,
            lineno: 1,
            pid: 0,
            status: Vec::new(),
            ret,
        };
        self.runq = Some(Box::new(p));
    }

    pub(crate) fn q(&mut self) -> &mut Thread {
        self.runq.as_mut().expect("rc: no running thread")
    }

    pub(crate) fn qref(&self) -> &Thread {
        self.runq.as_ref().expect("rc: no running thread")
    }

    /// `pushword`
    pub(crate) fn pushword(&mut self, wd: Word) {
        let q = self.q();
        match q.argv.last_mut() {
            Some(l) => l.push_front(wd),
            None => self.panic("pushword but no argv!"),
        }
    }

    /// `popword`
    pub(crate) fn popword(&mut self) {
        let q = self.q();
        match q.argv.last_mut() {
            Some(l) => {
                if l.pop_front().is_none() {
                    self.panic("popword but no word!");
                }
            }
            None => self.panic("popword but no argv!"),
        }
    }

    /// `pushlist`
    pub(crate) fn pushlist(&mut self) {
        self.q().argv.push(Words::new());
    }

    /// `poplist`
    pub(crate) fn poplist(&mut self) {
        if self.q().argv.pop().is_none() {
            self.panic("poplist but no argv");
        }
    }

    /// The top argument list (`runq->argv->words`).
    pub(crate) fn top(&mut self) -> &mut Words {
        self.q().argv.last_mut().expect("rc: no argv")
    }

    pub(crate) fn top_ref(&self) -> &Words {
        self.qref().argv.last().expect("rc: no argv")
    }

    /// The list below the top (`runq->argv->next->words`).
    pub(crate) fn next_list(&mut self, depth: usize) -> &mut Words {
        let q = self.q();
        let n = q.argv.len();
        &mut q.argv[n - 1 - depth]
    }

    /// `pushredir`
    pub(crate) fn pushredir(&mut self, ty: i32, from: i32, to: i32) {
        let q = self.q();
        let next = q.redir.take();
        q.redir = Some(Rc::new(Redir { ty, from, to, next }));
    }

    /// `newvar` pushed on the local list of the current thread
    /// (`runq->local = newvar(name, runq->local)`).
    pub(crate) fn push_local(&mut self, name: Word, val: Words) -> VarRef {
        let mut v = Var::new(name);
        v.val = val;
        v.changed = true;
        let v = Rc::new(RefCell::new(v));
        let q = self.q();
        let next = q.local.take();
        q.local = Some(Rc::new(LocalNode { var: v.clone(), next }));
        v
    }

    pub(crate) fn flag_set(&self, c: u8) -> bool {
        (c as usize) < NFLAG && self.flag[c as usize].is_some()
    }

    pub(crate) fn flag_arg(&self, c: u8) -> Option<&Word> {
        self.flag.get(c as usize).and_then(|f| f.as_ref()).and_then(|v| v.first())
    }

    /// `panic` (rc's, not Rust's): print a message and abort.
    pub(crate) fn panic(&mut self, s: &str) -> ! {
        self.err.pstr(b"rc: ");
        self.err.pstr(s.as_bytes());
        self.err.pchr(b'\n');
        self.err.flush();
        self.abort()
    }

    /// `Abort`
    pub(crate) fn abort(&mut self) -> ! {
        self.err.pstr(b"aborting\n");
        self.err.flush();
        self.exit(b"aborting")
    }
}
