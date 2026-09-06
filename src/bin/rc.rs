//! The `rc` command: a drop-in replacement for plan9port's `rc`.

use std::os::unix::ffi::OsStrExt;

fn main() {
    let args: Vec<Vec<u8>> = std::env::args_os().map(|a| a.as_bytes().to_vec()).collect();
    let code = rust_rc::run_main(&args);
    std::process::exit(code);
}
