//! # rust-rc
//!
//! A faithful Rust port of the Plan 9 shell `rc`, as found in
//! [plan9port](https://github.com/9fans/plan9port).  The lexer, parser,
//! compiler, interpreter, builtins, globbing, here documents, environment
//! handling and signal behaviour follow the C sources one for one, so that
//! scripts behave identically (including error messages and exit codes).
//!
//! The crate provides:
//!
//! * a library: create a [`Shell`], evaluate rc code in it, and inspect or
//!   set its variables;
//! * an `rc` binary that is a drop-in replacement for plan9port's `rc`.
//!
//! ```no_run
//! use rust_rc::Shell;
//!
//! let mut sh = Shell::new();
//! sh.eval("x=(a b c); echo $x(2)").unwrap();
//! assert_eq!(sh.var("x"), vec!["a", "b", "c"]);
//! let (out, _status) = sh.eval_capture("echo hello").unwrap();
//! assert_eq!(out, b"hello\n");
//! ```
//!
//! ## Caveats for embedding
//!
//! rc is a Unix shell: pipelines, subshells, backquotes and background
//! commands are implemented with `fork(2)`, exactly as in C, and the
//! interpreter runs in the child processes too.  Embedding programs should
//! therefore evaluate rc code from a single thread and be aware that the
//! child processes exit with `_exit(2)` without running the host's atexit
//! handlers or destructors.  `exit` inside evaluated code unwinds back to
//! the caller (see [`Shell::eval`]); the interpreter relies on
//! `panic = "unwind"`.

#![allow(clippy::new_without_default)]

pub mod code;
pub mod exec;
pub mod getflags;
pub mod glob;
pub mod here;
pub mod io;
pub mod lex;
pub mod parse;
pub mod pcmd;
pub mod shell;
pub mod simple;
pub mod trap;
pub mod tree;
pub mod unix;
pub mod var;
pub mod word;

use std::collections::HashMap;
use std::rc::Rc;

pub use shell::Shell;
use shell::*;
use word::{Word, Words};

/// The `rcmain` startup script: plan9port's Unix version, extended so that
/// interactive shells read `$home/lib/rcrc` (after `$home/lib/profile` for
/// login shells).  The `rc` binary runs it unless `-m` names another file.
pub const RCMAIN: &str = include_str!("../rcmain");

/// The portion of `rcmain` that sets up variables and functions without
/// reading commands from anywhere; [`Shell::new`] evaluates it.
pub const RCMAIN_PRELUDE: &str = "if(~ $#home 0) home=$HOME
if(~ $#home 0) home=/
if(~ $#ifs 0) ifs=' \t
'
switch($#prompt){
case 0
	prompt=('% ' '\t')
case 1
	prompt=($prompt '\t')
}
if(flag p) path=(/bin /usr/bin)
if not{
	finit
}
fn sigexit
";

/// The default location of the plan9port tree when `$PLAN9` is unset.
pub const DEFAULT_PLAN9: &str = "/usr/local/plan9";

/// The result of evaluating rc code: the final value of `$status`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Status(pub Vec<u8>);

impl Status {
    /// rc's notion of truth: the status is empty or consists only of `0`
    /// and `|` characters.
    pub fn is_true(&self) -> bool {
        self.0.iter().all(|&c| c == b'|' || c == b'0')
    }

    /// The process exit code rc would use for this status.
    pub fn exit_code(&self) -> i32 {
        if self.is_true() {
            0
        } else {
            unix::exitcode(&self.0)
        }
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

impl std::fmt::Display for Status {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", String::from_utf8_lossy(&self.0))
    }
}

/// The evaluated code asked the shell to exit (the `exit` builtin, a fatal
/// signal trap, or `-e`).  The embedded shell does not terminate the
/// process; it reports the requested status instead.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Exited {
    pub status: Status,
}

impl std::fmt::Display for Exited {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "rc exited with status `{}`", self.status)
    }
}

impl std::error::Error for Exited {}

/// Configuration for creating a [`Shell`].
#[derive(Debug, Clone)]
pub struct Builder {
    argv0: Word,
    install_signal_handlers: bool,
    import_environment: bool,
    set_plan9_env: bool,
    run_prelude: bool,
    flags: Vec<(u8, Option<Word>)>,
}

impl Builder {
    fn new() -> Builder {
        Builder {
            argv0: b"rc".to_vec(),
            install_signal_handlers: true,
            import_environment: true,
            set_plan9_env: true,
            run_prelude: true,
            flags: Vec::new(),
        }
    }

    /// The name used in error messages (`rc:` or `rc (name):`).
    pub fn argv0(mut self, name: &str) -> Builder {
        self.argv0 = name.as_bytes().to_vec();
        self
    }

    /// Install rc's signal handlers in the process (default: true).  Like
    /// the C implementation, only signals with the default disposition are
    /// touched.
    pub fn install_signal_handlers(mut self, yes: bool) -> Builder {
        self.install_signal_handlers = yes;
        self
    }

    /// Import the process environment as rc variables (default: true).
    pub fn import_environment(mut self, yes: bool) -> Builder {
        self.import_environment = yes;
        self
    }

    /// Set `$PLAN9` in the process environment when it is unset, as the
    /// `rc` binary does (default: true).
    pub fn set_plan9_env(mut self, yes: bool) -> Builder {
        self.set_plan9_env = yes;
        self
    }

    /// Evaluate [`RCMAIN_PRELUDE`] (default: true), which sets `$home`,
    /// `$ifs`, `$prompt`, and reads function definitions from the
    /// environment.
    pub fn run_prelude(mut self, yes: bool) -> Builder {
        self.run_prelude = yes;
        self
    }

    /// Set a command line flag (e.g. `b'e'` for `-e`, or `b'x'`).
    pub fn flag(mut self, c: u8, value: Option<&str>) -> Builder {
        self.flags.push((c, value.map(|v| v.as_bytes().to_vec())));
        self
    }

    pub fn build(self) -> Shell {
        let mut sh = Shell::raw(ExitMode::Unwind, self.argv0.clone());
        for (c, v) in &self.flags {
            if (*c as usize) < NFLAG {
                sh.flag[*c as usize] = Some(v.iter().cloned().collect());
            }
        }
        sh.init(self.install_signal_handlers, self.import_environment, self.set_plan9_env);
        if self.run_prelude {
            // like rcmain itself, the prelude runs before -e takes effect
            let _ = sh.eval_internal(RCMAIN_PRELUDE.as_bytes(), false);
        }
        sh
    }
}

impl Shell {
    /// A shell with the default configuration (see [`Builder`]).
    pub fn new() -> Shell {
        Builder::new().build()
    }

    pub fn builder() -> Builder {
        Builder::new()
    }

    /// The uninitialised interpreter state.
    pub(crate) fn raw(exit_mode: ExitMode, argv0: Word) -> Shell {
        let rdcmds_code: Code = Rc::new(vec![Instr::I(1), Instr::F(Op::Rdcmds), Instr::F(Op::Return)]);
        let dotcmds_code: Code = Rc::new(vec![
            Instr::I(1),
            Instr::F(Op::Mark),
            Instr::F(Op::Word),
            Instr::S(b"0".to_vec()),
            Instr::F(Op::Local),
            Instr::F(Op::Mark),
            Instr::F(Op::Word),
            Instr::S(b"*".to_vec()),
            Instr::F(Op::Local),
            Instr::F(Op::Rdcmds),
            Instr::F(Op::Unlocal),
            Instr::F(Op::Unlocal),
            Instr::F(Op::Return),
        ]);
        let rdfns_code: Code = Rc::new(vec![Instr::I(1), Instr::F(Op::Rdfn), Instr::F(Op::Jump), Instr::I(1)]);
        Shell {
            runq: None,
            gvar: HashMap::new(),
            mypid: unsafe { libc::getpid() },
            nerror: 0,
            ndot: 0,
            lastc: 0,
            err: io::Io::openfd(2),
            promptstr: Vec::new(),
            eflagok: false,
            ifnot: false,
            argv0,
            exit_mode,
            exit_beenhere: false,
            flag: vec![None; NFLAG],
            tok: Vec::new(),
            tok_overflow: false,
            future: tree::EOF,
            peekc: tree::EOF,
            doprompt: true,
            inquote: false,
            incomm: false,
            lastdol: false,
            lastword: false,
            yylval: None,
            here: Vec::new(),
            here_ser: 0,
            codebuf: Vec::new(),
            waitpids: Vec::new(),
            environ: Vec::new(),
            envp: 0,
            wdirfd: -2,
            dot_first: true,
            rcmain_path: Vec::new(),
            rdcmds_code,
            dotcmds_code,
            rdfns_code,
        }
    }

    /// The initialisation done by `main` before the bootstrap code runs.
    pub(crate) fn init(&mut self, signals: bool, import_env: bool, set_plan9: bool) {
        if set_plan9 && std::env::var_os("PLAN9").is_none() {
            std::env::set_var("PLAN9", DEFAULT_PLAN9);
        }
        self.environ = std::env::vars_os()
            .map(|(k, v)| {
                use std::os::unix::ffi::OsStrExt;
                let mut e = k.as_bytes().to_vec();
                e.push(b'=');
                e.extend_from_slice(v.as_bytes());
                e
            })
            .collect();
        if signals {
            trap::trapinit();
        }
        if import_env {
            self.vinit();
        }
        self.mypid = unsafe { libc::getpid() };
        self.pathinit();
        let mut w = Words::new();
        w.push_back(self.mypid.to_string().into_bytes());
        self.setvar(b"pid", w);
        let cflag: Words = self.flag_arg(b'c').cloned().into_iter().collect();
        self.setvar(b"cflag", cflag);
        let mut w = Words::new();
        w.push_back(self.argv0.clone());
        self.setvar(b"rcname", w);
    }

    /// Run the interpreter until the current evaluation finishes, turning
    /// an `exit` into an `Err`.
    fn run_embedded(&mut self) -> Result<Status, Exited> {
        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| self.run_loop()));
        match r {
            Ok(()) => Ok(Status(self.getstatus())),
            Err(payload) => match payload.downcast::<ExitUnwind>() {
                Ok(e) => {
                    self.runq = None;
                    Err(Exited { status: Status(e.status) })
                }
                Err(payload) => std::panic::resume_unwind(payload),
            },
        }
    }

    /// Evaluate `src` as rc input (like the `eval` builtin, or `rc -c`).
    /// Returns the final `$status`, or [`Exited`] if the code executed
    /// `exit`.  Errors in the code are reported on standard error, as rc
    /// does, and set `$status` to `error`.
    pub fn eval(&mut self, src: &str) -> Result<Status, Exited> {
        self.eval_bytes(src.as_bytes())
    }

    /// [`Shell::eval`] for input that is not valid UTF-8.
    pub fn eval_bytes(&mut self, src: &[u8]) -> Result<Status, Exited> {
        self.eval_internal(src, true)
    }

    fn eval_internal(&mut self, src: &[u8], eflagok: bool) -> Result<Status, Exited> {
        if eflagok {
            // as the `eval` builtin does: -e is honoured from now on
            self.eflagok = true;
        }
        let mut input = src.to_vec();
        if input.last() != Some(&b'\n') {
            input.push(b'\n');
        }
        let saved = self.runq.take();
        let code = self.rdcmds_code.clone();
        self.start(code, 1, None);
        self.q().cmdfd = Some(Rc::new(std::cell::RefCell::new(io::Io::opencore(&input))));
        self.q().iflast = false;
        let r = self.run_embedded();
        self.runq = saved;
        r
    }

    /// Evaluate `src` with standard output captured; returns the output
    /// and the final status.
    pub fn eval_capture(&mut self, src: &str) -> Result<(Vec<u8>, Status), Exited> {
        static CAPTURE_SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let seq = CAPTURE_SEQ.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let mut path = std::env::temp_dir();
        path.push(format!("rc-capture-{}-{:p}-{}", std::process::id(), self as *const Shell, seq));
        use std::os::unix::ffi::OsStrExt;
        let name = path.as_os_str().as_bytes().to_vec();
        let fd = unix::creat(&name);
        if fd < 0 {
            let e = unix::errno();
            self.err.pstr(&name);
            self.err.pstr(b": ");
            self.err.flush();
            self.xerror("can't open", e);
            return Ok((Vec::new(), Status(self.getstatus())));
        }
        let mut input = src.as_bytes().to_vec();
        if input.last() != Some(&b'\n') {
            input.push(b'\n');
        }
        self.eflagok = true;
        let saved = self.runq.take();
        let code = self.rdcmds_code.clone();
        self.start(code, 1, None);
        self.pushredir(ROPEN, fd, 1);
        self.q().startredir = self.q().redir.clone();
        self.q().cmdfd = Some(Rc::new(std::cell::RefCell::new(io::Io::opencore(&input))));
        let r = self.run_embedded();
        unsafe {
            libc::close(fd);
        }
        self.runq = saved;
        let out = std::fs::read(&path).unwrap_or_default();
        let _ = std::fs::remove_file(&path);
        r.map(|st| (out, st))
    }

    /// Run the commands in `file` (like `. file args...`).
    pub fn run_file(&mut self, file: &str, args: &[&str]) -> Result<Status, Exited> {
        let mut cmd = b". ".to_vec();
        let mut io = io::Io::openstr();
        io.pwrd(file.as_bytes());
        for a in args {
            io.pchr(b' ');
            io.pwrd(a.as_bytes());
        }
        cmd.extend_from_slice(io.strp());
        self.eval_bytes(&cmd)
    }

    /// The value of a variable (an empty list if unset).
    pub fn var(&mut self, name: &str) -> Vec<String> {
        self.var_bytes(name.as_bytes()).into_iter().map(|w| String::from_utf8_lossy(&w).into_owned()).collect()
    }

    pub fn var_bytes(&mut self, name: &[u8]) -> Vec<Word> {
        self.vlook(name).borrow().val.iter().cloned().collect()
    }

    /// Assign a variable.
    pub fn set_var<S: AsRef<str>>(&mut self, name: &str, value: &[S]) {
        let w: Words = value.iter().map(|s| s.as_ref().as_bytes().to_vec()).collect();
        self.setvar(name.as_bytes(), w);
    }

    /// The current `$status`.
    pub fn status(&mut self) -> Status {
        Status(self.getstatus())
    }

    /// Is the function `name` defined?
    pub fn has_fn(&mut self, name: &str) -> bool {
        self.gvlook(name.as_bytes()).borrow().func.is_some()
    }

    /// The printed definition of function `name`, if defined.
    pub fn fn_def(&mut self, name: &str) -> Option<String> {
        let v = self.gvlook(name.as_bytes());
        let vb = v.borrow();
        let (code, pc) = vb.func.as_ref()?;
        let body = code_s(code, pc - 1);
        Some(format!("fn {} {}", name, String::from_utf8_lossy(body)))
    }

    /// Is command line flag `c` set?
    pub fn flag(&self, c: u8) -> bool {
        self.flag_set(c)
    }

    /// Set or clear a command line flag (like the `flag` builtin).
    pub fn set_flag(&mut self, c: u8, on: bool) {
        if (c as usize) < NFLAG {
            self.flag[c as usize] = if on { Some(Vec::new()) } else { None };
        }
    }
}

/// The entry point of the `rc` command: a port of `main` in `exec.c`.
/// Returns only when flag parsing fails; otherwise the interpreter runs
/// until the shell exits the process.
pub fn run_main(argv: &[Word]) -> i32 {
    // The Rust runtime ignores SIGPIPE before main runs; a C program starts
    // with the default disposition, which rc's handler installation (and
    // the children that inherit dispositions) depends on.
    unsafe {
        let mut old: libc::sigaction = std::mem::zeroed();
        libc::sigaction(libc::SIGPIPE, std::ptr::null(), &mut old);
        if old.sa_sigaction == libc::SIG_IGN {
            libc::signal(libc::SIGPIPE, libc::SIG_DFL);
        }
    }
    // needed for rcmain later
    if std::env::var_os("PLAN9").is_none() {
        std::env::set_var("PLAN9", DEFAULT_PLAN9);
    }
    let mut flag: Vec<Option<Vec<Word>>> = vec![None; NFLAG];
    let argv = match getflags::getflags(argv, "DSYsrdiIlxepvVc:1m:1[command]", true, &mut flag) {
        Ok(a) => a,
        Err(e) => {
            unix::write_all(2, &e.usage(Some("[file [arg ...]]")));
            return 1; // Exit("bad flags")
        }
    };
    let argc = argv.len();
    if argv[0].first() == Some(&b'-') {
        flag[b'l' as usize] = Some(Vec::new());
    }
    if flag[b'I' as usize].is_some() {
        flag[b'i' as usize] = None;
    } else if flag[b'i' as usize].is_none() && argc == 1 && unix::isatty(0) {
        flag[b'i' as usize] = Some(Vec::new());
    }
    let mut sh = Shell::raw(ExitMode::Process, argv[0].clone());
    sh.flag = flag;
    let rcmain: Word = match sh.flag_arg(b'm') {
        Some(m) => m.clone(),
        None => {
            // rust-rc always uses its own copy of rcmain (which also reads
            // $home/lib/rcrc); -m selects another file.
            use std::os::unix::ffi::OsStrExt;
            let mut path = std::env::temp_dir();
            path.push(format!("rcmain.{}", std::process::id()));
            let name = path.as_os_str().as_bytes().to_vec();
            if std::fs::write(&path, RCMAIN).is_ok() {
                sh.rcmain_path = name.clone();
            }
            name
        }
    };
    sh.init(true, true, true);
    let bootstrap: Code = Rc::new(vec![
        Instr::I(1),
        Instr::F(Op::Mark),
        Instr::F(Op::Word),
        Instr::S(b"*".to_vec()),
        Instr::F(Op::Assign),
        Instr::F(Op::Mark),
        Instr::F(Op::Mark),
        Instr::F(Op::Word),
        Instr::S(b"*".to_vec()),
        Instr::F(Op::Dol),
        Instr::F(Op::Word),
        Instr::S(rcmain),
        Instr::F(Op::Word),
        Instr::S(b".".to_vec()),
        Instr::F(Op::Simple),
        Instr::F(Op::Exit),
        Instr::I(0),
    ]);
    sh.start(bootstrap, 1, None);
    // prime bootstrap argv
    sh.pushlist();
    for i in (1..argc).rev() {
        sh.pushword(argv[i].clone());
    }
    sh.run_loop();
    0
}
