//! Runs the differential test cases in `tests/cases` against the expected
//! output in `tests/expected` (recorded from plan9port's rc).
//!
//! Set `RC_REF` to a reference `rc` binary and run
//! `tests/difftest.sh diff` to compare against it directly.

use std::process::Command;

#[test]
fn cases_match_reference_output() {
    let root = env!("CARGO_MANIFEST_DIR");
    let rc = env!("CARGO_BIN_EXE_rc");
    let status = Command::new("/bin/sh")
        .arg(format!("{}/tests/difftest.sh", root))
        .arg("check")
        .env("RC_OURS", rc)
        .status()
        .expect("failed to run tests/difftest.sh");
    assert!(status.success(), "differential test cases failed (see output above)");
}
