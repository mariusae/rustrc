//! Tests of the embedding API.

use rust_rc::{Exited, Shell, Status};
use std::sync::{Mutex, MutexGuard};

/// rc waits for children with `wait(2)`, so shells in one process must not
/// run concurrently; the tests take this lock.
static LOCK: Mutex<()> = Mutex::new(());

fn lock() -> MutexGuard<'static, ()> {
    LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

#[test]
fn eval_sets_and_reads_variables() {
    let _g = lock();
    let mut sh = Shell::builder().install_signal_handlers(false).build();
    sh.eval("x=(a b c); y=$x(2)").unwrap();
    assert_eq!(sh.var("x"), vec!["a", "b", "c"]);
    assert_eq!(sh.var("y"), vec!["b"]);
    assert_eq!(sh.var("nonesuch"), Vec::<String>::new());
    sh.set_var("z", &["one", "two"]);
    assert_eq!(sh.eval("echo $#z").unwrap(), Status(b"".to_vec()));
    let (out, st) = sh.eval_capture("echo $z(2) $#z").unwrap();
    assert_eq!(out, b"two 2\n");
    assert!(st.is_true());
}

#[test]
fn status_and_exit() {
    let _g = lock();
    let mut sh = Shell::builder().install_signal_handlers(false).build();
    let st = sh.eval("false").unwrap();
    assert_eq!(st.as_bytes(), b"1");
    assert!(!st.is_true());
    assert_eq!(st.exit_code(), 1);
    assert!(sh.eval("true").unwrap().is_true());
    let st = sh.eval("~ a b").unwrap();
    assert_eq!(st.to_string(), "no match");
    match sh.eval("echo before; exit 7; echo after") {
        Err(Exited { status }) => assert_eq!(status.exit_code(), 7),
        other => panic!("expected exit, got {:?}", other),
    }
    // the shell is still usable afterwards
    assert_eq!(sh.eval_capture("echo again").unwrap().0, b"again\n");
}

#[test]
fn functions_pipes_and_subshells() {
    let _g = lock();
    let mut sh = Shell::builder().install_signal_handlers(false).build();
    sh.eval("fn double { echo $1 $1 }").unwrap();
    assert!(sh.has_fn("double"));
    assert_eq!(sh.fn_def("double").as_deref(), Some("fn double {echo $1 $1}"));
    let (out, _) = sh.eval_capture("double x | tr a-z A-Z; @{cd /; pwd}; echo `{echo bq}").unwrap();
    assert_eq!(out, b"X X\n/\nbq\n");
    sh.eval("fn double").unwrap();
    assert!(!sh.has_fn("double"));
}

#[test]
fn errors_set_status_and_do_not_terminate() {
    let _g = lock();
    let mut sh = Shell::builder().install_signal_handlers(false).build();
    let st = sh.eval("echo (a b)^(c d e); echo not reached").unwrap();
    assert_eq!(st.as_bytes(), b"error");
    let st = sh.eval("echo a )").unwrap();
    assert_eq!(st.as_bytes(), b"missing newline at end of line");
    let st = sh.eval("if (").unwrap();
    assert_eq!(st.as_bytes(), b"syntax error");
    assert_eq!(sh.eval_capture("echo fine").unwrap().0, b"fine\n");
}

#[test]
fn run_file_passes_arguments() {
    let _g = lock();
    let mut sh = Shell::builder().install_signal_handlers(false).build();
    let dir = std::env::temp_dir();
    let path = dir.join(format!("rust-rc-embed-{}.rc", std::process::id()));
    std::fs::write(&path, "echo $#* $2\nv=set\n").unwrap();
    let (out, _) = {
        let p = path.to_str().unwrap().to_string();
        let mut sh2 = Shell::builder().install_signal_handlers(false).build();
        let cmd = format!("{} one two", p);
        sh2.eval_capture(&format!(". {}", cmd)).unwrap()
    };
    assert_eq!(out, b"2 two\n");
    sh.run_file(path.to_str().unwrap(), &["a", "b c"]).unwrap();
    assert_eq!(sh.var("v"), vec!["set"]);
    std::fs::remove_file(&path).unwrap();
}

#[test]
fn flags_and_prelude() {
    let _g = lock();
    let mut sh = Shell::builder().install_signal_handlers(false).flag(b'e', None).build();
    assert!(sh.flag(b'e'));
    // -e exits on the first failing simple command
    assert!(matches!(sh.eval("false; echo no"), Err(Exited { .. })));
    sh.set_flag(b'e', false);
    assert!(sh.eval("false; echo yes").is_ok());
    // the prelude set $ifs, $prompt and $home
    assert_eq!(sh.var("ifs"), vec![" \t\n"]);
    assert_eq!(sh.var("prompt").len(), 2);
    assert!(!sh.var("home").is_empty());
}
