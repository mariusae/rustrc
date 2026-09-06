//! The interpreter's opcode routines (port of `exec.c`).
//!
//! Arguments on stack (...), arguments in line [...], code in line with
//! jump around {...}:
//!
//! ```text
//! Xappend(file)[fd]                 open file to append
//! Xassign(name, val)                assign val to name
//! Xasync{... Xexit}                 make thread for {}, no wait
//! Xbackq(split){... Xreturn}        make thread for {}, push stdout
//! Xbang                             complement condition
//! Xcase(pat, value){...}            exec code on match, leave (value) on stack
//! Xclose[i]                         close file descriptor
//! Xconc(left, right)                concatenate, push results
//! Xcount(name)                      push var count
//! Xdelfn(name)                      delete function definition
//! Xdol(name)                        get variable value
//! Xqdol(name)                       concatenate variable components
//! Xdup[i j]                         dup file descriptor
//! Xexit                             rc exits with status
//! Xfalse{...}                       execute {} if false
//! Xfn(name){... Xreturn}            define function
//! Xfor(var, list){... Xreturn}      for loop
//! Xjump[addr]                       goto
//! Xlocal(name, val)                 create local variable, assign value
//! Xmark                             mark stack
//! Xmatch(pat, str)                  match pattern, set status
//! Xpipe[i j]{... Xreturn}{... Xreturn}  construct a pipe between 2 new threads, wait for both
//! Xpipefd[type]{... Xreturn}        connect {} to pipe (input or output, depending on type), push /dev/fd/??
//! Xpopm(value)                      pop value from stack
//! Xrdwr(file)[fd]                   open file for reading and writing
//! Xread(file)[fd]                   open file to read
//! Xsimple(args)                     run command and wait
//! Xreturn                           kill thread
//! Xsubshell{... Xexit}              execute {} in a subshell and wait
//! Xtrue{...}                        execute {} if true
//! Xunlocal                          delete local variable
//! Xword[string]                     push string
//! Xwrite(file)[fd]                  open file to write
//! ```

use std::rc::Rc;

use crate::glob::{deglob, match_pat};
use crate::shell::*;
use crate::word::{list2str, Word, Words};

impl Shell {
    /// The interpreter loop of `main`.  Returns when there is no thread
    /// left to run (only possible for an embedded shell).
    pub(crate) fn run_loop(&mut self) {
        loop {
            if self.runq.is_none() {
                return;
            }
            if self.flag_set(b'r') {
                self.pfnc();
            }
            let op = {
                let q = self.q();
                q.pc += 1;
                match &q.code[q.pc - 1] {
                    Instr::F(op) => *op,
                    other => {
                        let msg = format!("bad instruction {:?} at pc {}", other, q.pc - 1);
                        self.panic(&msg)
                    }
                }
            };
            self.exec_op(op);
            if crate::trap::ntrap() != 0 {
                self.dotrap(true);
            }
        }
    }

    fn exec_op(&mut self, op: Op) {
        match op {
            Op::Append => self.xappend(),
            Op::Async => self.xasync(),
            Op::Backq => self.xbackq(),
            Op::Bang => self.xbang(),
            Op::Close => self.xclose(),
            Op::Conc => self.xconc(),
            Op::Count => self.xcount(),
            Op::Delfn => self.xdelfn(),
            Op::Dol => self.xdol(),
            Op::Qdol => self.xqdol(),
            Op::Dup => self.xdup(),
            Op::Exit => self.xexit(),
            Op::False => self.xfalse(),
            Op::Fn => self.xfn(),
            Op::For => self.xfor(),
            Op::Glob => self.globlist(),
            Op::Jump => self.xjump(),
            Op::Mark => self.pushlist(),
            Op::Match => self.xmatch(),
            Op::Pipe => self.xpipe(),
            Op::Read => self.xread(),
            Op::Rdwr => self.xrdwr(),
            Op::Rdfn => self.xrdfn(),
            Op::Return => self.xreturn(),
            Op::Subshell => self.xsubshell(),
            Op::True => self.xtrue(),
            Op::Word => self.xword(),
            Op::Write => self.xwrite(),
            Op::Pipefd => self.xpipefd(),
            Op::Case => self.xcase(),
            Op::Local => self.xlocal(),
            Op::Unlocal => self.xunlocal(),
            Op::Assign => self.xassign(),
            Op::Simple => self.xsimple(),
            Op::Popm => self.poplist(),
            Op::Rdcmds => self.xrdcmds(),
            Op::Wastrue => self.ifnot = false,
            Op::If => self.xif(),
            Op::Ifnot => self.xifnot(),
            Op::Pipewait => self.xpipewait(),
            Op::Delhere => self.xdelhere(),
            Op::Popredir => self.xpopredir(),
            Op::Sub => self.xsub(),
            Op::Eflag => self.xeflag(),
            Op::Settrue => self.setstatus(b""),
        }
    }

    /// `pfnc` (from `pfnc.c`): the `-r` trace line.
    fn pfnc(&mut self) {
        let q = self.qref();
        let pid = unsafe { libc::getpid() };
        let codeptr = Rc::as_ptr(&q.code) as usize;
        let pc = q.pc;
        let name = match &q.code[pc] {
            Instr::F(op) => op.name().to_string(),
            _ => String::new(),
        };
        let lists: Vec<Words> = q.argv.iter().rev().cloned().collect();
        self.err.pstr(b"pid ");
        self.err.pdec(pid as i64);
        self.err.pstr(b" cycle ");
        self.err.pptr(codeptr);
        self.err.pchr(b' ');
        self.err.pdec(pc as i64);
        self.err.pchr(b' ');
        self.err.pstr(name.as_bytes());
        for a in &lists {
            self.err.pstr(b" (");
            self.err.pval(a);
            self.err.pchr(b')');
        }
        self.err.pchr(b'\n');
        self.err.flush();
    }

    fn code_int(&self, off: usize) -> i32 {
        let q = self.qref();
        code_i(&q.code, q.pc + off)
    }

    fn open_redir(&mut self, name: &str, mode: i32) {
        let n = self.top_ref().len();
        if n != 1 {
            let msg = if n == 0 {
                format!("{} requires file{}", name, if name == ">>" { "" } else { "\n" })
            } else {
                format!("{} requires singleton{}", name, if name == ">>" { "" } else { "\n" })
            };
            self.xerror1(&msg);
            return;
        }
        let file = self.top_ref()[0].clone();
        let (f, errno) = match mode {
            0 => crate::unix::open_flags(&file, libc::O_RDONLY),
            1 => {
                let (f, e) = crate::unix::open_flags(&file, libc::O_WRONLY);
                if f < 0 {
                    let c = crate::unix::creat(&file);
                    if c < 0 {
                        (c, crate::unix::errno())
                    } else {
                        (c, e)
                    }
                } else {
                    (f, e)
                }
            }
            2 => {
                let c = crate::unix::creat(&file);
                (c, crate::unix::errno())
            }
            _ => crate::unix::open_flags(&file, libc::O_RDWR),
        };
        if f < 0 {
            self.err.pstr(&file);
            self.err.pstr(b": ");
            self.err.flush();
            self.xerror("can't open", errno);
            return;
        }
        if mode == 1 {
            unsafe {
                libc::lseek(f, 0, libc::SEEK_END);
            }
        }
        let to = self.code_int(0);
        self.pushredir(ROPEN, f, to);
        self.q().pc += 1;
        self.poplist();
    }

    fn xappend(&mut self) {
        self.open_redir(">>", 1);
    }

    fn xread(&mut self) {
        self.open_redir("<", 0);
    }

    fn xrdwr(&mut self) {
        self.open_redir("<>", 3);
    }

    fn xwrite(&mut self) {
        self.open_redir(">", 2);
    }

    fn xbang(&mut self) {
        let s: &[u8] = if self.truestatus() { b"false" } else { b"" };
        self.setstatus(s);
    }

    fn xclose(&mut self) {
        let from = self.code_int(0);
        self.pushredir(RCLOSE, from, 0);
        self.q().pc += 1;
    }

    fn xdup(&mut self) {
        let from = self.code_int(0);
        let to = self.code_int(1);
        self.pushredir(RDUP, from, to);
        self.q().pc += 2;
    }

    fn xeflag(&mut self) {
        if self.eflagok && !self.truestatus() {
            self.xexit();
        }
    }

    pub(crate) fn xexit(&mut self) {
        if unsafe { libc::getpid() } == self.mypid && !self.exit_beenhere {
            let trapreq = self.vlook(b"sigexit");
            let func = trapreq.borrow().func.clone();
            if let Some((code, pc)) = func {
                self.exit_beenhere = true;
                self.q().pc -= 1;
                let starval = self.vlook(b"*").borrow().val.clone();
                self.start(code, pc, None);
                self.push_local(b"*".to_vec(), starval);
                let q = self.q();
                q.redir = None;
                q.startredir = None;
                return;
            }
        }
        let st = self.getstatus();
        self.exit(&st);
    }

    fn xfalse(&mut self) {
        if self.truestatus() {
            let a = self.code_int(0);
            self.q().pc = a as usize;
        } else {
            self.q().pc += 1;
        }
    }

    fn xifnot(&mut self) {
        if self.ifnot {
            self.q().pc += 1;
        } else {
            let a = self.code_int(0);
            self.q().pc = a as usize;
        }
    }

    fn xjump(&mut self) {
        let a = self.code_int(0);
        self.q().pc = a as usize;
    }

    fn turfredir(&mut self) {
        loop {
            let q = self.qref();
            if redir_ptr_eq(&q.redir, &q.startredir) {
                break;
            }
            self.xpopredir();
        }
    }

    pub(crate) fn xpopredir(&mut self) {
        let rp = match self.q().redir.take() {
            Some(rp) => rp,
            None => self.panic("turfredir null!"),
        };
        self.q().redir = rp.next.clone();
        if rp.ty == ROPEN {
            unsafe {
                libc::close(rp.from);
            }
        }
    }

    /// `Xreturn`: kill the current thread and continue its parent.  In an
    /// embedded shell, returning from the last thread ends the evaluation
    /// instead of exiting the process.
    pub(crate) fn xreturn(&mut self) {
        self.turfredir();
        let p = self.runq.take().expect("rc: Xreturn without thread");
        self.runq = p.ret;
        drop(p.code);
        if self.runq.is_none() && self.exit_mode == ExitMode::Process {
            let st = self.getstatus();
            self.exit(&st);
        }
    }

    fn xtrue(&mut self) {
        if self.truestatus() {
            self.q().pc += 1;
        } else {
            let a = self.code_int(0);
            self.q().pc = a as usize;
        }
    }

    fn xif(&mut self) {
        self.ifnot = true;
        if self.truestatus() {
            self.q().pc += 1;
        } else {
            let a = self.code_int(0);
            self.q().pc = a as usize;
        }
    }

    fn xword(&mut self) {
        let w = {
            let q = self.qref();
            code_s(&q.code, q.pc).clone()
        };
        self.q().pc += 1;
        self.pushword(w);
    }

    fn xmatch(&mut self) {
        let subject = list2str(self.top_ref());
        self.setstatus(b"no match");
        let pats = self.next_list(1).clone();
        for p in &pats {
            if match_pat(&subject, p, 0) {
                self.setstatus(b"");
                break;
            }
        }
        self.poplist();
        self.poplist();
    }

    fn xcase(&mut self) {
        let s = list2str(self.next_list(1));
        let mut ok = false;
        for p in self.top_ref() {
            if match_pat(&s, p, 0) {
                ok = true;
                break;
            }
        }
        if ok {
            self.q().pc += 1;
        } else {
            let a = self.code_int(0);
            self.q().pc = a as usize;
        }
        self.poplist();
    }

    fn xconc(&mut self) {
        let lp = self.top_ref().clone();
        let rp = self.next_list(1).clone();
        let lc = lp.len();
        let rc = rp.len();
        let mut result: Vec<Word> = Vec::new();
        if lc != 0 || rc != 0 {
            if lc == 0 || rc == 0 {
                self.xerror1("null list in concatenation");
                return;
            }
            if lc != 1 && rc != 1 && lc != rc {
                self.xerror1("mismatched list lengths in concatenation");
                return;
            }
            let n = lc.max(rc);
            for i in 0..n {
                let l = &lp[if lc == 1 { 0 } else { i }];
                let r = &rp[if rc == 1 { 0 } else { i }];
                let mut w = l.clone();
                w.extend_from_slice(r);
                result.push(w);
            }
        }
        self.poplist();
        self.poplist();
        let vp = self.top();
        for w in result.into_iter().rev() {
            vp.push_front(w);
        }
    }

    fn xassign(&mut self) {
        if self.top_ref().len() != 1 {
            self.xerror1("variable name not singleton!");
            return;
        }
        let mut name = self.top_ref()[0].clone();
        deglob(&mut name);
        let v = self.vlook(&name);
        self.poplist();
        self.globlist();
        let val = std::mem::take(self.top());
        let cf = {
            let mut vb = v.borrow_mut();
            vb.val = val;
            vb.changed = true;
            vb.changefn
        };
        self.changefn(cf, &v);
        self.poplist();
    }

    /// Parse the leading decimal number of a variable name as `Xdol` does;
    /// returns `(n, all_digits)`.
    fn varnum(s: &[u8]) -> (i32, bool) {
        let mut n: i32 = 0;
        let mut i = 0;
        while i < s.len() && s[i].is_ascii_digit() {
            n = n.wrapping_mul(10).wrapping_add((s[i] - b'0') as i32);
            i += 1;
        }
        (n, i == s.len())
    }

    fn xdol(&mut self) {
        if self.top_ref().len() != 1 {
            self.xerror1("variable name not singleton!");
            return;
        }
        let mut s = self.top_ref()[0].clone();
        deglob(&mut s);
        let (n, all) = Shell::varnum(&s);
        let vals: Vec<Word> = if n == 0 || !all {
            self.vlook(&s).borrow().val.iter().cloned().collect()
        } else {
            let star = self.vlook(b"*");
            let star = star.borrow();
            if 1 <= n && (n as usize) <= star.val.len() {
                vec![star.val[n as usize - 1].clone()]
            } else {
                Vec::new()
            }
        };
        self.poplist();
        let a = self.top();
        for w in vals.into_iter().rev() {
            a.push_front(w);
        }
    }

    fn xqdol(&mut self) {
        if self.top_ref().len() != 1 {
            self.xerror1("variable name not singleton!");
            return;
        }
        let mut s = self.top_ref()[0].clone();
        deglob(&mut s);
        let a = self.vlook(&s).borrow().val.clone();
        self.poplist();
        if a.is_empty() {
            self.pushword(Vec::new());
            return;
        }
        self.pushword(list2str(&a));
    }

    /// `subwords`
    fn subwords(val: &Words, sub: &Words) -> Vec<Word> {
        let len = val.len() as i32;
        let mut out = Vec::new();
        for s in sub {
            let mut s = s.clone();
            deglob(&mut s);
            let mut i = 0;
            let mut n: i32 = 0;
            let mut m: i32 = 0;
            while i < s.len() && s[i].is_ascii_digit() {
                n = n.wrapping_mul(10).wrapping_add((s[i] - b'0') as i32);
                i += 1;
            }
            if i < s.len() && s[i] == b'-' {
                i += 1;
                if i >= s.len() {
                    m = len - n;
                } else {
                    while i < s.len() && s[i].is_ascii_digit() {
                        m = m.wrapping_mul(10).wrapping_add((s[i] - b'0') as i32);
                        i += 1;
                    }
                    m -= n;
                }
            }
            if n < 1 || n > len || m < 0 {
                continue;
            }
            if n + m > len {
                m = len - n;
            }
            for k in 0..=m {
                out.push(val[(n - 1 + k) as usize].clone());
            }
        }
        out
    }

    fn xsub(&mut self) {
        if self.next_list(1).len() != 1 {
            self.xerror1("variable name not singleton!");
            return;
        }
        let mut s = self.next_list(1)[0].clone();
        deglob(&mut s);
        let v = self.vlook(&s).borrow().val.clone();
        let sub = self.top_ref().clone();
        let words = Shell::subwords(&v, &sub);
        self.poplist();
        self.poplist();
        let a = self.top();
        for w in words.into_iter().rev() {
            a.push_front(w);
        }
    }

    fn xcount(&mut self) {
        if self.top_ref().len() != 1 {
            self.xerror1("variable name not singleton!");
            return;
        }
        let mut s = self.top_ref()[0].clone();
        deglob(&mut s);
        let (n, all) = Shell::varnum(&s);
        let num: i64 = if n == 0 || !all {
            self.vlook(&s).borrow().val.len() as i64
        } else {
            let star = self.vlook(b"*");
            let len = star.borrow().val.len();
            if 1 <= n && (n as usize) <= len {
                1
            } else {
                0
            }
        };
        self.poplist();
        self.pushword(num.to_string().into_bytes());
    }

    fn xlocal(&mut self) {
        if self.top_ref().len() != 1 {
            self.xerror1("variable name must be singleton\n");
            return;
        }
        let mut name = self.top_ref()[0].clone();
        deglob(&mut name);
        let val = self.next_list(1).clone();
        self.push_local(name, val);
        self.poplist();
        self.poplist();
    }

    fn xunlocal(&mut self) {
        let v = match self.q().local.take() {
            Some(v) => v,
            None => self.panic("Xunlocal: no locals!"),
        };
        self.q().local = v.next.clone();
        let name = v.var.borrow().name.clone();
        let hid = self.vlook(&name);
        hid.borrow_mut().changed = true;
    }

    fn xfn(&mut self) {
        let end = self.code_int(0) as usize;
        let (code, pc) = {
            let q = self.qref();
            (q.code.clone(), q.pc + 2)
        };
        let names: Vec<Word> = self.top_ref().iter().cloned().collect();
        for a in names {
            let v = self.gvlook(&a);
            let mut vb = v.borrow_mut();
            vb.func = Some((code.clone(), pc));
            vb.fnchanged = true;
        }
        self.q().pc = end;
        self.poplist();
    }

    fn xdelfn(&mut self) {
        let names: Vec<Word> = self.top_ref().iter().cloned().collect();
        for a in names {
            let v = self.gvlook(&a);
            let mut vb = v.borrow_mut();
            vb.func = None;
            vb.fnchanged = true;
        }
        self.poplist();
    }

    /// `concstatus`
    fn concstatus(s: &[u8], t: &[u8]) -> Word {
        let mut v: Vec<u8> = s.iter().take(NSTATUS).cloned().collect();
        let n = s.len();
        if n < NSTATUS {
            v.push(b'|');
            v.extend(t.iter().take(NSTATUS - n - 1));
        }
        v.truncate(NSTATUS);
        v
    }

    fn xpipewait(&mut self) {
        if self.qref().pid == -1 {
            let st = Shell::concstatus(&self.qref().status.clone(), &self.getstatus());
            self.setstatus(&st);
        } else {
            let mut status = self.getstatus();
            status.truncate(NSTATUS);
            let pid = self.qref().pid;
            self.waitfor(pid, true);
            self.q().pid = -1;
            let st = Shell::concstatus(&self.getstatus(), &status);
            self.setstatus(&st);
        }
    }

    fn xrdcmds(&mut self) {
        self.err.flush();
        self.nerror = 0;
        if self.flag_set(b's') && !self.truestatus() {
            let v = self.vlook(b"status").borrow().val.clone();
            self.err.pstr(b"status=");
            self.err.pval(&v);
            self.err.pchr(b'\n');
            self.err.flush();
        }
        if self.qref().iflag {
            let prompt = self.vlook(b"prompt").borrow().val.front().cloned();
            self.promptstr = match prompt {
                Some(p) => p,
                None => b"% ".to_vec(),
            };
        }
        crate::unix::noerror();
        if self.parse() {
            let (iflag, eof) = {
                let q = self.qref();
                (q.iflag, q.eof)
            };
            if !iflag || (eof != 0 && !crate::unix::eintr()) {
                if let Some(f) = self.q().cmdfd.take() {
                    if let Ok(f) = Rc::try_unwrap(f) {
                        f.into_inner().close();
                    }
                }
                self.q().cmdfile = None;
                self.xreturn(); // should this be omitted?
            } else {
                if crate::unix::eintr() {
                    self.err.pchr(b'\n');
                    self.q().eof = 0;
                }
                self.q().pc -= 1; // go back for next command
            }
        } else {
            // avoid double-interrupts during blocked writes
            crate::trap::clear_traps_without_handlers(self);
            self.q().pc -= 1; // re-execute Xrdcmds after codebuf runs
            let code = self.take_codebuf();
            let local = self.qref().local.clone();
            self.start(code, 1, local);
        }
    }

    fn error_prefix(&mut self) {
        if self.argv0 == b"rc" || self.argv0 == b"/bin/rc" {
            self.err.pstr(b"rc: ");
        } else {
            self.err.pstr(b"rc (");
            let a = self.argv0.clone();
            self.err.pstr(&a);
            self.err.pstr(b"): ");
        }
    }

    /// `Xerror`: report a system error and abandon the non-interactive
    /// threads.
    pub(crate) fn xerror(&mut self, s: &str, errno: i32) {
        self.error_prefix();
        self.err.pstr(s.as_bytes());
        self.err.pstr(b": ");
        self.err.pstr(&crate::unix::errstr(errno));
        self.err.pchr(b'\n');
        self.err.flush();
        self.setstatus(b"error");
        self.unwind_to_interactive();
    }

    /// `Xerror1`
    pub(crate) fn xerror1(&mut self, s: &str) {
        self.error_prefix();
        self.err.pstr(s.as_bytes());
        self.err.pchr(b'\n');
        self.err.flush();
        self.setstatus(b"error");
        self.unwind_to_interactive();
    }

    fn unwind_to_interactive(&mut self) {
        while let Some(q) = &self.runq {
            if q.iflag {
                break;
            }
            self.xreturn();
        }
    }

    pub(crate) fn setstatus(&mut self, s: &[u8]) {
        let mut w = Words::new();
        w.push_back(s.to_vec());
        self.setvar(b"status", w);
    }

    pub(crate) fn getstatus(&mut self) -> Word {
        self.vlook(b"status").borrow().val.front().cloned().unwrap_or_default()
    }

    pub(crate) fn truestatus(&mut self) -> bool {
        self.getstatus().iter().all(|&c| c == b'|' || c == b'0')
    }

    fn xdelhere(&mut self) {
        let name = {
            let q = self.qref();
            code_s(&q.code, q.pc).clone()
        };
        self.q().pc += 1;
        crate::unix::unlink(&name);
    }

    fn xfor(&mut self) {
        if self.top_ref().is_empty() {
            self.poplist();
            let a = self.code_int(0);
            self.q().pc = a as usize;
        } else {
            let w = self.top().pop_front().unwrap();
            let local = self.qref().local.clone().expect("rc: Xfor without local");
            let mut v = local.var.borrow_mut();
            v.val.clear();
            v.val.push_back(w);
            v.changed = true;
            drop(v);
            self.q().pc += 1;
        }
    }
}
