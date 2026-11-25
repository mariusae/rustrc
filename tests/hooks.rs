//! `fn prompt` and `fn preexec`, the hooks an interactive rc runs round
//! each command (not in plan9port's rc), and the prompt marks `rcmain`
//! makes of them in an apex terminal.

use std::io::Write;
use std::process::{Command, Stdio};

/// What an interactive `rc` (`-i`, input from a pipe) writes for `input`,
/// standard error (the prompts) and output together in the order
/// written; `$home` somewhere without an rcrc, `TERM_PROGRAM` as given.
fn interactive(input: &str, term_program: Option<&str>) -> String {
    // through sh, for 2>&1 on one pipe: the order kept
    let mut c = Command::new("sh");
    c.arg("-c").arg("exec \"$0\" -i 2>&1").arg(env!("CARGO_BIN_EXE_rc")).env("HOME", "/nonexistent").env_remove("TERM_PROGRAM");
    if let Some(t) = term_program {
        c.env("TERM_PROGRAM", t);
    }
    let mut child = c.stdin(Stdio::piped()).stdout(Stdio::piped()).spawn().unwrap();
    child.stdin.take().unwrap().write_all(input.as_bytes()).unwrap();
    String::from_utf8_lossy(&child.wait_with_output().unwrap().stdout).into_owned()
}

#[test]
fn prompt_runs_before_each_prompt_with_status_kept() {
    let out = interactive("fn prompt {echo P $status}\nfalse\n\necho $status\n", None);
    // after `false`: the hook sees its status, and so does the next command
    assert!(out.contains("P 1\n% P 1\n% 1\n"), "{out:?}");
}

#[test]
fn preexec_has_the_command_its_lines_and_runs_before_it() {
    let out = interactive("fn preexec {echo X $#* $\"*}\nfor(i in a){\necho $i\n}\n", None);
    assert!(out.contains("X 3 for(i in a){ echo $i }\na\n"), "{out:?}");
}

#[test]
fn preexec_is_not_run_for_nothing() {
    let out = interactive("fn preexec {echo X $\"*}\n\n# a comment\n   \necho a # and one\n", None);
    assert_eq!(out.matches("X ").count(), 1, "{out:?}");
    assert!(out.contains("X echo a # and one\na\n"), "{out:?}");
}

#[test]
fn an_error_in_a_hook_ends_the_hook_alone() {
    let out = interactive("fn preexec {nosuchcommand-rc-test}\necho still\n", None);
    assert!(out.contains("still\n"), "{out:?}");
}

#[test]
fn rcmain_marks_prompts_in_apex() {
    let out = interactive("false\necho ok; echo two\n\n", Some("apex"));
    // a prompt marked, the command with it, and its status after
    assert!(out.contains("\x1b]133;C;cmdline=false\x07"), "{out:?}");
    assert!(out.contains("\x1b]133;D;1\x07"), "{out:?}");
    assert!(out.contains("\x1b]133;C;cmdline=echo ok; echo two\x07ok\ntwo\n\x1b]133;D;0\x07"), "{out:?}");
    // an empty line ran nothing: no second end for the last command
    assert_eq!(out.matches("133;D").count(), 2, "{out:?}");
    assert!(out.matches("\x1b]133;A\x07").count() >= 3, "{out:?}");
}

#[test]
fn rcmain_marks_nothing_elsewhere() {
    let out = interactive("echo ok\n", None);
    assert!(!out.contains("133;"), "{out:?}");
}
