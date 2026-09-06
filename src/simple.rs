//! Simple commands and the built-in commands (port of `simple.c`, with the
//! `ulimit`/`umask`/`rfork` builtins from `unixcrap.c`/`plan9ish.c`).
//!
//! Maybe `simple' is a misnomer.

use std::rc::Rc;

use crate::io::Io;
use crate::shell::*;
use crate::var::VarRef;
use crate::word::{atoi, Word, Words};

/// The built-in commands, in the order of the C `Builtin` table.
pub const BUILTINS: [&str; 13] =
    ["cd", "whatis", "eval", "exec", "exit", "shift", "wait", ".", "finit", "flag", "ulimit", "umask", "rfork"];

impl Shell {
    /// `exitnext`: are we just going to exit after this command?
    fn exitnext(&self) -> bool {
        let q = self.qref();
        let mut pc = q.pc;
        while matches!(q.code.get(pc), Some(Instr::F(Op::Popredir))) {
            pc += 1;
        }
        matches!(q.code.get(pc), Some(Instr::F(Op::Exit)))
    }

    pub(crate) fn xsimple(&mut self) {
        self.globlist();
        if self.top_ref().is_empty() {
            self.xerror1("empty argument list");
            return;
        }
        if self.flag_set(b'x') {
            let words = self.top_ref().clone();
            self.err.pval(&words); // wrong, should do redirs
            self.err.pchr(b'\n');
            self.err.flush();
        }
        let first = self.top_ref()[0].clone();
        let v = self.gvlook(&first);
        if v.borrow().func.is_some() {
            self.execfunc(&v);
            return;
        }
        let mut name = first;
        if name == b"builtin" {
            if self.top_ref().len() == 1 {
                self.err.pstr(b"builtin: empty argument list\n");
                self.err.flush();
                self.setstatus(b"empty arg list");
                self.poplist();
                return;
            }
            self.popword();
            name = self.top_ref()[0].clone();
        }
        if let Some(b) = BUILTINS.iter().find(|b| b.as_bytes() == name.as_slice()) {
            match *b {
                "cd" => self.execcd(),
                "whatis" => self.execwhatis(),
                "eval" => self.execeval(),
                "exec" => self.execexec(),
                "exit" => self.execexit(),
                "shift" => self.execshift(),
                "wait" => self.execwait(),
                "." => self.execdot(),
                "finit" => self.execfinit(),
                "flag" => self.execflag(),
                "ulimit" => self.execulimit(),
                "umask" => self.execumask(),
                "rfork" => self.execrfork(),
                _ => unreachable!(),
            }
            return;
        }
        if self.exitnext() {
            // fork and wait is redundant
            self.pushword(b"exec".to_vec());
            self.execexec();
            self.xexit();
        } else {
            self.err.flush();
            let pid = self.execforkexec();
            if pid < 0 {
                let e = crate::unix::errno();
                self.xerror("try again", e);
                return;
            }
            // interrupts don't get us out
            self.poplist();
            while self.waitfor(pid, true) < 0 {}
        }
    }

    /// `doredir`: apply a redirection list (oldest first).
    pub(crate) fn doredir(rp: &RedirList) {
        let mut v = Vec::new();
        let mut r = rp.clone();
        while let Some(x) = r {
            v.push(x.clone());
            r = x.next.clone();
        }
        for rp in v.iter().rev() {
            unsafe {
                match rp.ty {
                    ROPEN => {
                        if rp.from != rp.to {
                            libc::dup2(rp.from, rp.to);
                            libc::close(rp.from);
                        }
                    }
                    RDUP => {
                        libc::dup2(rp.from, rp.to);
                    }
                    RCLOSE => {
                        libc::close(rp.from);
                    }
                    _ => {}
                }
            }
        }
    }

    /// `searchpath`: the directories to search for `w`.
    pub(crate) fn searchpath(&mut self, w: &[u8]) -> Words {
        let mut nullpath = Words::new();
        nullpath.push_back(Vec::new());
        if w.starts_with(b"/") || w.starts_with(b"./") || w.starts_with(b"../") {
            return nullpath;
        }
        let path = self.vlook(b"path").borrow().val.clone();
        if path.is_empty() {
            nullpath
        } else {
            path
        }
    }

    /// `execexec`: the `exec` builtin (and the tail of a simple command
    /// that is about to exit anyway).
    pub(crate) fn execexec(&mut self) {
        self.popword(); // "exec"
        if self.top_ref().is_empty() {
            self.xerror1("empty argument list");
            return;
        }
        let redir = self.qref().redir.clone();
        Shell::doredir(&redir);
        let args = self.top_ref().clone();
        let path = self.searchpath(&args[0]);
        self.execute(&args, &path);
        self.poplist();
    }

    /// `execfunc`: call a user-defined function.
    fn execfunc(&mut self, func: &VarRef) {
        self.popword();
        let starval = std::mem::take(self.top());
        self.poplist();
        let (code, pc) = func.borrow().func.clone().unwrap();
        let local = self.qref().local.clone();
        self.start(code, pc, local);
        self.push_local(b"*".to_vec(), starval);
    }

    /// `dochdir`
    fn dochdir(&mut self, word: &[u8]) -> i32 {
        if crate::unix::chdir(word) < 0 {
            return -1;
        }
        // report to /dev/wdir if it exists and we're interactive
        if self.flag_set(b'i') {
            if self.wdirfd == -2 {
                // try only once
                self.wdirfd = crate::unix::open_flags(b"/dev/wdir", libc::O_WRONLY | libc::O_CLOEXEC).0;
            }
            if self.wdirfd >= 0 {
                crate::unix::write_all(self.wdirfd, word);
            }
        }
        1
    }

    fn execcd(&mut self) {
        let a = self.top_ref().clone();
        self.setstatus(b"can't cd");
        let mut cdpath = self.vlook(b"cdpath").borrow().val.clone();
        match a.len() {
            2 => {
                if a[1].first() == Some(&b'/') || cdpath.is_empty() {
                    cdpath = Words::new();
                    cdpath.push_back(Vec::new());
                }
                let mut found = false;
                for p in &cdpath {
                    let mut dir = p.clone();
                    if !dir.is_empty() {
                        dir.push(b'/');
                    }
                    dir.extend_from_slice(&a[1]);
                    if self.dochdir(&dir) >= 0 {
                        if !p.is_empty() && p != b"." {
                            self.err.pstr(&dir);
                            self.err.pchr(b'\n');
                            self.err.flush();
                        }
                        self.setstatus(b"");
                        found = true;
                        break;
                    }
                }
                if !found {
                    let e = crate::unix::errno();
                    self.err.pstr(b"Can't cd ");
                    self.err.pstr(&a[1]);
                    self.err.pstr(b": ");
                    self.err.pstr(&crate::unix::errstr(e));
                    self.err.pchr(b'\n');
                    self.err.flush();
                }
            }
            1 => {
                let home = self.vlook(b"home").borrow().val.clone();
                if let Some(h) = home.front() {
                    if self.dochdir(h) >= 0 {
                        self.setstatus(b"");
                    } else {
                        let e = crate::unix::errno();
                        self.err.pstr(b"Can't cd ");
                        self.err.pstr(h);
                        self.err.pstr(b": ");
                        self.err.pstr(&crate::unix::errstr(e));
                        self.err.pchr(b'\n');
                        self.err.flush();
                    }
                } else {
                    self.err.pstr(b"Can't cd -- $home empty\n");
                    self.err.flush();
                }
            }
            _ => {
                self.err.pstr(b"Usage: cd [directory]\n");
                self.err.flush();
            }
        }
        self.poplist();
    }

    fn execexit(&mut self) {
        let a = self.top_ref().clone();
        match a.len() {
            1 => {}
            2 => self.setstatus(&a[1]),
            _ => {
                self.err.pstr(b"Usage: exit [status]\nExiting anyway\n");
                self.err.flush();
                self.setstatus(&a[1]);
            }
        }
        self.xexit();
    }

    fn execshift(&mut self) {
        let a = self.top_ref().clone();
        let mut n = match a.len() {
            2 => atoi(&a[1]),
            1 => 1,
            _ => {
                self.err.pstr(b"Usage: shift [n]\n");
                self.err.flush();
                self.setstatus(b"shift usage");
                self.poplist();
                return;
            }
        };
        let star = self.vlook(b"*");
        {
            let mut sb = star.borrow_mut();
            while n != 0 && !sb.val.is_empty() {
                sb.val.pop_front();
                sb.changed = true;
                n -= 1;
            }
        }
        self.setstatus(b"");
        self.poplist();
    }

    /// `mapfd`: the file descriptor that `fd` refers to after the current
    /// redirections.
    pub(crate) fn mapfd(&self, fd: i32) -> i32 {
        let mut fd = fd;
        let mut r = self.qref().redir.clone();
        while let Some(rp) = r {
            match rp.ty {
                RCLOSE => {
                    if rp.from == fd {
                        fd = -1;
                    }
                }
                RDUP | ROPEN => {
                    if rp.to == fd {
                        fd = rp.from;
                    }
                }
                _ => {}
            }
            r = rp.next.clone();
        }
        fd
    }

    /// `execcmds`: start a thread reading commands from `f`.
    pub(crate) fn execcmds(&mut self, f: Io) {
        let code = self.rdcmds_code.clone();
        let local = self.qref().local.clone();
        self.start(code, 1, local);
        let q = self.q();
        q.cmdfd = Some(Rc::new(std::cell::RefCell::new(f)));
        q.iflast = false;
    }

    fn execeval(&mut self) {
        if self.top_ref().len() <= 1 {
            self.xerror1("Usage: eval cmd ...");
            return;
        }
        self.eflagok = true;
        let mut cmdline = Vec::new();
        for w in self.top_ref().iter().skip(1) {
            cmdline.extend_from_slice(w);
            cmdline.push(b' ');
        }
        *cmdline.last_mut().unwrap() = b'\n';
        self.poplist();
        self.execcmds(Io::opencore(&cmdline));
    }

    fn execdot(&mut self) {
        if self.dot_first {
            self.dot_first = false;
        } else {
            self.eflagok = true;
        }
        self.popword();
        let mut iflag = false;
        if self.top_ref().front().map(|w| w.as_slice()) == Some(b"-i") {
            iflag = true;
            self.popword();
        }
        // get input file
        if self.top_ref().is_empty() {
            self.xerror1("Usage: . [-i] file [arg ...]");
            return;
        }
        let zero = self.top_ref()[0].clone();
        self.popword();
        let mut fd = -1;
        let mut errno = 0;
        for p in self.searchpath(&zero) {
            let mut file = p.clone();
            if !file.is_empty() {
                file.push(b'/');
            }
            file.extend_from_slice(&zero);
            let (f, e) = crate::unix::open_flags(&file, libc::O_RDONLY);
            fd = f;
            errno = e;
            if fd >= 0 {
                break;
            }
            if file == b"/dev/stdin" {
                // for sun & ucb
                fd = unsafe { libc::dup(0) };
                if fd >= 0 {
                    break;
                }
                errno = crate::unix::errno();
            }
        }
        if fd < 0 {
            self.err.pstr(&zero);
            self.err.pstr(b": ");
            self.err.flush();
            self.setstatus(b"can't open");
            self.xerror(".: can't open", errno);
            return;
        }
        if !self.rcmain_path.is_empty() && zero == self.rcmain_path {
            // the embedded rcmain was written to a temporary file
            crate::unix::unlink(&zero);
            self.rcmain_path.clear();
        }
        // set up for a new command loop
        let args = std::mem::take(self.top());
        self.poplist(); // free caller's copy of $*
        let code = self.dotcmds_code.clone();
        self.start(code, 1, None);
        self.pushredir(RCLOSE, fd, 0);
        {
            let q = self.q();
            q.cmdfile = Some(zero.clone());
            q.cmdfd = Some(Rc::new(std::cell::RefCell::new(Io::openfd(fd))));
            q.iflag = iflag;
            q.iflast = false;
        }
        // push $* value
        self.pushlist();
        *self.top() = args;
        // push $0 value
        self.pushlist();
        self.pushword(zero);
        self.ndot += 1;
    }

    fn execflag(&mut self) {
        let a = self.top_ref().clone();
        match a.len() {
            2 => {
                let c = a[1].first().copied().unwrap_or(0);
                let set = self.flag_set(c);
                self.setstatus(if set { b"" } else { b"flag not set" });
            }
            3 => {
                let letter = &a[1];
                let val = &a[2];
                let mut ok = false;
                if letter.len() == 1 && (letter[0] as usize) < NFLAG {
                    if val == b"+" {
                        self.flag[letter[0] as usize] = Some(Vec::new());
                        ok = true;
                    } else if val == b"-" {
                        self.flag[letter[0] as usize] = None;
                        ok = true;
                    }
                }
                if !ok {
                    self.xerror1("Usage: flag [letter] [+-]");
                    return;
                }
            }
            _ => {
                self.xerror1("Usage: flag [letter] [+-]");
                return;
            }
        }
        self.poplist();
    }

    /// `execwhatis` -- mildly wrong -- should fork before writing
    fn execwhatis(&mut self) {
        let a: Vec<Word> = self.top_ref().iter().skip(1).cloned().collect();
        if a.is_empty() {
            self.xerror1("Usage: whatis name ...");
            return;
        }
        self.setstatus(b"");
        let mut out = Io::openfd(self.mapfd(1));
        for name in &a {
            let v = self.vlook(name);
            let val = v.borrow().val.clone();
            let found = if !val.is_empty() {
                out.pstr(name);
                out.pchr(b'=');
                if val.len() == 1 {
                    out.pwrd(&val[0]);
                    out.pchr(b'\n');
                } else {
                    let mut sep = b'(';
                    for b in &val {
                        out.pchr(sep);
                        out.pwrd(b);
                        sep = b' ';
                    }
                    out.pstr(b")\n");
                }
                out.flush();
                true
            } else {
                false
            };
            let v = self.gvlook(name);
            let func = v.borrow().func.clone();
            if let Some((code, pc)) = func {
                out.pstr(b"fn ");
                out.pstr(&v.borrow().name);
                out.pchr(b' ');
                out.pstr(code_s(&code, pc - 1));
                out.pchr(b'\n');
                out.flush();
            } else if BUILTINS.iter().any(|b| b.as_bytes() == name.as_slice()) {
                out.pstr(b"builtin ");
                out.pstr(name);
                out.pchr(b'\n');
                out.flush();
            } else {
                let mut hit = false;
                for p in self.searchpath(name) {
                    let mut file = p.clone();
                    if !file.is_empty() {
                        file.push(b'/');
                    }
                    file.extend_from_slice(name);
                    if crate::unix::executable(&file) {
                        out.pstr(&file);
                        out.pchr(b'\n');
                        out.flush();
                        hit = true;
                        break;
                    }
                }
                if !hit && !found {
                    self.err.pstr(name);
                    self.err.pstr(b": not found\n");
                    self.err.flush();
                    self.setstatus(b"not found");
                }
            }
        }
        self.poplist();
        self.err.flush();
    }

    fn execwait(&mut self) {
        let a = self.top_ref().clone();
        match a.len() {
            2 => {
                let pid = atoi(&a[1]);
                self.waitfor(pid, false);
            }
            1 => {
                self.waitfor(-1, false);
            }
            _ => {
                self.xerror1("Usage: wait [pid]");
                return;
            }
        }
        self.poplist();
    }

    /// `execfinit`: read function definitions from the environment.
    fn execfinit(&mut self) {
        self.poplist();
        self.envp = 0;
        let code = self.rdfns_code.clone();
        let local = self.qref().local.clone();
        self.start(code, 1, local);
    }

    /// `execrfork` (Plan 9 semantics as emulated by lib9's `rfork`).
    fn execrfork(&mut self) {
        const RFNAMEG: i32 = 1 << 0;
        const RFENVG: i32 = 1 << 1;
        const RFFDG: i32 = 1 << 2;
        const RFNOTEG: i32 = 1 << 3;
        const RFCNAMEG: i32 = 1 << 10;
        const RFCENVG: i32 = 1 << 11;
        const RFCFDG: i32 = 1 << 12;
        let a = self.top_ref().clone();
        let mut arg = 0;
        let mut usage = false;
        match a.len() {
            1 => arg = RFENVG | RFNOTEG | RFNAMEG,
            2 => {
                for &c in &a[1] {
                    match c {
                        b'n' => arg |= RFNAMEG,
                        b'N' => arg |= RFCNAMEG,
                        b'e' => { /* arg |= RFENVG; */ }
                        b'E' => arg |= RFCENVG,
                        b's' => arg |= RFNOTEG,
                        b'f' => arg |= RFFDG,
                        b'F' => arg |= RFCFDG,
                        _ => {
                            usage = true;
                            break;
                        }
                    }
                }
            }
            _ => usage = true,
        }
        if usage {
            self.err.pstr(b"Usage: ");
            self.err.pstr(&a[0]);
            self.err.pstr(b" [nNeEsfF]\n");
            self.err.flush();
            self.setstatus(b"rfork usage");
            self.poplist();
            return;
        }
        // lib9's p9rfork without RFPROC: RFNAMEG is ignored, RFNOTEG makes a
        // new process group, anything else is an error.
        let mut flags = arg;
        let mut ok = true;
        if flags & RFNAMEG != 0 {
            flags &= !RFNAMEG;
        }
        if flags & RFNOTEG != 0 {
            unsafe {
                libc::setpgid(0, libc::getpid());
            }
            flags &= !RFNOTEG;
        }
        if flags != 0 {
            ok = false;
        }
        if !ok {
            self.err.pstr(b"rc: ");
            self.err.pstr(&a[0]);
            self.err.pstr(b" failed\n");
            self.err.flush();
            self.setstatus(b"rfork failed");
        } else {
            self.setstatus(b"");
        }
        self.poplist();
    }

    /// `execulimit`
    fn execulimit(&mut self) {
        const EARGS: &[u8] = b"cdflmnstuv";
        fn rlx(i: usize) -> libc::c_int {
            let v: [libc::c_int; 10] = [
                libc::RLIMIT_CORE as libc::c_int,
                libc::RLIMIT_DATA as libc::c_int,
                libc::RLIMIT_FSIZE as libc::c_int,
                libc::RLIMIT_MEMLOCK as libc::c_int,
                libc::RLIMIT_RSS as libc::c_int,
                libc::RLIMIT_NOFILE as libc::c_int,
                libc::RLIMIT_STACK as libc::c_int,
                libc::RLIMIT_CPU as libc::c_int,
                libc::RLIMIT_NPROC as libc::c_int,
                libc::RLIMIT_RSS as libc::c_int,
            ];
            v[i]
        }
        const NOTSET: i64 = -4;
        const UNLIMITED: i64 = -3;
        const HARD: i64 = -2;
        const SOFT: i64 = -1;

        self.setstatus(b"");
        let argv: Vec<Word> = self.top_ref().iter().cloned().collect();
        let eusage = |sh: &mut Shell| {
            let fd = sh.mapfd(2);
            let msg = format!("usage: ulimit [-SHa{} [limit]]\n", std::str::from_utf8(EARGS).unwrap());
            crate::unix::write_all(fd, msg.as_bytes());
        };
        let mut flag = [false; 256];
        // ARGBEGIN over argv[1..]
        let mut i = 1;
        while i < argv.len() && argv[i].first() == Some(&b'-') && argv[i].len() > 1 {
            if argv[i] == b"--" {
                i += 1;
                break;
            }
            for &c in &argv[i][1..] {
                if c != b'S' && c != b'H' && c != b'a' && !EARGS.contains(&c) {
                    eusage(self);
                    return; // (the C code returns without popping the list)
                }
                flag[c as usize] = true;
            }
            i += 1;
        }
        let rest = &argv[i..];
        let argc = rest.len();
        let fd = self.mapfd(1);
        let mut sethard = true;
        let mut setsoft = true;
        if flag[b'S' as usize] && flag[b'H' as usize] {
        } else if flag[b'S' as usize] {
            sethard = false;
        } else if flag[b'H' as usize] {
            setsoft = false;
        }
        'out: {
            if argc > 1 {
                eusage(self);
                break 'out;
            }
            let mut limit = NOTSET;
            if argc > 0 {
                let w = &rest[0];
                if w == b"unlimited" {
                    limit = UNLIMITED;
                } else if w == b"hard" {
                    limit = HARD;
                } else if w == b"soft" {
                    limit = SOFT;
                } else {
                    match crate::unix::strtol(w, 0) {
                        Some(n) if n >= 0 => limit = n,
                        _ => {
                            eusage(self);
                            break 'out;
                        }
                    }
                }
            }
            let print = |fd: i32, c: u8, n: libc::rlim_t| {
                let s = if n == libc::RLIM_INFINITY {
                    format!("ulimit -{} unlimited\n", c as char)
                } else {
                    format!("ulimit -{} {}\n", c as char, n as u64)
                };
                crate::unix::write_all(fd, s.as_bytes());
            };
            if flag[b'a' as usize] {
                for (k, &c) in EARGS.iter().enumerate() {
                    let mut rl: libc::rlimit = unsafe { std::mem::zeroed() };
                    unsafe {
                        libc::getrlimit(rlx(k) as _, &mut rl);
                    }
                    let n = if flag[b'H' as usize] { rl.rlim_max } else { rl.rlim_cur };
                    print(fd, c, n);
                }
                break 'out;
            }
            for (k, &c) in EARGS.iter().enumerate() {
                if !flag[c as usize] {
                    continue;
                }
                let mut rl: libc::rlimit = unsafe { std::mem::zeroed() };
                unsafe {
                    libc::getrlimit(rlx(k) as _, &mut rl);
                }
                let n: libc::rlim_t;
                match limit {
                    NOTSET => {
                        let n = if flag[b'H' as usize] { rl.rlim_max } else { rl.rlim_cur };
                        print(fd, c, n);
                        continue;
                    }
                    HARD => n = rl.rlim_max,
                    SOFT => n = rl.rlim_cur,
                    UNLIMITED => n = libc::RLIM_INFINITY,
                    _ => n = limit as libc::rlim_t,
                }
                if setsoft {
                    rl.rlim_cur = n;
                }
                if sethard {
                    rl.rlim_max = n;
                }
                if unsafe { libc::setrlimit(rlx(k) as _, &rl) } < 0 {
                    let e = crate::unix::errno();
                    let fd2 = self.mapfd(2);
                    let mut msg = b"setrlimit: ".to_vec();
                    msg.extend_from_slice(&crate::unix::errstr(e));
                    msg.push(b'\n');
                    crate::unix::write_all(fd2, &msg);
                }
            }
        }
        self.poplist();
        self.err.flush();
    }

    /// `execumask`
    fn execumask(&mut self) {
        self.setstatus(b"");
        let argv: Vec<Word> = self.top_ref().iter().cloned().collect();
        let usage = |sh: &mut Shell| {
            let fd = sh.mapfd(2);
            crate::unix::write_all(fd, b"usage: umask [mode]\n");
        };
        'out: {
            let mut i = 1;
            let mut bad = false;
            while i < argv.len() && argv[i].first() == Some(&b'-') && argv[i].len() > 1 {
                if argv[i] == b"--" {
                    i += 1;
                    break;
                }
                bad = true;
                break;
            }
            if bad {
                usage(self);
                break 'out;
            }
            let rest = &argv[i..];
            if rest.len() > 1 {
                usage(self);
                break 'out;
            }
            if rest.len() == 1 {
                match crate::unix::strtol(&rest[0], 8) {
                    Some(n) => unsafe {
                        libc::umask(n as libc::mode_t);
                    },
                    None => usage(self),
                }
                break 'out;
            }
            let n = unsafe {
                let n = libc::umask(0);
                libc::umask(n);
                n
            };
            let fd = self.mapfd(1);
            crate::unix::write_all(fd, format!("umask {:03o}\n", n).as_bytes());
        }
        self.poplist();
        self.err.flush();
    }
}
