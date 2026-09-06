//! The Unix side of the interpreter (port of `plan9ish.c`, `havefork.c`
//! and `unixcrap.c`, plus the tiny pieces of lib9 that rc relies on).

use std::ffi::{CStr, CString};
use std::rc::Rc;
use std::sync::atomic::Ordering;

use crate::io::Io;
use crate::shell::*;
use crate::trap::{INTERRUPTED, KIDPID, NTRAP};
use crate::word::{Word, Words};

/// The separator between list elements in environment values.
pub const SEP: u8 = 1;
pub const FDPREFIX: &[u8] = b"/dev/fd/";

pub fn errno() -> i32 {
    std::io::Error::last_os_error().raw_os_error().unwrap_or(0)
}

/// lib9's `rerrstr`: the text for an errno value.
pub fn errstr(e: i32) -> Word {
    if e == libc::EINTR {
        return b"interrupted".to_vec();
    }
    unsafe {
        let p = libc::strerror(e);
        if p.is_null() {
            return format!("errno {}", e).into_bytes();
        }
        CStr::from_ptr(p).to_bytes().to_vec()
    }
}

fn cstr(s: &[u8]) -> CString {
    // C strings end at the first NUL.
    let end = s.iter().position(|&c| c == 0).unwrap_or(s.len());
    CString::new(&s[..end]).unwrap()
}

/// A single `write(2)` (rc's `Write`); returns the count or -1.
pub fn write_all(fd: i32, buf: &[u8]) -> isize {
    unsafe { libc::write(fd, buf.as_ptr() as *const libc::c_void, buf.len()) }
}

/// `open(name, mode)`; returns `(fd, errno)`.
pub fn open_flags(name: &[u8], flags: i32) -> (i32, i32) {
    let c = cstr(name);
    let fd = unsafe { libc::open(c.as_ptr(), flags) };
    (fd, if fd < 0 { errno() } else { 0 })
}

/// `Creat`: `create(file, OWRITE, 0666)`.
pub fn creat(name: &[u8]) -> i32 {
    let c = cstr(name);
    unsafe { libc::open(c.as_ptr(), libc::O_WRONLY | libc::O_CREAT | libc::O_TRUNC, 0o666) }
}

pub fn chdir(name: &[u8]) -> i32 {
    let c = cstr(name);
    unsafe { libc::chdir(c.as_ptr()) }
}

/// `Unlink`
pub fn unlink(name: &[u8]) {
    let c = cstr(name);
    unsafe {
        libc::unlink(c.as_ptr());
    }
}

/// `access(name, 0) == 0`
pub fn access_exists(name: &[u8]) -> bool {
    let c = cstr(name);
    unsafe { libc::access(c.as_ptr(), libc::F_OK) == 0 }
}

/// `Executable`: is `file` an executable non-directory (following links)?
pub fn executable(file: &[u8]) -> bool {
    let c = cstr(file);
    unsafe {
        let mut st: libc::stat = std::mem::zeroed();
        if libc::stat(c.as_ptr(), &mut st) < 0 {
            return false;
        }
        (st.st_mode & 0o111) != 0 && (st.st_mode & libc::S_IFMT) != libc::S_IFDIR
    }
}

pub fn isatty(fd: i32) -> bool {
    unsafe { libc::isatty(fd) == 1 }
}

/// C `strtol` with the "whole string must be consumed" check the callers
/// apply; `None` when invalid.
pub fn strtol(s: &[u8], base: i32) -> Option<i64> {
    let c = cstr(s);
    unsafe {
        let mut end: *mut libc::c_char = std::ptr::null_mut();
        let n = libc::strtol(c.as_ptr(), &mut end, base);
        if end == c.as_ptr() as *mut libc::c_char || *end != 0 {
            return None;
        }
        Some(n as i64)
    }
}

/// `Eintr`
pub fn eintr() -> bool {
    INTERRUPTED.load(Ordering::SeqCst)
}

/// `Noerror`
pub fn noerror() {
    INTERRUPTED.store(false, Ordering::SeqCst);
}

/// rc's `exitcode`: the numeric exit status for a status string.
pub fn exitcode(msg: &[u8]) -> i32 {
    let n = crate::word::atoi(msg);
    if n == 0 {
        1
    } else {
        n
    }
}

/// `readnb`: cope with non-blocking read.
fn readnb(fd: i32, buf: &mut [u8]) -> isize {
    let mut didreset = false;
    loop {
        let n = unsafe { libc::read(fd, buf.as_mut_ptr() as *mut libc::c_void, buf.len()) };
        if n == -1 && errno() == libc::EAGAIN {
            if !didreset {
                unsafe {
                    let flgs = libc::fcntl(fd, libc::F_GETFL, 0);
                    if flgs == -1 {
                        return -1;
                    }
                    if libc::fcntl(fd, libc::F_SETFL, flgs & !libc::O_NONBLOCK) == -1 {
                        return -1;
                    }
                }
                didreset = true;
            }
            continue;
        }
        return n;
    }
}

/// The wait message for a child, as lib9's `await` formats it.
fn waitmsg(status: i32) -> Word {
    if libc::WIFEXITED(status) {
        let s = libc::WEXITSTATUS(status);
        if s != 0 {
            return s.to_string().into_bytes();
        }
        return Vec::new();
    }
    if libc::WIFSIGNALED(status) {
        let mut m = b"signal: ".to_vec();
        m.extend_from_slice(crate::trap::sigstr(libc::WTERMSIG(status)).as_bytes());
        if libc::WCOREDUMP(status) {
            m.extend_from_slice(b" (core dumped)");
        }
        return m;
    }
    Vec::new()
}

impl Shell {
    /// `Read`: a read that processes pending traps afterwards.
    pub(crate) fn read(&mut self, fd: i32, buf: &mut [u8]) -> isize {
        let n = readnb(fd, buf);
        let e = errno();
        if crate::trap::ntrap() != 0 {
            self.dotrap(false);
        }
        // restore errno for the caller's EINTR check
        set_errno(e);
        n
    }

    /// `Exit`: terminate with the given status.
    pub(crate) fn exit(&mut self, stat: &[u8]) -> ! {
        let stat = stat.to_vec();
        self.setstatus(&stat);
        let status = self.getstatus();
        let code = if self.truestatus() { 0 } else { exitcode(&status) };
        let in_main = unsafe { libc::getpid() } == self.mypid;
        if in_main && self.exit_mode == ExitMode::Unwind {
            std::panic::resume_unwind(Box::new(ExitUnwind { status }));
        }
        if in_main {
            std::process::exit(code);
        }
        unsafe { libc::_exit(code) }
    }

    // ---- waitpid bookkeeping ----

    fn addwaitpid(&mut self, pid: i32) {
        self.waitpids.push(pid);
    }

    fn delwaitpid(&mut self, pid: i32) {
        self.waitpids.retain(|&p| p != pid);
    }

    fn clearwaitpids(&mut self) {
        self.waitpids.clear();
    }

    fn havewaitpid(&self, pid: i32) -> bool {
        self.waitpids.contains(&pid)
    }

    /// `Waitfor`: wait for `pid` (or any child when negative), recording
    /// the exit status of pipeline members in their threads.  Returns -1
    /// when interrupted.
    pub(crate) fn waitfor(&mut self, pid: i32, _unused: bool) -> i32 {
        if pid >= 0 && !self.havewaitpid(pid) {
            return 0;
        }
        let e;
        loop {
            let mut status: libc::c_int = 0;
            let mut ru: libc::rusage = unsafe { std::mem::zeroed() };
            let wpid = unsafe { libc::wait4(-1, &mut status, 0, &mut ru) };
            if wpid <= 0 {
                e = errno();
                break;
            }
            let msg = waitmsg(status);
            self.delwaitpid(wpid);
            if wpid == pid {
                if msg.starts_with(b"signal: ") {
                    let fd = self.mapfd(2);
                    let mut m = wpid.to_string().into_bytes();
                    m.extend_from_slice(b": ");
                    m.extend_from_slice(&msg);
                    m.push(b'\n');
                    write_all(fd, &m);
                }
                self.setstatus(&msg);
                return 0;
            }
            if self.qref().iflag && msg.starts_with(b"signal: ") {
                let mut m = wpid.to_string().into_bytes();
                m.extend_from_slice(b": ");
                m.extend_from_slice(&msg);
                m.push(b'\n');
                write_all(2, &m);
            }
            let mut p = self.q().ret.as_deref_mut();
            while let Some(t) = p {
                if t.pid == wpid {
                    t.pid = -1;
                    t.status = msg.clone();
                }
                p = t.ret.as_deref_mut();
            }
        }
        if e == libc::EINTR {
            return -1;
        }
        0
    }

    // ---- environment ----

    /// `enval`: split an environment value into a list.
    fn enval(s: &[u8]) -> Words {
        s.split(|&c| c == SEP).map(|w| w.to_vec()).collect()
    }

    /// `Vinit`: read variables from the environment.
    pub(crate) fn vinit(&mut self) {
        let env = self.environ.clone();
        for e in env {
            let pos = e.iter().position(|&c| c == b'(' || c == b'=');
            match pos {
                Some(i) if e[i] == b'=' => {
                    let name = e[..i].to_vec();
                    let val = Shell::enval(&e[i + 1..]);
                    self.setvar(&name, val);
                }
                _ => {
                    // functions (Bourne style) or odd entries are ignored
                }
            }
        }
    }

    /// `Xrdfn`: define the next function found in the environment.
    pub(crate) fn xrdfn(&mut self) {
        while self.envp < self.environ.len() {
            let i = self.envp;
            let s = self.environ[i].clone();
            if s.starts_with(b"fn#") {
                if let Some(p) = s.iter().position(|&c| c == b'=') {
                    let mut t = s.clone();
                    t[p] = b' ';
                    t[2] = b' ';
                    // the C code rewrites the entry in place, so it is not
                    // matched again on the next call
                    self.environ[i] = t.clone();
                    self.execcmds(Io::opencore(&t));
                    return;
                }
            }
            self.envp += 1;
        }
        self.xreturn();
    }

    /// `mkenv`: build the environment for a child process.
    fn mkenv(&mut self) -> Vec<Word> {
        let mut env: Vec<Word> = Vec::new();
        // locals first, then globals; only the visible binding of a name
        let mut vars: Vec<crate::var::VarRef> = Vec::new();
        let mut l = self.qref().local.clone();
        while let Some(node) = l {
            vars.push(node.var.clone());
            l = node.next.clone();
        }
        for v in self.gvar.values() {
            vars.push(v.clone());
        }
        for v in vars {
            let name = v.borrow().name.clone();
            let visible = Rc::ptr_eq(&self.vlook(&name), &v);
            let vb = v.borrow();
            if visible && !vb.val.is_empty() {
                let mut e = name.clone();
                let mut sep = b'=';
                for w in &vb.val {
                    e.push(sep);
                    sep = SEP;
                    e.extend_from_slice(w);
                }
                env.push(e);
            }
            if let Some((code, pc)) = &vb.func {
                let mut e = b"fn#".to_vec();
                e.extend_from_slice(&name);
                e.push(b'=');
                e.extend_from_slice(code_s(code, pc - 1));
                e.push(b'\n');
                env.push(e);
            }
        }
        env.sort();
        env
    }

    /// `Execute`: try to exec `args[0]` in each directory of `path`.
    pub(crate) fn execute(&mut self, args: &Words, path: &Words) {
        let argv: Vec<CString> = args.iter().map(|a| cstr(a)).collect();
        let env: Vec<CString> = self.mkenv().into_iter().map(|e| cstr(&e)).collect();
        let mut argp: Vec<*const libc::c_char> = argv.iter().map(|a| a.as_ptr()).collect();
        argp.push(std::ptr::null());
        let mut envp: Vec<*const libc::c_char> = env.iter().map(|e| e.as_ptr()).collect();
        envp.push(std::ptr::null());
        let name = &args[0];
        let mut lasterr: Word = errstr(errno());
        for p in path {
            let mut nc = p.len();
            if nc < 1024 {
                let mut file = p.clone();
                if !file.is_empty() {
                    file.push(b'/');
                    nc += 1;
                }
                if nc + name.len() < 1024 {
                    file.extend_from_slice(name);
                    let cfile = cstr(&file);
                    unsafe {
                        libc::execve(cfile.as_ptr(), argp.as_ptr(), envp.as_ptr());
                    }
                    lasterr = errstr(errno());
                } else {
                    lasterr = b"command name too long".to_vec();
                }
            }
        }
        self.err.pstr(name);
        self.err.pstr(b": ");
        self.err.pstr(&lasterr);
        self.err.pchr(b'\n');
        self.err.flush();
    }

    // ---- fork-based operations (havefork.c) ----

    fn fork(&mut self) -> i32 {
        self.err.flush();
        unsafe { libc::fork() }
    }

    /// `Xasync`
    pub(crate) fn xasync(&mut self) {
        let (null, e) = open_flags(b"/dev/null", libc::O_RDONLY);
        if null < 0 {
            self.xerror("Can't open /dev/null\n", e);
            return;
        }
        // rfork(RFFDG|RFPROC|RFNOTEG): fork, then a new process group
        let pid = self.fork();
        match pid {
            -1 => {
                let e = errno();
                unsafe {
                    libc::close(null);
                }
                self.xerror("try again", e);
            }
            0 => {
                unsafe {
                    libc::setpgid(0, libc::getpid());
                }
                self.clearwaitpids();
                /*
                 * If we just dup /dev/null onto 0, then running
                 * ssh foo & will reopen /dev/tty, try to read a password,
                 * get a signal, and repeat, in a tight loop, forever.
                 * If we dissociate the process from the tty, then it won't
                 * be able to open /dev/tty ever again.  The SIG_IGN on
                 * SIGTTOU makes writing the tty (via fd 1 or 2, for
                 * example) succeed even though our pgrp is not the
                 * terminal's controlling pgrp.
                 */
                let (tty, _) = open_flags(b"/dev/tty", libc::O_RDONLY);
                if tty >= 0 {
                    unsafe {
                        libc::signal(libc::SIGTTIN, libc::SIG_IGN);
                        libc::signal(libc::SIGTTOU, libc::SIG_IGN);
                        libc::ioctl(tty, libc::TIOCNOTTY as _);
                        libc::close(tty);
                    }
                }
                if isatty(0) {
                    self.pushredir(ROPEN, null, 0);
                } else {
                    unsafe {
                        libc::close(null);
                    }
                }
                let (code, pc, local) = {
                    let q = self.qref();
                    (q.code.clone(), q.pc + 1, q.local.clone())
                };
                self.start(code, pc, local);
                self.q().ret = None;
            }
            _ => {
                self.addwaitpid(pid);
                unsafe {
                    libc::close(null);
                }
                let a = code_i(&self.qref().code, self.qref().pc);
                self.q().pc = a as usize;
                let mut w = Words::new();
                w.push_back(pid.to_string().into_bytes());
                self.setvar(b"apid", w);
            }
        }
    }

    /// `Xpipe`
    pub(crate) fn xpipe(&mut self) {
        let pc = self.qref().pc;
        let code = self.qref().code.clone();
        let lfd = code_i(&code, pc);
        let rfd = code_i(&code, pc + 1);
        let mut pfd = [0i32; 2];
        if unsafe { libc::pipe(pfd.as_mut_ptr()) } < 0 {
            let e = errno();
            self.xerror("can't get pipe", e);
            return;
        }
        let local = self.qref().local.clone();
        let forkid = self.fork();
        match forkid {
            -1 => {
                let e = errno();
                self.xerror("try again", e);
            }
            0 => {
                self.clearwaitpids();
                self.start(code, pc + 4, local);
                self.q().ret = None;
                unsafe {
                    libc::close(pfd[0]);
                }
                self.pushredir(ROPEN, pfd[1], lfd);
            }
            _ => {
                self.addwaitpid(forkid);
                let right = code_i(&code, pc + 2) as usize;
                let after = code_i(&code, pc + 3) as usize;
                self.start(code, right, local);
                unsafe {
                    libc::close(pfd[1]);
                }
                self.pushredir(ROPEN, pfd[0], rfd);
                let p = self.q().ret.as_deref_mut().expect("rc: Xpipe without parent thread");
                p.pc = after;
                p.pid = forkid;
            }
        }
    }

    /// `Xbackq`
    pub(crate) fn xbackq(&mut self) {
        let stop: Word = self.qref().argv.last().and_then(|l| l.front().cloned()).unwrap_or_default();
        let mut pfd = [0i32; 2];
        if unsafe { libc::pipe(pfd.as_mut_ptr()) } < 0 {
            let e = errno();
            self.xerror("can't make pipe", e);
            return;
        }
        let pid = self.fork();
        match pid {
            -1 => {
                let e = errno();
                self.xerror("try again", e);
                unsafe {
                    libc::close(pfd[0]);
                    libc::close(pfd[1]);
                }
            }
            0 => {
                self.clearwaitpids();
                unsafe {
                    libc::close(pfd[0]);
                }
                let (code, pc, local) = {
                    let q = self.qref();
                    (q.code.clone(), q.pc + 1, q.local.clone())
                };
                self.start(code, pc, local);
                self.pushredir(ROPEN, pfd[1], 1);
            }
            _ => {
                self.addwaitpid(pid);
                unsafe {
                    libc::close(pfd[1]);
                }
                let f = Rc::new(std::cell::RefCell::new(Io::openfd(pfd[0])));
                let stops = utf8_runes(&stop);
                let mut wd: Vec<u8> = Vec::new();
                let mut v: Vec<Word> = Vec::new();
                const EWD: usize = 8192;
                loop {
                    let c = f.borrow_mut().rchr_with(|fd, buf| self.read(fd, buf));
                    if c == crate::io::EOF {
                        break;
                    }
                    let mut split = false;
                    if wd.len() != EWD {
                        wd.push(c as u8);
                        for q in &stops {
                            let n = q.len();
                            if wd.len() >= n && &wd[wd.len() - n..] == q.as_slice() {
                                wd.truncate(wd.len() - n);
                                split = true;
                                break;
                            }
                        }
                        if !split {
                            continue;
                        }
                    }
                    if !wd.is_empty() {
                        v.push(cword(&wd));
                    }
                    wd.clear();
                }
                if !wd.is_empty() {
                    v.push(cword(&wd));
                }
                if let Ok(f) = Rc::try_unwrap(f) {
                    f.into_inner().close();
                }
                self.waitfor(pid, false);
                self.poplist(); // ditch split in "stop"
                let top = self.top();
                for w in v.into_iter().rev() {
                    top.push_front(w);
                }
                let a = code_i(&self.qref().code, self.qref().pc);
                self.q().pc = a as usize;
            }
        }
    }

    /// `Xpipefd`
    pub(crate) fn xpipefd(&mut self) {
        let pc = self.qref().pc;
        let code = self.qref().code.clone();
        let ty = code_i(&code, pc);
        let mut r: Option<(i32, i32)> = None; // (sidefd, mainfd)
        let mut w: Option<(i32, i32)> = None;
        let want_r = ty != crate::tree::WRITE;
        let want_w = ty != crate::tree::READ;
        if want_r {
            let mut pfd = [0i32; 2];
            if unsafe { libc::pipe(pfd.as_mut_ptr()) } < 0 {
                let e = errno();
                self.xerror("can't get pipe", e);
                return;
            }
            r = Some((pfd[1], pfd[0]));
        }
        if want_w {
            let mut pfd = [0i32; 2];
            if unsafe { libc::pipe(pfd.as_mut_ptr()) } < 0 {
                let e = errno();
                self.xerror("can't get pipe", e);
                return;
            }
            w = Some((pfd[0], pfd[1]));
        }
        let local = self.qref().local.clone();
        let pid = self.fork();
        match pid {
            -1 => {
                let e = errno();
                self.xerror("try again", e);
            }
            0 => {
                self.clearwaitpids();
                self.start(code, pc + 2, local);
                if let Some((side, main)) = r {
                    unsafe {
                        libc::close(main);
                    }
                    self.pushredir(ROPEN, side, 1);
                }
                if let Some((side, main)) = w {
                    unsafe {
                        libc::close(main);
                    }
                    self.pushredir(ROPEN, side, 0);
                }
                self.q().ret = None;
            }
            _ => {
                self.addwaitpid(pid);
                if let Some((side, main)) = w {
                    unsafe {
                        libc::close(side);
                    }
                    // so that Xpopredir can close it later
                    self.pushredir(ROPEN, main, main);
                    let mut name = FDPREFIX.to_vec();
                    name.extend_from_slice(main.to_string().as_bytes());
                    self.pushword(name);
                }
                if let Some((side, main)) = r {
                    unsafe {
                        libc::close(side);
                    }
                    self.pushredir(ROPEN, main, main);
                    let mut name = FDPREFIX.to_vec();
                    name.extend_from_slice(main.to_string().as_bytes());
                    self.pushword(name);
                }
                let a = code_i(&code, pc + 1);
                self.q().pc = a as usize;
            }
        }
    }

    /// `Xsubshell`
    pub(crate) fn xsubshell(&mut self) {
        let pid = self.fork();
        match pid {
            -1 => {
                let e = errno();
                self.xerror("try again", e);
            }
            0 => {
                self.clearwaitpids();
                let (code, pc, local) = {
                    let q = self.qref();
                    (q.code.clone(), q.pc + 1, q.local.clone())
                };
                self.start(code, pc, local);
                self.q().ret = None;
            }
            _ => {
                self.addwaitpid(pid);
                self.waitfor(pid, true);
                let a = code_i(&self.qref().code, self.qref().pc);
                self.q().pc = a as usize;
            }
        }
    }

    /// `execforkexec`: fork and exec the current simple command.
    pub(crate) fn execforkexec(&mut self) -> i32 {
        let pid = self.fork();
        match pid {
            -1 => -1,
            0 => {
                self.clearwaitpids();
                self.pushword(b"exec".to_vec());
                self.execexec();
                let mut buf = b"can't exec: ".to_vec();
                buf.extend_from_slice(&errstr(errno()));
                self.exit(&buf)
            }
            _ => {
                self.addwaitpid(pid);
                pid
            }
        }
    }
}

/// Words are C strings: they end at the first NUL byte.
fn cword(w: &[u8]) -> Word {
    let end = w.iter().position(|&c| c == 0).unwrap_or(w.len());
    w[..end].to_vec()
}

/// Split a byte string into the byte sequences of its runes, the way
/// repeated `chartorune` calls would (an invalid sequence counts as one
/// byte).
fn utf8_runes(s: &[u8]) -> Vec<Vec<u8>> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < s.len() {
        let c = s[i];
        let n = if c < 0x80 {
            1
        } else if c & 0xe0 == 0xc0 {
            2
        } else if c & 0xf0 == 0xe0 {
            3
        } else if c & 0xf8 == 0xf0 {
            4
        } else {
            1
        };
        let n = if n > 1 && i + n <= s.len() && std::str::from_utf8(&s[i..i + n]).is_ok() { n } else { 1 };
        out.push(s[i..i + n].to_vec());
        i += n;
    }
    out
}

pub fn set_errno(e: i32) {
    #[cfg(any(target_os = "macos", target_os = "ios", target_os = "freebsd"))]
    unsafe {
        *libc::__error() = e;
    }
    #[cfg(any(target_os = "linux", target_os = "android"))]
    unsafe {
        *libc::__errno_location() = e;
    }
}

pub fn kidpid() -> i32 {
    KIDPID.load(Ordering::SeqCst)
}

pub fn ntrap_raw() -> i32 {
    NTRAP.load(Ordering::SeqCst)
}
