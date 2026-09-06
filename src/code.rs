//! The compiler from parse trees to code vectors (port of `code.c`).

use std::rc::Rc;

use crate::shell::{Code, Instr, Op, Shell};
use crate::tree::*;

impl Shell {
    fn emitf(&mut self, op: Op) -> usize {
        self.codebuf.push(Instr::F(op));
        self.codebuf.len() - 1
    }

    fn emiti(&mut self, i: i32) -> usize {
        self.codebuf.push(Instr::I(i));
        self.codebuf.len() - 1
    }

    fn emits(&mut self, s: &[u8]) -> usize {
        self.codebuf.push(Instr::S(s.to_vec()));
        self.codebuf.len() - 1
    }

    fn stuffdot(&mut self, a: usize) {
        if self.codebuf.len() <= a {
            self.panic(&format!("Bad address {} in stuffdot", a));
        }
        self.codebuf[a] = Instr::I(self.codebuf.len() as i32);
    }

    /// `compile`: returns `true` on success, leaving the code in
    /// `self.codebuf` (retrieve it with [`Shell::take_codebuf`]).
    pub(crate) fn compile(&mut self, t: Option<&Tree>) -> bool {
        let mut t = t;
        if self.flag_set(b'D') {
            let mut s = crate::io::Io::openstr();
            s.pstr(b"compile: ");
            crate::pcmd::pcmdu(&mut s, t);
            s.pchr(b'\n');
            crate::unix::write_all(2, s.strp());
            if self.eflagok {
                // made it out of rcmain - stop executing commands, just print them
                t = None;
            }
        }
        self.codebuf = Vec::with_capacity(100);
        self.emiti(0); // reference count
        let eflag = self.flag_set(b'e');
        self.outcode(t, eflag);
        if self.nerror != 0 {
            self.codebuf = Vec::new();
            return false;
        }
        self.readhere();
        self.emitf(Op::Return);
        self.emiti(0);
        true
    }

    pub(crate) fn take_codebuf(&mut self) -> Code {
        Rc::new(std::mem::take(&mut self.codebuf))
    }

    /// `cleanhere`
    pub(crate) fn cleanhere(&mut self, f: &[u8]) {
        self.emitf(Op::Delhere);
        self.emits(f);
    }

    /// `fnstr`: the printed form of a function body, with `;` for newlines.
    pub(crate) fn fnstr(t: Option<&Tree>) -> Vec<u8> {
        let mut f = crate::io::Io::openstr();
        crate::pcmd::pcmd(&mut f, t, b';');
        f.into_string()
    }

    fn set_iflast(&mut self, v: bool) {
        if let Some(q) = self.runq.as_mut() {
            q.iflast = v;
        }
    }

    fn get_iflast(&self) -> bool {
        self.runq.as_ref().map(|q| q.iflast).unwrap_or(false)
    }

    /// `outcode`
    pub(crate) fn outcode(&mut self, t: Option<&Tree>, eflag: bool) {
        let t = match t {
            None => return,
            Some(t) => t,
        };
        let c0 = t.child[0].as_deref();
        let c1 = t.child[1].as_deref();
        let c2 = t.child[2].as_deref();
        if t.ty != NOT && t.ty != b';' as i32 {
            self.set_iflast(false);
        }
        match t.ty {
            0x24 /* $ */ => {
                self.emitf(Op::Mark);
                self.outcode(c0, eflag);
                self.emitf(Op::Dol);
            }
            0x22 /* " */ => {
                self.emitf(Op::Mark);
                self.outcode(c0, eflag);
                self.emitf(Op::Qdol);
            }
            SUB => {
                self.emitf(Op::Mark);
                self.outcode(c0, eflag);
                self.emitf(Op::Mark);
                self.outcode(c1, eflag);
                self.emitf(Op::Sub);
            }
            0x26 /* & */ => {
                self.emitf(Op::Async);
                let p = self.emiti(0);
                self.outcode(c0, eflag);
                self.emitf(Op::Exit);
                self.stuffdot(p);
            }
            0x3b /* ; */ => {
                self.outcode(c0, eflag);
                self.outcode(c1, eflag);
            }
            0x5e /* ^ */ => {
                self.emitf(Op::Mark);
                self.outcode(c1, eflag);
                self.emitf(Op::Mark);
                self.outcode(c0, eflag);
                self.emitf(Op::Conc);
            }
            0x60 /* ` */ => {
                self.emitf(Op::Mark);
                if c0.is_some() {
                    self.outcode(c0, false);
                    self.emitf(Op::Glob);
                } else {
                    self.emitf(Op::Mark);
                    self.emitf(Op::Word);
                    self.emits(b"ifs");
                    self.emitf(Op::Dol);
                }
                self.emitf(Op::Backq);
                let p = self.emiti(0);
                self.outcode(c1, false);
                self.emitf(Op::Exit);
                self.stuffdot(p);
            }
            ANDAND => {
                self.outcode(c0, false);
                self.emitf(Op::True);
                let p = self.emiti(0);
                self.outcode(c1, eflag);
                self.stuffdot(p);
            }
            ARGLIST => {
                self.outcode(c1, eflag);
                self.outcode(c0, eflag);
            }
            BANG => {
                self.outcode(c0, eflag);
                self.emitf(Op::Bang);
            }
            PCMD | BRACE => {
                self.outcode(c0, eflag);
            }
            COUNT => {
                self.emitf(Op::Mark);
                self.outcode(c0, eflag);
                self.emitf(Op::Count);
            }
            FN => {
                self.emitf(Op::Mark);
                self.outcode(c0, eflag);
                if c1.is_some() {
                    self.emitf(Op::Fn);
                    let p = self.emiti(0);
                    let s = Shell::fnstr(c1);
                    self.emits(&s);
                    self.outcode(c1, eflag);
                    self.emitf(Op::Unlocal); // get rid of $*
                    self.emitf(Op::Return);
                    self.stuffdot(p);
                } else {
                    self.emitf(Op::Delfn);
                }
            }
            IF => {
                self.outcode(c0, false);
                self.emitf(Op::If);
                let p = self.emiti(0);
                self.outcode(c1, eflag);
                self.emitf(Op::Wastrue);
                self.stuffdot(p);
            }
            NOT => {
                if !self.get_iflast() {
                    self.yyerror("`if not' does not follow `if(...)'");
                }
                self.emitf(Op::Ifnot);
                let p = self.emiti(0);
                self.outcode(c0, eflag);
                self.stuffdot(p);
            }
            OROR => {
                self.outcode(c0, false);
                self.emitf(Op::False);
                let p = self.emiti(0);
                self.outcode(c1, eflag);
                self.stuffdot(p);
            }
            PAREN => {
                self.outcode(c0, eflag);
            }
            SIMPLE => {
                self.emitf(Op::Mark);
                self.outcode(c0, eflag);
                self.emitf(Op::Simple);
                if eflag {
                    self.emitf(Op::Eflag);
                }
            }
            SUBSHELL => {
                self.emitf(Op::Subshell);
                let p = self.emiti(0);
                self.outcode(c0, eflag);
                self.emitf(Op::Exit);
                self.stuffdot(p);
                if eflag {
                    self.emitf(Op::Eflag);
                }
            }
            SWITCH => {
                self.codeswitch(t, eflag);
            }
            TWIDDLE => {
                self.emitf(Op::Mark);
                self.outcode(c1, eflag);
                self.emitf(Op::Mark);
                self.outcode(c0, eflag);
                self.emitf(Op::Match);
                if eflag {
                    self.emitf(Op::Eflag);
                }
            }
            WHILE => {
                let q = self.codebuf.len();
                self.outcode(c0, false);
                if q == self.codebuf.len() {
                    self.emitf(Op::Settrue); // empty condition == while(true)
                }
                self.emitf(Op::True);
                let p = self.emiti(0);
                self.outcode(c1, eflag);
                self.emitf(Op::Jump);
                self.emiti(q as i32);
                self.stuffdot(p);
            }
            WORDS => {
                self.outcode(c1, eflag);
                self.outcode(c0, eflag);
            }
            FOR => {
                self.emitf(Op::Mark);
                if c1.is_some() {
                    self.outcode(c1, eflag);
                    self.emitf(Op::Glob);
                } else {
                    self.emitf(Op::Mark);
                    self.emitf(Op::Word);
                    self.emits(b"*");
                    self.emitf(Op::Dol);
                }
                self.emitf(Op::Mark); // dummy value for Xlocal
                self.emitf(Op::Mark);
                self.outcode(c0, eflag);
                self.emitf(Op::Local);
                let p = self.emitf(Op::For);
                let q = self.emiti(0);
                self.outcode(c2, eflag);
                self.emitf(Op::Jump);
                self.emiti(p as i32);
                self.stuffdot(q);
                self.emitf(Op::Unlocal);
            }
            WORD => {
                self.emitf(Op::Word);
                let s = t.s.clone().unwrap_or_default();
                self.emits(&s);
            }
            DUP => {
                if t.rtype == DUPFD {
                    self.emitf(Op::Dup);
                    self.emiti(t.fd0);
                    self.emiti(t.fd1);
                } else {
                    self.emitf(Op::Close);
                    self.emiti(t.fd0);
                }
                self.outcode(c1, eflag);
                self.emitf(Op::Popredir);
            }
            PIPEFD => {
                self.emitf(Op::Pipefd);
                self.emiti(t.rtype);
                let p = self.emiti(0);
                self.outcode(c0, eflag);
                self.emitf(Op::Exit);
                self.stuffdot(p);
            }
            REDIR => {
                self.emitf(Op::Mark);
                self.outcode(c0, eflag);
                self.emitf(Op::Glob);
                match t.rtype {
                    APPEND => {
                        self.emitf(Op::Append);
                    }
                    WRITE => {
                        self.emitf(Op::Write);
                    }
                    READ | HERE => {
                        self.emitf(Op::Read);
                    }
                    RDWR => {
                        self.emitf(Op::Rdwr);
                    }
                    _ => {}
                }
                self.emiti(t.fd0);
                self.outcode(c1, eflag);
                self.emitf(Op::Popredir);
            }
            0x3d /* = */ => {
                // Is there a command after the assignments?
                let mut tt = Some(t);
                while let Some(x) = tt {
                    if x.ty != b'=' as i32 {
                        break;
                    }
                    tt = x.child[2].as_deref();
                }
                if let Some(cmd) = tt {
                    let mut x = t;
                    let mut n = 0;
                    while x.ty == b'=' as i32 {
                        self.emitf(Op::Mark);
                        self.outcode(x.child[1].as_deref(), eflag);
                        self.emitf(Op::Mark);
                        self.outcode(x.child[0].as_deref(), eflag);
                        self.emitf(Op::Local);
                        n += 1;
                        x = x.child[2].as_deref().unwrap();
                    }
                    self.outcode(Some(cmd), eflag);
                    for _ in 0..n {
                        self.emitf(Op::Unlocal);
                    }
                } else {
                    let mut x = Some(t);
                    while let Some(y) = x {
                        self.emitf(Op::Mark);
                        self.outcode(y.child[1].as_deref(), eflag);
                        self.emitf(Op::Mark);
                        self.outcode(y.child[0].as_deref(), eflag);
                        self.emitf(Op::Assign);
                        x = y.child[2].as_deref();
                    }
                }
            }
            PIPE => {
                self.emitf(Op::Pipe);
                self.emiti(t.fd0);
                self.emiti(t.fd1);
                let p = self.emiti(0);
                let q = self.emiti(0);
                self.outcode(c0, eflag);
                self.emitf(Op::Exit);
                self.stuffdot(p);
                self.outcode(c1, eflag);
                self.emitf(Op::Return);
                self.stuffdot(q);
                self.emitf(Op::Pipewait);
            }
            _ => {
                self.err.pstr(b"bad type ");
                self.err.pdec(t.ty as i64);
                self.err.pstr(b" in outcode\n");
                self.err.flush();
            }
        }
        if t.ty != NOT && t.ty != b';' as i32 {
            self.set_iflast(t.ty == IF);
        } else if let Some(c0) = c0 {
            self.set_iflast(c0.ty == IF);
        }
    }

    /*
     * switch code looks like this:
     *	Xmark
     *	(get switch value)
     *	Xjump	1f
     * out:	Xjump	leave
     * 1:	Xmark
     *	(get case values)
     *	Xcase	1f
     *	(commands)
     *	Xjump	out
     * 1:	Xmark
     *	(get case values)
     *	Xcase	1f
     *	(commands)
     *	Xjump	out
     * 1:
     * leave:
     *	Xpopm
     */
    fn codeswitch(&mut self, t: &Tree, eflag: bool) {
        let c0 = t.child[0].as_deref();
        let c1 = t.child[1].as_deref().expect("rc: switch without brace");
        let body = match c1.child[0].as_deref() {
            Some(b) if b.ty == b';' as i32 && iscase(b.child[0].as_deref()) => b,
            _ => {
                self.yyerror("case missing in switch");
                return;
            }
        };
        self.emitf(Op::Mark);
        self.outcode(c0, eflag);
        self.emitf(Op::Jump);
        let mut nextcase = self.emiti(0);
        let out = self.emitf(Op::Jump);
        let leave = self.emiti(0);
        self.stuffdot(nextcase);
        let mut t: &Tree = body;
        while t.ty == b';' as i32 {
            let tt = t.child[1].as_deref();
            self.emitf(Op::Mark);
            // emit the case patterns (the arguments after `case`)
            let mut u = t.child[0].as_deref().unwrap().child[0].as_deref().unwrap();
            while u.ty == ARGLIST {
                self.outcode(u.child[1].as_deref(), eflag);
                u = u.child[0].as_deref().unwrap();
            }
            self.emitf(Op::Case);
            nextcase = self.emiti(0);
            let mut cur = tt;
            loop {
                match cur {
                    Some(x) if x.ty == b';' as i32 => {
                        if iscase(x.child[0].as_deref()) {
                            break;
                        }
                        self.outcode(x.child[0].as_deref(), eflag);
                        cur = x.child[1].as_deref();
                    }
                    Some(x) => {
                        if !iscase(Some(x)) {
                            self.outcode(Some(x), eflag);
                        }
                        break;
                    }
                    None => break,
                }
            }
            self.emitf(Op::Jump);
            self.emiti(out as i32);
            self.stuffdot(nextcase);
            t = match cur {
                Some(x) => x,
                None => break,
            };
        }
        self.stuffdot(leave);
        self.emitf(Op::Popm);
    }
}

/// `iscase`
fn iscase(t: Option<&Tree>) -> bool {
    let mut t = match t {
        Some(t) if t.ty == SIMPLE => t,
        _ => return false,
    };
    loop {
        t = match t.child[0].as_deref() {
            Some(c) => c,
            None => return false,
        };
        if t.ty != ARGLIST {
            break;
        }
    }
    t.ty == WORD && !t.quoted && t.s.as_deref() == Some(b"case".as_slice())
}
