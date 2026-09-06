//! Signal handling (port of `trap.c`, the `notifyf` part of `plan9ish.c`,
//! and the pieces of lib9's `notify.c`/`await.c` that rc depends on).

use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};

use crate::shell::{Shell, NSIG};

pub static NTRAP: AtomicI32 = AtomicI32::new(0);
#[allow(clippy::declare_interior_mutable_const)]
const ZERO: AtomicI32 = AtomicI32::new(0);
pub static TRAP: [AtomicI32; NSIG] = [ZERO; NSIG];
pub static INTERRUPTED: AtomicBool = AtomicBool::new(false);
pub static KIDPID: AtomicI32 = AtomicI32::new(0);

/// The names of the trap functions, indexed by trap number.
pub const SIGNAME: [&str; 8] = ["sigexit", "sighup", "sigint", "sigquit", "sigalrm", "sigkill", "sigfpe", "sigterm"];
/// The (prefixes of the) note strings corresponding to `SIGNAME`.
const SYSSIGNAME: [&str; 8] = [
    "exit", // can't happen
    "hangup",
    "interrupt",
    "quit", // can't happen
    "alarm",
    "kill",
    "sys: fp: ",
    "term",
];

pub fn ntrap() -> i32 {
    NTRAP.load(Ordering::SeqCst)
}

/// lib9's `_p9sigstr`: the Plan 9 note for a Unix signal, for the
/// signals with a fixed name.  (Everything used from the signal handler
/// must avoid allocation: the handler may run while `fork` holds the
/// allocator lock.)
fn sigstr_static(sig: i32) -> Option<&'static str> {
    Some(match sig {
        libc::SIGHUP => "hangup",
        libc::SIGINT => "interrupt",
        libc::SIGQUIT => "quit",
        libc::SIGILL => "sys: illegal instruction",
        libc::SIGTRAP => "sys: breakpoint",
        libc::SIGABRT => "sys: abort",
        #[cfg(any(target_os = "macos", target_os = "freebsd", target_os = "netbsd", target_os = "openbsd"))]
        libc::SIGEMT => "sys: emulate instruction executed",
        libc::SIGFPE => "sys: fp: trap",
        libc::SIGKILL => "sys: kill",
        libc::SIGBUS => "sys: bus error",
        libc::SIGSEGV => "sys: segmentation violation",
        libc::SIGALRM => "alarm",
        libc::SIGTERM => "kill",
        libc::SIGURG => "sys: urgent condition on socket",
        libc::SIGSTOP => "sys: stop",
        libc::SIGTSTP => "sys: tstp",
        libc::SIGCONT => "sys: cont",
        libc::SIGCHLD => "sys: child",
        libc::SIGTTIN => "sys: ttin",
        libc::SIGTTOU => "sys: ttou",
        libc::SIGIO => "sys: i/o possible on fd",
        libc::SIGXCPU => "sys: cpu time limit exceeded",
        libc::SIGXFSZ => "sys: file size limit exceeded",
        libc::SIGVTALRM => "sys: virtual time alarm",
        libc::SIGPROF => "sys: profiling timer alarm",
        libc::SIGWINCH => "sys: window size change",
        #[cfg(any(target_os = "macos", target_os = "freebsd", target_os = "netbsd", target_os = "openbsd"))]
        libc::SIGINFO => "sys: status request",
        libc::SIGUSR1 => "sys: usr1",
        libc::SIGUSR2 => "sys: usr2",
        libc::SIGPIPE => "sys: write on closed pipe",
        _ => return None,
    })
}

/// lib9's `_p9sigstr`: the Plan 9 note for a Unix signal.
pub fn sigstr(sig: i32) -> String {
    match sigstr_static(sig) {
        Some(s) => s.to_string(),
        None => format!("sys: signal {}", sig),
    }
}

/// A fixed-size byte buffer for building messages inside the signal
/// handler without allocating.
struct Buf {
    b: [u8; 128],
    n: usize,
}

impl Buf {
    fn new() -> Buf {
        Buf { b: [0; 128], n: 0 }
    }
    fn push(&mut self, s: &[u8]) {
        for &c in s {
            if self.n < self.b.len() {
                self.b[self.n] = c;
                self.n += 1;
            }
        }
    }
    fn push_num(&mut self, mut v: u32) {
        let mut d = [0u8; 10];
        let mut i = d.len();
        loop {
            i -= 1;
            d[i] = b'0' + (v % 10) as u8;
            v /= 10;
            if v == 0 {
                break;
            }
        }
        self.push(&d[i..]);
    }
    fn as_bytes(&self) -> &[u8] {
        &self.b[..self.n]
    }
}

/// The note for `sig` written into `buf` (no allocation).
fn sigstr_into(sig: i32, buf: &mut Buf) {
    match sigstr_static(sig) {
        Some(s) => buf.push(s.as_bytes()),
        None => {
            buf.push(b"sys: signal ");
            buf.push_num(sig as u32);
        }
    }
}

const RESTART: u32 = 1 << 0;
const IGNORE: u32 = 1 << 1;
const NONOTIFY: u32 = 1 << 2;

/// lib9's table of handled signals.
static SIGS: &[(i32, u32)] = &[
    (libc::SIGHUP, 0),
    (libc::SIGINT, 0),
    (libc::SIGQUIT, 0),
    (libc::SIGILL, 0),
    (libc::SIGTRAP, 0),
    #[cfg(any(target_os = "macos", target_os = "freebsd", target_os = "netbsd", target_os = "openbsd"))]
    (libc::SIGEMT, 0),
    (libc::SIGFPE, 0),
    (libc::SIGBUS, 0),
    (libc::SIGCHLD, RESTART | IGNORE),
    (libc::SIGSYS, 0),
    (libc::SIGPIPE, IGNORE),
    (libc::SIGALRM, 0),
    (libc::SIGTERM, 0),
    (libc::SIGTSTP, RESTART | IGNORE | NONOTIFY),
    (libc::SIGXCPU, 0),
    (libc::SIGXFSZ, 0),
    (libc::SIGVTALRM, 0),
    (libc::SIGUSR1, 0),
    (libc::SIGUSR2, 0),
    (libc::SIGWINCH, RESTART | IGNORE | NONOTIFY),
    #[cfg(any(target_os = "macos", target_os = "freebsd", target_os = "netbsd", target_os = "openbsd"))]
    (libc::SIGINFO, RESTART | IGNORE | NONOTIFY),
];

fn sigflags(sig: i32) -> Option<u32> {
    SIGS.iter().find(|(s, _)| *s == sig).map(|(_, f)| *f)
}

fn write2(s: &[u8]) {
    crate::unix::write_all(2, s);
}

/// The signal handler: lib9's `signotify` calling rc's `notifyf`.  It
/// must be async-signal-safe, so it uses only atomics, stack buffers and
/// raw system calls.
extern "C" fn signotify(sig: libc::c_int) {
    let mut sb = Buf::new();
    sigstr_into(sig, &mut sb);
    let s = sb.as_bytes();
    // rc's notifyf
    for (i, name) in SYSSIGNAME.iter().enumerate() {
        if s.starts_with(name.as_bytes()) {
            if !s.starts_with(b"sys: ") {
                // (kidpid is never set in plan9port's rc)
                INTERRUPTED.store(true, Ordering::SeqCst);
            }
            if s != b"interrupt" || TRAP[i].load(Ordering::SeqCst) == 0 {
                TRAP[i].fetch_add(1, Ordering::SeqCst);
                NTRAP.fetch_add(1, Ordering::SeqCst);
            }
            if NTRAP.load(Ordering::SeqCst) >= 32 {
                // rc is probably in a trap loop
                let mut m = Buf::new();
                m.push(b"rc: Too many traps (trap ");
                m.push(s);
                m.push(b"), aborting\n");
                write2(m.as_bytes());
                unsafe { libc::abort() };
            }
            return; // noted(NCONT)
        }
    }
    if s != b"sys: window size change" && s != b"sys: write on closed pipe" && s != b"sys: child" {
        let mut m = Buf::new();
        m.push(b"rc: note: ");
        m.push(s);
        m.push(b"\n");
        write2(m.as_bytes());
    }
    // noted(NDFLT)
    if let Some(f) = sigflags(sig) {
        if f & IGNORE != 0 {
            return;
        }
    }
    unsafe {
        libc::signal(sig, libc::SIG_DFL);
        libc::raise(sig);
        libc::_exit(1);
    }
}

extern "C" fn signonotify(_sig: libc::c_int) {}

/// `Trapinit` (lib9's `notify`): install the signal handlers.  Signals
/// that already have a non-default disposition are left alone.
pub fn trapinit() {
    for &(sig, flags) in SIGS {
        unsafe {
            let mut old: libc::sigaction = std::mem::zeroed();
            libc::sigaction(sig, std::ptr::null(), &mut old);
            if old.sa_sigaction != libc::SIG_DFL {
                continue;
            }
            let mut sa: libc::sigaction = std::mem::zeroed();
            sa.sa_sigaction = if flags & NONOTIFY != 0 {
                signonotify as extern "C" fn(libc::c_int) as libc::sighandler_t
            } else {
                signotify as extern "C" fn(libc::c_int) as libc::sighandler_t
            };
            if flags & RESTART != 0 {
                sa.sa_flags |= libc::SA_RESTART;
            }
            // We can't allow signals within signals because there's only
            // one jump buffer.
            libc::sigfillset(&mut sa.sa_mask);
            libc::sigaction(sig, &sa, std::ptr::null_mut());
        }
    }
}

impl Shell {
    /// `dotrap`: run the pending traps.  When `allow_fn` is false (the
    /// interpreter is in the middle of reading input) traps that would run
    /// a user-defined function are left pending until the main loop, since
    /// starting a thread there is unsafe (the C implementation crashes).
    pub(crate) fn dotrap(&mut self, allow_fn: bool) {
        let starval = self.vlook(b"*").borrow().val.clone();
        while ntrap() != 0 {
            let mut progressed = false;
            for i in 0..NSIG {
                while TRAP[i].load(Ordering::SeqCst) != 0 {
                    let name = SIGNAME.get(i).copied();
                    let trapreq = name.map(|n| self.vlook(n.as_bytes()));
                    let has_fn = trapreq.as_ref().map(|v| v.borrow().func.is_some()).unwrap_or(false);
                    if !allow_fn && has_fn {
                        break;
                    }
                    TRAP[i].fetch_sub(1, Ordering::SeqCst);
                    NTRAP.fetch_sub(1, Ordering::SeqCst);
                    progressed = true;
                    if unsafe { libc::getpid() } != self.mypid {
                        let st = self.getstatus();
                        self.exit(&st);
                    }
                    if has_fn {
                        let (code, pc) = trapreq.unwrap().borrow().func.clone().unwrap();
                        self.start(code, pc, None);
                        self.push_local(b"*".to_vec(), starval.clone());
                        let q = self.q();
                        q.redir = None;
                        q.startredir = None;
                    } else if i == 2 /* SIGINT */ || i == 3 /* SIGQUIT */ {
                        // run the stack down until we uncover the command
                        // reading loop.  Xreturn will exit if there is none
                        // (i.e. if this is not an interactive rc.)
                        while let Some(q) = &self.runq {
                            if q.iflag {
                                break;
                            }
                            self.xreturn();
                        }
                    } else {
                        let st = self.getstatus();
                        self.exit(&st);
                    }
                }
            }
            if !progressed {
                break;
            }
        }
    }
}

/// The `ntrap = 0` reset in `Xrdcmds` ("avoid double-interrupts during
/// blocked writes").  Traps whose handler function is defined are kept,
/// since they may have been deferred by [`Shell::dotrap`].
pub(crate) fn clear_traps_without_handlers(sh: &mut Shell) {
    for i in 0..NSIG {
        let has_fn = SIGNAME.get(i).map(|n| sh.vlook(n.as_bytes()).borrow().func.is_some()).unwrap_or(false);
        if !has_fn {
            let n = TRAP[i].swap(0, Ordering::SeqCst);
            NTRAP.fetch_sub(n, Ordering::SeqCst);
        }
    }
    if NTRAP.load(Ordering::SeqCst) < 0 {
        NTRAP.store(0, Ordering::SeqCst);
    }
}
