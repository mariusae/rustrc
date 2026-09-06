//! File name pattern matching (port of `glob.c` plus the directory helpers
//! from `plan9ish.c`).

use crate::shell::Shell;
use crate::tree::GLOB;
use crate::word::{Word, Words};

/// `deglob`: delete all the GLOB marks from `s`, in place.
pub fn deglob(s: &mut Word) {
    let mut out = Vec::with_capacity(s.len());
    let mut i = 0;
    while i < s.len() {
        if s[i] == GLOB {
            i += 1;
            if i >= s.len() {
                break;
            }
        }
        out.push(s[i]);
        i += 1;
    }
    *s = out;
}

/// Byte at `i`, or NUL past the end (C strings are NUL terminated).
#[inline]
fn at(s: &[u8], i: usize) -> u8 {
    if i < s.len() {
        s[i]
    } else {
        0
    }
}

fn twobyte(c: u8) -> bool {
    (c & 0xe0) == 0xc0
}
fn threebyte(c: u8) -> bool {
    (c & 0xf0) == 0xe0
}
fn fourbyte(c: u8) -> bool {
    (c & 0xf8) == 0xf0
}

/// `equtf`: do `p[i]` and `q[j]` point at equal utf codes?
fn equtf(p: &[u8], i: usize, q: &[u8], j: usize) -> bool {
    let c = at(p, i);
    if c != at(q, j) {
        return false;
    }
    if twobyte(c) {
        return at(p, i + 1) == at(q, j + 1);
    }
    if threebyte(c) {
        if at(p, i + 1) != at(q, j + 1) {
            return false;
        }
        if at(p, i + 1) == 0 {
            return true; // broken code at end of string!
        }
        return at(p, i + 2) == at(q, j + 2);
    }
    if fourbyte(c) {
        if at(p, i + 1) != at(q, j + 1) {
            return false;
        }
        if at(p, i + 1) == 0 {
            return true;
        }
        if at(p, i + 2) != at(q, j + 2) {
            return false;
        }
        if at(p, i + 2) == 0 {
            return true;
        }
        return at(p, i + 3) == at(q, j + 3);
    }
    true
}

/// `nextutf`: index of the next utf code in the string, not jumping past
/// nuls in broken utf codes.
fn nextutf(p: &[u8], i: usize) -> usize {
    let c = at(p, i);
    if twobyte(c) {
        return if at(p, i + 1) == 0 { i + 1 } else { i + 2 };
    }
    if threebyte(c) {
        return if at(p, i + 1) == 0 {
            i + 1
        } else if at(p, i + 2) == 0 {
            i + 2
        } else {
            i + 3
        };
    }
    if fourbyte(c) {
        return if at(p, i + 1) == 0 {
            i + 1
        } else if at(p, i + 2) == 0 {
            i + 2
        } else if at(p, i + 3) == 0 {
            i + 3
        } else {
            i + 4
        };
    }
    i + 1
}

/// `unicode`: convert the utf code at `p[i]` to a unicode value.
fn unicode(p: &[u8], i: usize) -> i32 {
    let u = at(p, i) as i32;
    if twobyte(u as u8) {
        return ((u & 0x1f) << 6) | (at(p, i + 1) as i32 & 0x3f);
    }
    if threebyte(u as u8) {
        return (u << 12) | ((at(p, i + 1) as i32 & 0x3f) << 6) | (at(p, i + 2) as i32 & 0x3f);
    }
    if fourbyte(u as u8) {
        return (u << 18)
            | ((at(p, i + 1) as i32 & 0x3f) << 12)
            | ((at(p, i + 2) as i32 & 0x3f) << 6)
            | (at(p, i + 3) as i32 & 0x3f);
    }
    u
}

/// `matchfn`: `.` and `..` are only matched by patterns starting with `.`.
fn matchfn(s: &[u8], p: &[u8]) -> bool {
    if at(s, 0) == b'.' && (at(s, 1) == 0 || (at(s, 1) == b'.' && at(s, 2) == 0)) && at(p, 0) != b'.' {
        return false;
    }
    match_(s, 0, p, 0, b'/')
}

/// `match`: does the string `s` match the pattern `p` (up to `stop`)?
/// `*` matches any sequence of characters, `?` any single character and
/// `[...]` the enclosed list of characters.
pub fn match_pat(s: &[u8], p: &[u8], stop: u8) -> bool {
    match_(s, 0, p, 0, stop)
}

fn match_(s: &[u8], mut si: usize, p: &[u8], mut pi: usize, stop: u8) -> bool {
    while at(p, pi) != stop && at(p, pi) != 0 {
        if at(p, pi) != GLOB {
            if !equtf(p, pi, s, si) {
                return false;
            }
        } else {
            pi += 1;
            match at(p, pi) {
                GLOB => {
                    if at(s, si) != GLOB {
                        return false;
                    }
                }
                b'*' => loop {
                    if match_(s, si, p, nextutf(p, pi), stop) {
                        return true;
                    }
                    if at(s, si) == 0 {
                        return false;
                    }
                    si = nextutf(s, si);
                },
                b'?' => {
                    if at(s, si) == 0 {
                        return false;
                    }
                }
                b'[' => {
                    if at(s, si) == 0 {
                        return false;
                    }
                    let c = unicode(s, si);
                    pi += 1;
                    let compl = at(p, pi) == b'~';
                    if compl {
                        pi += 1;
                    }
                    let mut hit = false;
                    while at(p, pi) != b']' {
                        if at(p, pi) == 0 {
                            return false; // syntax error
                        }
                        let lo = unicode(p, pi);
                        pi = nextutf(p, pi);
                        let hi;
                        if at(p, pi) != b'-' {
                            hi = lo;
                        } else {
                            pi += 1;
                            if at(p, pi) == 0 {
                                return false; // syntax error
                            }
                            let h = unicode(p, pi);
                            pi = nextutf(p, pi);
                            hi = h;
                        }
                        let (lo, hi) = if hi < lo { (hi, lo) } else { (lo, hi) };
                        if lo <= c && c <= hi {
                            hit = true;
                        }
                    }
                    if compl {
                        hit = !hit;
                    }
                    if !hit {
                        return false;
                    }
                }
                _ => {}
            }
        }
        si = nextutf(s, si);
        pi = nextutf(p, pi);
    }
    at(s, si) == 0
}

/// `Globsize` reduced to its observable effect: does the word contain any
/// glob metacharacter?
fn isglob(p: &[u8]) -> bool {
    let mut i = 0;
    while i < p.len() {
        if p[i] == GLOB {
            i += 1;
            if at(p, i) != GLOB {
                return true;
            }
        }
        i += 1;
    }
    false
}

/// `Opendir`/`Readdir`/`Closedir`: the names in a directory, in directory
/// order, without `.` and `..` (and without entries that cannot be
/// stat'ed, as lib9's `dirread` does).
fn readdir_names(name: &[u8]) -> Option<Vec<Word>> {
    use std::ffi::CStr;
    let cname = std::ffi::CString::new(name.to_vec()).ok()?;
    let mut names = Vec::new();
    unsafe {
        let d = libc::opendir(cname.as_ptr());
        if d.is_null() {
            return None;
        }
        let fd = libc::dirfd(d);
        loop {
            let de = libc::readdir(d);
            if de.is_null() {
                break;
            }
            let n = CStr::from_ptr((*de).d_name.as_ptr()).to_bytes();
            if n == b"." || n == b".." {
                continue;
            }
            let mut st: libc::stat = std::mem::zeroed();
            if libc::fstatat(fd, (*de).d_name.as_ptr(), &mut st, libc::AT_SYMLINK_NOFOLLOW) < 0 {
                continue;
            }
            names.push(n.to_vec());
        }
        libc::closedir(d);
    }
    Some(names)
}

struct Globber {
    globv: Vec<Word>,
}

impl Globber {
    /// `globdir`: push names prefixed by `globname` and suffixed by a match
    /// of `p` onto the list.  `namep` is the end of the prefix in
    /// `globname`.
    fn globdir(&mut self, p: &[u8], globname: &mut Word, mut namep: usize) {
        // scan the pattern looking for a component with a metacharacter in it
        if p.is_empty() {
            self.globv.push(globname.clone());
            return;
        }
        let mut t = namep;
        let mut newp = 0;
        let mut p = p;
        globname.truncate(t);
        while newp < p.len() {
            if p[newp] == GLOB {
                break;
            }
            globname.push(p[newp]);
            newp += 1;
            t += 1;
            if p[newp - 1] == b'/' {
                namep = t;
                p = &p[newp..];
                newp = 0;
            }
        }
        // If we ran out of pattern, append the name if accessible
        if newp >= p.len() {
            globname.truncate(t);
            if crate::unix::access_exists(globname) {
                self.globv.push(globname.clone());
            }
            return;
        }
        // read the directory and recur for any entry that matches
        globname.truncate(namep);
        let dirname: &[u8] = if globname.is_empty() { b"." } else { globname };
        let names = match readdir_names(dirname) {
            Some(n) => n,
            None => return,
        };
        while newp < p.len() && p[newp] != b'/' {
            newp += 1;
        }
        let comp = &p[..newp];
        let rest = &p[newp..];
        for name in names {
            if matchfn(&name, comp) {
                globname.truncate(namep);
                globname.extend_from_slice(&name);
                let end = globname.len();
                self.globdir(rest, globname, end);
            }
        }
    }

    /// `glob`: push all file names matched by `p`.  If there are no
    /// matches, the list consists of `p` itself (with GLOB marks removed).
    fn glob(&mut self, mut p: Word) {
        if !isglob(&p) {
            deglob(&mut p);
            self.globv.push(p);
            return;
        }
        let start = self.globv.len();
        let mut globname = Vec::new();
        self.globdir(&p, &mut globname, 0);
        if self.globv.len() == start {
            deglob(&mut p);
            self.globv.push(p);
        } else {
            self.globv[start..].sort();
        }
    }
}

impl Shell {
    /// `globlist`: replace the top argument list by its glob expansion.
    pub(crate) fn globlist(&mut self) {
        let words = std::mem::take(self.top());
        let mut g = Globber { globv: Vec::new() };
        for w in words {
            g.glob(w);
        }
        self.poplist();
        self.pushlist();
        let out: Words = g.globv.into_iter().collect();
        *self.top() = out;
    }
}
