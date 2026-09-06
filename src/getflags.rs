//! Command line flag parsing (port of `getflags.c`).

use crate::shell::NFLAG;
use crate::word::Word;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Reason {
    Reset,
    FewArgs,
    FlagSyn,
    BadFlag,
}

/// The result of a failed `getflags` call, from which the usage message is
/// built.
#[derive(Debug, Clone)]
pub struct FlagError {
    reason: Reason,
    badflag: u8,
    cmdname: Word,
    flagarg: Vec<u8>,
}

impl FlagError {
    /// `usage`: the text printed on standard error.
    pub fn usage(&self, tail: Option<&str>) -> Vec<u8> {
        let mut out = Vec::new();
        match self.reason {
            Reason::Reset => {
                out.extend_from_slice(b"Flag -");
                out.push(self.badflag);
                out.extend_from_slice(b": set twice\n");
            }
            Reason::FewArgs => {
                out.extend_from_slice(b"Flag -");
                out.push(self.badflag);
                out.extend_from_slice(b": too few arguments\n");
            }
            Reason::FlagSyn => {
                out.extend_from_slice(b"Bad argument to getflags!\n");
            }
            Reason::BadFlag => {
                out.extend_from_slice(b"Illegal flag -");
                out.push(self.badflag);
                out.push(b'\n');
            }
        }
        out.extend_from_slice(b"Usage: ");
        out.extend_from_slice(&self.cmdname);
        let s = &self.flagarg;
        let mut nflag = 0;
        let mut i = 0;
        while i < s.len() {
            let c = s[i];
            i += 1;
            if c == b' ' {
                continue;
            }
            let mut count = 0;
            if i < s.len() && s[i] == b':' {
                i += 1;
                while i < s.len() && s[i].is_ascii_digit() {
                    count = count * 10 + (s[i] - b'0') as usize;
                    i += 1;
                }
            }
            if count == 0 {
                if nflag == 0 {
                    out.extend_from_slice(b" [-");
                }
                nflag += 1;
                out.push(c);
            }
            if i < s.len() && s[i] == b'[' {
                i += 1;
                while i < s.len() && s[i] != b']' {
                    i += 1;
                }
                if i < s.len() && s[i] == b']' {
                    i += 1;
                }
            }
        }
        if nflag != 0 {
            out.push(b']');
        }
        let mut i = 0;
        while i < s.len() {
            let c = s[i];
            i += 1;
            if c == b' ' {
                continue;
            }
            let mut count = 0;
            if i < s.len() && s[i] == b':' {
                i += 1;
                while i < s.len() && s[i].is_ascii_digit() {
                    count = count * 10 + (s[i] - b'0') as usize;
                    i += 1;
                }
            }
            if count != 0 {
                out.extend_from_slice(b" [-");
                out.push(c);
                if i < s.len() && s[i] == b'[' {
                    i += 1;
                    let t = i;
                    while i < s.len() && s[i] != b']' {
                        i += 1;
                    }
                    out.push(b' ');
                    out.extend_from_slice(&s[t..i]);
                    if i < s.len() && s[i] == b']' {
                        i += 1;
                    }
                } else {
                    for _ in 0..count {
                        out.extend_from_slice(b" arg");
                    }
                }
                out.push(b']');
            } else if i < s.len() && s[i] == b'[' {
                i += 1;
                while i < s.len() && s[i] != b']' {
                    i += 1;
                }
                if i < s.len() && s[i] == b']' {
                    i += 1;
                }
            }
        }
        if let Some(tail) = tail {
            out.push(b' ');
            out.extend_from_slice(tail.as_bytes());
        }
        out.push(b'\n');
        out
    }
}

/// `scanflag`: the number of arguments taken by flag `c`, or an error.
fn scanflag(c: u8, f: &[u8]) -> Result<usize, Reason> {
    let mut i = 0;
    if (c as usize) < NFLAG {
        while i < f.len() {
            if f[i] == b' ' {
                i += 1;
                continue;
            }
            let fc = f[i];
            i += 1;
            let mut count = 0;
            if i < f.len() && f[i] == b':' {
                i += 1;
                if i >= f.len() || !f[i].is_ascii_digit() {
                    return Err(Reason::FlagSyn);
                }
                while i < f.len() && f[i].is_ascii_digit() {
                    count = count * 10 + (f[i] - b'0') as usize;
                    i += 1;
                }
            }
            if i < f.len() && f[i] == b'[' {
                loop {
                    i += 1;
                    if i >= f.len() {
                        return Err(Reason::FlagSyn);
                    }
                    if f[i] == b']' {
                        break;
                    }
                }
                i += 1;
            }
            if c == fc {
                return Ok(count);
            }
        }
    }
    Err(Reason::BadFlag)
}

/// `getflags`: parse the flags described by `flags` out of `argv`
/// (`argv[0]` is the command name).  On success returns the remaining
/// arguments (still starting with the command name); `flag` receives the
/// values.  With `stop` set, parsing stops at the first non-flag argument.
pub fn getflags(
    argv: &[Word],
    flags: &str,
    stop: bool,
    flag: &mut Vec<Option<Vec<Word>>>,
) -> Result<Vec<Word>, FlagError> {
    let mut argv: Vec<Word> = argv.to_vec();
    let cmdname = argv.first().cloned().unwrap_or_default();
    let mkerr = |reason: Reason, badflag: u8| FlagError {
        reason,
        badflag,
        cmdname: cmdname.clone(),
        flagarg: flags.as_bytes().to_vec(),
    };
    let mut i = 1;
    while i < argv.len() {
        if argv[i].first() != Some(&b'-') || argv[i].len() == 1 {
            if stop {
                return Ok(argv);
            }
            i += 1;
            continue;
        }
        let s: Vec<u8> = argv[i][1..].to_vec();
        let mut si = 0;
        while si < s.len() {
            let c = s[si];
            si += 1;
            let count = match scanflag(c, flags.as_bytes()) {
                Ok(n) => n,
                Err(r) => return Err(mkerr(r, c)),
            };
            if (c as usize) < NFLAG && flag[c as usize].is_some() {
                return Err(mkerr(Reason::Reset, c));
            }
            if count == 0 {
                flag[c as usize] = Some(Vec::new());
                if si == s.len() {
                    argv.remove(i);
                }
            } else {
                let rest: Vec<u8>;
                if si == s.len() {
                    argv.remove(i);
                    rest = argv.get(i).cloned().unwrap_or_default();
                } else {
                    rest = s[si..].to_vec();
                }
                if argv.len() - i < count {
                    return Err(mkerr(Reason::FewArgs, c));
                }
                // The C code rotates the flag's arguments to the end of
                // argv (past the returned argc); here they are simply
                // removed and recorded.
                let mut vals: Vec<Word> = argv.drain(i..i + count).collect();
                vals[0] = rest;
                flag[c as usize] = Some(vals);
                si = s.len();
            }
        }
    }
    Ok(argv)
}
