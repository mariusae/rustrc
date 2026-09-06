//! Here documents (port of `here.c`).

use crate::io::Io;
use crate::shell::{Here, Shell};
use crate::tree::{token, Tree, WORD};
use crate::word::Words;

const NLINE: usize = 4096;
const HEX: &[u8; 16] = b"0123456789abcdef";

fn hexnum(p: &mut [u8], n: i32) {
    p[0] = HEX[((n >> 12) & 0xF) as usize];
    p[1] = HEX[((n >> 8) & 0xF) as usize];
    p[2] = HEX[((n >> 4) & 0xF) as usize];
    p[3] = HEX[(n & 0xF) as usize];
}

impl Shell {
    /// `heredoc`: register a here document for `tag` and return the word
    /// naming the temporary file that will hold its contents.
    pub(crate) fn heredoc(&mut self, tag: Box<Tree>) -> Box<Tree> {
        if tag.ty != WORD {
            self.yyerror("Bad here tag");
        }
        let mut tmp = b"/tmp/here0000.0000".to_vec();
        hexnum(&mut tmp[9..13], unsafe { libc::getpid() });
        hexnum(&mut tmp[14..18], self.here_ser);
        self.here_ser += 1;
        self.here.push(Here { tag, name: tmp.clone() });
        token(&tmp, WORD)
    }

    /// `readhere`: read the bodies of the pending here documents from the
    /// command input into their temporary files.
    ///
    /// bug (preserved): lines longer than NLINE get split -- this can cause
    /// spurious missubstitution, or a misrecognized EOF marker.
    pub(crate) fn readhere(&mut self) {
        let heres = std::mem::take(&mut self.here);
        for h in heres {
            let subst = !h.tag.quoted;
            let tag = h.tag.s.clone();
            let c = crate::unix::creat(&h.name);
            if c < 0 {
                self.yyerror("can't create here document");
            }
            let mut f = Io::openfd(c);
            let mut line: Vec<u8> = Vec::new();
            self.pprompt();
            loop {
                let c = self.rchr_cmd();
                if c == crate::io::EOF {
                    break;
                }
                if c == b'\n' as i32 || line.len() == NLINE {
                    if tag.as_deref() == Some(line.as_slice()) {
                        break;
                    }
                    if subst {
                        self.psubst(&mut f, &line);
                    } else {
                        f.pstr(&line);
                    }
                    line.clear();
                    if c == b'\n' as i32 {
                        self.pprompt();
                        f.pchr(c as u8);
                    } else {
                        line.push(c as u8);
                    }
                } else {
                    line.push(c as u8);
                }
            }
            f.flush();
            f.close();
            self.cleanhere(&h.name);
        }
        self.doprompt = true;
    }

    /// `psubst`: copy a here document line, substituting `$var`.
    fn psubst(&mut self, f: &mut Io, s: &[u8]) {
        let mut i = 0;
        let n = s.len();
        while i < n {
            if s[i] != b'$' {
                let c = s[i];
                if (0xa0..=0xf5).contains(&c) {
                    f.pchr(s[i]);
                    i += 1;
                    if i >= n {
                        break;
                    }
                } else if (0xf6..=0xf7).contains(&c) {
                    f.pchr(s[i]);
                    i += 1;
                    if i >= n {
                        break;
                    }
                    f.pchr(s[i]);
                    i += 1;
                    if i >= n {
                        break;
                    }
                }
                f.pchr(s[i]);
                i += 1;
            } else {
                i += 1;
                let mut t = i;
                if t < n && s[t] == b'$' {
                    f.pchr(s[t]);
                    t += 1;
                } else {
                    // idchr is applied to a (signed) char here
                    while t < n && crate::lex::idchr(s[t] as i8 as i32) {
                        t += 1;
                    }
                    let name = &s[i..t];
                    let mut num: i32 = 0;
                    let mut u = 0;
                    while u < name.len() && name[u].is_ascii_digit() {
                        num = num.wrapping_mul(10).wrapping_add((name[u] - b'0') as i32);
                        u += 1;
                    }
                    if num != 0 && u == name.len() {
                        let star = self.vlook(b"*");
                        let star = star.borrow();
                        if 1 <= num && (num as usize) <= star.val.len() {
                            f.pstr(&star.val[num as usize - 1]);
                        }
                    } else {
                        let v = self.vlook(name);
                        let v = v.borrow();
                        pstrs(f, &v.val);
                    }
                    if t < n && s[t] == b'^' {
                        t += 1;
                    }
                }
                i = t;
            }
        }
    }
}

fn pstrs(f: &mut Io, a: &Words) {
    for (i, w) in a.iter().enumerate() {
        if i != 0 {
            f.pchr(b' ');
        }
        f.pstr(w);
    }
}
