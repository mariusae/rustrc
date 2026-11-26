//! A signal that comes as rc starts a command is not lost.
//!
//! rc's handler only notes a signal, for a trap run after the
//! instruction in hand. One that came while rc was starting a command
//! was noted for a trap that never ran, the exec replacing rc, and the
//! command never saw it: SIGTERM to the process group left the command
//! running. Each script here makes rc start its command slowly -- the
//! exec tried in thousands of directories that are not there before the
//! one that holds it -- and the signal is sent while it does.

use std::io::{BufRead, BufReader};
use std::os::unix::process::{CommandExt, ExitStatusExt};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// A `path` of directories that are not there, then the real ones.
/// Kept under Linux's 128K for a single environment string.
const SLOW_PATH: &str = "path=(`{seq -f /n/%g 12000} /bin /usr/bin)";

/// Run `script` (which says "ready" just before the slow command), send
/// SIGTERM to its process group once it is ready, and say how it ended,
/// or `None` if it was still running seconds later.
fn terminated(script: &str) -> Option<std::process::ExitStatus> {
    let mut child = Command::new(env!("CARGO_BIN_EXE_rc"))
        .arg("-c")
        .arg(format!("{SLOW_PATH}; /bin/echo ready; {script}"))
        .stdout(Stdio::piped())
        .process_group(0)
        .spawn()
        .unwrap();
    let mut line = String::new();
    BufReader::new(child.stdout.take().unwrap()).read_line(&mut line).unwrap();
    assert_eq!(line, "ready\n");
    // into the search, which takes a good part of a second
    std::thread::sleep(Duration::from_millis(20));
    let group = child.id() as i32;
    // SAFETY: a signal to the process group made for this child
    unsafe {
        libc::killpg(group, libc::SIGTERM);
    }
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        if let Some(status) = child.try_wait().unwrap() {
            return Some(status);
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    // SAFETY: as above
    unsafe {
        libc::killpg(group, libc::SIGKILL);
    }
    let _ = child.wait();
    None
}

#[test]
fn a_command_rc_forks_is_ended_by_a_signal_as_it_starts() {
    let status = terminated("sleep 30; /bin/echo after");
    assert!(status.is_some(), "the command ran on");
    assert!(!status.unwrap().success());
}

#[test]
fn a_command_rc_execs_is_ended_by_a_signal_as_it_starts() {
    let status = terminated("exec sleep 30");
    assert!(status.is_some(), "the command ran on");
    assert_eq!(status.unwrap().signal(), Some(libc::SIGTERM));
}

#[test]
fn a_trap_still_runs_after_an_exec_that_failed() {
    let out = Command::new(env!("CARGO_BIN_EXE_rc"))
        .arg("-c")
        .arg("fn sigkill {/bin/echo caught}; exec /nonexistent; /bin/kill -TERM $pid; /bin/echo after")
        .stderr(Stdio::null())
        .output()
        .unwrap();
    assert_eq!(String::from_utf8_lossy(&out.stdout), "caught\nafter\n");
}
