# rust-rc

A faithful Rust port of the Plan 9 shell `rc`, as distributed in
[plan9port](https://github.com/9fans/plan9port).

The port follows the C sources one for one — lexer, parser, code generator,
interpreter, builtins, globbing, here documents, environment handling and
signal behaviour — so that scripts behave identically, down to error messages,
`$status` values and exit codes.  The crate provides:

* a library, so that `rc` can be embedded in other programs: create a
  `Shell`, evaluate code in it, inspect or set its variables;
* an `rc` binary that is a drop-in replacement for plan9port's `rc`
  (`rc [-srdiIlxepvV] [-c command] [-m initial] [file [arg ...]]`).

## The binary

```
cargo install --path .
rc -c 'for(i in a b c) echo $i'
```

At startup `rc` runs its embedded copy of plan9port's `rcmain` (or the file
named with `-m`), so the binary works without a plan9port installation.  The
one addition to `rcmain` is that `$home/lib/rcrc` is read at every startup —
login or not, interactive or not, including `rc -c` and scripts — after
`$home/lib/profile`, which login shells (`-l`) still read first.

## The library

```rust
use rust_rc::Shell;

let mut sh = Shell::new();
sh.eval("x=(a b c); fn twice { echo $1 $1 }").unwrap();
assert_eq!(sh.var("x"), vec!["a", "b", "c"]);

let (out, status) = sh.eval_capture("twice $x(2) | tr a-z A-Z").unwrap();
assert_eq!(out, b"B B\n");
assert!(status.is_true());

// `exit` inside evaluated code does not terminate the host process:
match sh.eval("exit 3") {
    Err(rust_rc::Exited { status }) => assert_eq!(status.exit_code(), 3),
    _ => unreachable!(),
}
```

`Shell::new()` initialises the shell the way the `rc` binary does (imports
the environment, sets `$pid`, `$path`/`$PATH`, `$home`, `$ifs`, `$prompt`,
reads `fn#` function definitions from the environment) but reads no commands
and no `rcrc`; evaluate `. $home/lib/rcrc` yourself if you want it.
`Shell::builder()` lets you turn off the signal handlers, the environment
import, the `$PLAN9` setting, or the startup prelude, and set command line
flags such as `-e` or `-x`.

Because rc is a Unix shell, pipelines, subshells, backquotes and background
commands are implemented with `fork(2)`, as in C, and the interpreter itself
keeps running in the child processes.  Evaluate rc code from a single thread,
and note that child processes exit with `_exit(2)`.  Errors in evaluated code
are reported on standard error and set `$status` to `error`, as rc does.

## Fidelity

Everything observable is meant to match plan9port's `rc`: parse trees (the
`-D` flag prints them in the same form), command output, error messages,
`$status`, exit codes, the environment passed to children (`\001`-separated
lists, `fn#name` definitions), input buffering (512-byte reads, visible when
children share the input), the here-document temporary files, `-v`/`-V`/`-x`
/`-s` tracing, prompts, and signal handling (including the `sigexit`,
`sigint`, `sighup`, `sigalrm` and — via lib9's naming of SIGTERM — `sigkill`
traps).

Known deliberate differences:

* The yacc parser (`-Y`) is not ported; plan9port's hand-written parser is
  equivalent, and `-Y` is accepted and ignored.
* When a trap function is defined and its signal arrives while rc is blocked
  reading input, the C implementation crashes; this port runs the function
  once the read completes.
* `rcmain` is always the embedded copy (`-m` overrides it) and additionally
  reads `$home/lib/rcrc` at every startup.

## Tests

`cargo test` runs the embedding tests and the differential test cases in
`tests/cases`, comparing against the outputs recorded from plan9port's `rc`
in `tests/expected`.  With a plan9port installation available:

```
PLAN9=/usr/local/plan9 tests/difftest.sh diff    # compare with $PLAN9/bin/rc
PLAN9=/usr/local/plan9 tests/difftest.sh regen   # re-record the expected outputs
```

## License

MIT, like plan9port.  rc was written by Tom Duff; the plan9port version is by
Russ Cox and others.  See `LICENSE`.
