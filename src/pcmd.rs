//! Printing parse trees (port of `pcmd.c`), used for `whatis`, function
//! definitions in the environment, and the `-D` debugging flag.

use crate::io::Io;
use crate::tree::*;

fn pdeglob(f: &mut Io, s: &[u8]) {
    let mut i = 0;
    while i < s.len() {
        if s[i] == GLOB {
            i += 1;
            if i >= s.len() {
                break;
            }
        }
        f.pchr(s[i]);
        i += 1;
    }
}

fn pword(f: &mut Io, t: &Tree) {
    let s = t.s.as_deref().unwrap_or(b"");
    if t.quoted {
        f.pquo(s);
    } else {
        pdeglob(f, s);
    }
}

fn predir(f: &mut Io, t: &Tree) {
    match t.rtype {
        HERE | READ | RDWR => {
            if t.rtype == HERE {
                f.pchr(b'<');
            }
            f.pchr(b'<');
            if t.rtype == RDWR {
                f.pchr(b'>');
            }
            if t.fd0 != 0 {
                f.pchr(b'[');
                f.pdec(t.fd0 as i64);
                f.pchr(b']');
            }
        }
        APPEND | WRITE => {
            if t.rtype == APPEND {
                f.pchr(b'>');
            }
            f.pchr(b'>');
            if t.fd0 != 1 {
                f.pchr(b'[');
                f.pdec(t.fd0 as i64);
                f.pchr(b']');
            }
        }
        _ => {}
    }
}

/// `pcmd`: print a tree in a form suitable for input to rc; `nl` is the
/// separator used for `;` nodes (newline, or `;` for function bodies).
pub fn pcmd(f: &mut Io, t: Option<&Tree>, nl: u8) {
    let t = match t {
        None => return,
        Some(t) => t,
    };
    let c0 = t.child[0].as_deref();
    let c1 = t.child[1].as_deref();
    let c2 = t.child[2].as_deref();
    match t.ty {
        0x24 /* $ */ => {
            f.pchr(b'$');
            pcmd(f, c0, nl);
        }
        0x22 /* " */ => {
            f.pstr(b"$\"");
            pcmd(f, c0, nl);
        }
        0x26 /* & */ => {
            pcmd(f, c0, nl);
            f.pchr(b'&');
        }
        0x5e /* ^ */ => {
            pcmd(f, c0, nl);
            f.pchr(b'^');
            pcmd(f, c1, nl);
        }
        0x60 /* ` */ => {
            f.pchr(b'`');
            pcmd(f, c0, nl);
            pcmd(f, c1, nl);
        }
        ANDAND => {
            pcmd(f, c0, nl);
            f.pstr(b" && ");
            pcmd(f, c1, nl);
        }
        BANG => {
            f.pstr(b"! ");
            pcmd(f, c0, nl);
        }
        BRACE => {
            f.pchr(b'{');
            pcmd(f, c0, nl);
            f.pchr(b'}');
        }
        COUNT => {
            f.pstr(b"$#");
            pcmd(f, c0, nl);
        }
        FN => {
            f.pstr(b"fn ");
            pcmd(f, c0, nl);
            f.pchr(b' ');
            pcmd(f, c1, nl);
        }
        IF => {
            f.pstr(b"if");
            pcmd(f, c0, nl);
            pcmd(f, c1, nl);
        }
        NOT => {
            f.pstr(b"if not ");
            pcmd(f, c0, nl);
        }
        OROR => {
            pcmd(f, c0, nl);
            f.pstr(b" || ");
            pcmd(f, c1, nl);
        }
        PCMD | PAREN => {
            f.pchr(b'(');
            pcmd(f, c0, nl);
            f.pchr(b')');
        }
        SUB => {
            f.pchr(b'$');
            pcmd(f, c0, nl);
            f.pchr(b'(');
            pcmd(f, c1, nl);
            f.pchr(b')');
        }
        SIMPLE => {
            pcmd(f, c0, nl);
        }
        SUBSHELL => {
            f.pstr(b"@ ");
            pcmd(f, c0, nl);
        }
        SWITCH => {
            f.pstr(b"switch ");
            pcmd(f, c0, nl);
            f.pchr(b' ');
            pcmd(f, c1, nl);
        }
        TWIDDLE => {
            f.pstr(b"~ ");
            pcmd(f, c0, nl);
            f.pchr(b' ');
            pcmd(f, c1, nl);
        }
        WHILE => {
            f.pstr(b"while ");
            pcmd(f, c0, nl);
            pcmd(f, c1, nl);
        }
        ARGLIST => {
            if c0.is_none() {
                pcmd(f, c1, nl);
            } else if c1.is_none() {
                pcmd(f, c0, nl);
            } else {
                pcmd(f, c0, nl);
                f.pchr(b' ');
                pcmd(f, c1, nl);
            }
        }
        0x3b /* ; */ => {
            if c0.is_some() {
                if c1.is_some() {
                    pcmd(f, c0, nl);
                    f.pchr(nl);
                    pcmd(f, c1, nl);
                } else {
                    pcmd(f, c0, nl);
                }
            } else {
                pcmd(f, c1, nl);
            }
        }
        WORDS => {
            if c0.is_some() {
                pcmd(f, c0, nl);
                f.pchr(b' ');
            }
            pcmd(f, c1, nl);
        }
        FOR => {
            f.pstr(b"for(");
            pcmd(f, c0, nl);
            if c1.is_some() {
                f.pstr(b" in ");
                pcmd(f, c1, nl);
            }
            f.pchr(b')');
            pcmd(f, c2, nl);
        }
        WORD => {
            pword(f, t);
        }
        DUP => {
            if t.rtype == DUPFD {
                // yes, fd1, then fd0; read lex.c
                f.pstr(b">[");
                f.pdec(t.fd1 as i64);
                f.pchr(b'=');
                f.pdec(t.fd0 as i64);
                f.pchr(b']');
            } else {
                f.pstr(b">[");
                f.pdec(t.fd0 as i64);
                f.pstr(b"=]");
            }
            pcmd(f, c1, nl);
        }
        PIPEFD | REDIR => {
            predir(f, t);
            pcmd(f, c0, nl);
            if c1.is_some() {
                f.pchr(b' ');
                pcmd(f, c1, nl);
            }
        }
        0x3d /* = */ => {
            pcmd(f, c0, nl);
            f.pchr(b'=');
            pcmd(f, c1, nl);
            if c2.is_some() {
                f.pchr(b' ');
                pcmd(f, c2, nl);
            }
        }
        PIPE => {
            pcmd(f, c0, nl);
            f.pchr(b'|');
            if t.fd1 == 0 {
                if t.fd0 != 1 {
                    f.pchr(b'[');
                    f.pdec(t.fd0 as i64);
                    f.pchr(b']');
                }
            } else {
                f.pchr(b'[');
                f.pdec(t.fd0 as i64);
                f.pchr(b'=');
                f.pdec(t.fd1 as i64);
                f.pchr(b']');
            }
            pcmd(f, c1, nl);
        }
        _ => {
            f.pstr(b"bad ");
            f.pdec(t.ty as i64);
            for c in [c0, c1, c2] {
                f.pchr(b' ');
                f.pptr(c.map(|c| c as *const Tree as usize).unwrap_or(0));
            }
        }
    }
}

/// `pcmdu`: unambiguous (fully parenthesised) form, used by `-D`.
pub fn pcmdu(f: &mut Io, t: Option<&Tree>) {
    let t = match t {
        None => {
            f.pstr(b"<nil>");
            return;
        }
        Some(t) => t,
    };
    let c0 = t.child[0].as_deref();
    let c1 = t.child[1].as_deref();
    let c2 = t.child[2].as_deref();
    let two = |f: &mut Io, name: &[u8], a: Option<&Tree>, b: Option<&Tree>| {
        f.pchr(b'(');
        f.pstr(name);
        f.pchr(b' ');
        pcmdu(f, a);
        f.pchr(b' ');
        pcmdu(f, b);
        f.pchr(b')');
    };
    let one = |f: &mut Io, name: &[u8], a: Option<&Tree>| {
        f.pchr(b'(');
        f.pstr(name);
        f.pchr(b' ');
        pcmdu(f, a);
        f.pchr(b')');
    };
    match t.ty {
        0x24 => one(f, b"$", c0),
        0x22 => one(f, b"$\"", c0),
        0x26 => one(f, b"&", c0),
        0x5e => two(f, b"^", c0, c1),
        0x60 => one(f, b"`", c0),
        ANDAND => two(f, b"&&", c0, c1),
        BANG => one(f, b"!", c0),
        BRACE => one(f, b"brace", c0),
        COUNT => one(f, b"$#", c0),
        FN => two(f, b"fn", c0, c1),
        IF => two(f, b"if", c0, c1),
        NOT => one(f, b"if not", c0),
        OROR => two(f, b"||", c0, c1),
        PCMD | PAREN => one(f, b"paren", c0),
        SUB => two(f, b"$sub", c0, c1),
        SIMPLE => one(f, b"simple", c0),
        SUBSHELL => one(f, b"@", c0),
        SWITCH => two(f, b"switch", c0, c1),
        TWIDDLE => two(f, b"~", c0, c1),
        WHILE => two(f, b"while", c0, c1),
        ARGLIST => two(f, b"arglist", c0, c1),
        0x3b => two(f, b";", c0, c1),
        WORDS => two(f, b"words", c0, c1),
        FOR => {
            f.pstr(b"(for ");
            pcmdu(f, c0);
            f.pchr(b' ');
            pcmdu(f, c1);
            f.pchr(b' ');
            pcmdu(f, c2);
            f.pchr(b')');
        }
        WORD => pword(f, t),
        DUP => {
            if t.rtype == DUPFD {
                f.pstr(b"(>[");
                f.pdec(t.fd1 as i64);
                f.pchr(b'=');
                f.pdec(t.fd0 as i64);
                f.pchr(b']');
            } else {
                f.pstr(b"(>[");
                f.pdec(t.fd0 as i64);
                f.pstr(b"=]");
            }
            f.pchr(b' ');
            pcmdu(f, c1);
            f.pchr(b')');
        }
        PIPEFD | REDIR => {
            f.pchr(b'(');
            predir(f, t);
            if t.rtype == HERE {
                f.pstr(b"HERE ");
                pcmdu(f, c1);
                f.pchr(b')');
            } else {
                pcmdu(f, c0);
                f.pchr(b' ');
                pcmdu(f, c1);
                f.pchr(b')');
            }
        }
        0x3d => {
            f.pchr(b'(');
            pcmdu(f, c0);
            f.pchr(b'=');
            pcmdu(f, c1);
            f.pchr(b' ');
            pcmdu(f, c2);
            f.pchr(b')');
        }
        PIPE => {
            f.pstr(b"(|");
            if t.fd1 == 0 {
                if t.fd0 != 1 {
                    f.pchr(b'[');
                    f.pdec(t.fd0 as i64);
                    f.pchr(b']');
                }
            } else {
                f.pchr(b'[');
                f.pdec(t.fd0 as i64);
                f.pchr(b'=');
                f.pdec(t.fd1 as i64);
                f.pchr(b']');
            }
            f.pchr(b' ');
            pcmdu(f, c0);
            f.pchr(b' ');
            pcmdu(f, c1);
        }
        _ => {
            f.pstr(b"(bad ");
            f.pdec(t.ty as i64);
            for c in [c0, c1, c2] {
                f.pchr(b' ');
                f.pptr(c.map(|c| c as *const Tree as usize).unwrap_or(0));
            }
            f.pchr(b')');
        }
    }
}
