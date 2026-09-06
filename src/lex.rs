//! The lexical analyser (port of `lex.c`).

use crate::shell::Shell;
use crate::tree::*;

pub const NTOK: usize = 8192;

/// `wordchr`
pub fn wordchr(c: i32) -> bool {
    if c == EOF || c == 0 {
        return false;
    }
    if c < 0 || c > 255 {
        return true;
    }
    !b"\n \t#;&|^$=`'{}()<>".contains(&(c as u8))
}

/// `idchr`
pub fn idchr(c: i32) -> bool {
    if c <= b' ' as i32 {
        return false;
    }
    if c > 255 {
        return true;
    }
    !b"!\"#$%&'()+,-./:;<=>?@[\\]^`{|}~".contains(&(c as u8))
}

pub fn twobyte(c: i32) -> bool {
    (c & 0xe0) == 0xc0
}
pub fn threebyte(c: i32) -> bool {
    (c & 0xf0) == 0xe0
}
pub fn fourbyte(c: i32) -> bool {
    (c & 0xf8) == 0xf0
}

impl Shell {
    /// `nextc`: look ahead in the input stream.
    pub(crate) fn nextc(&mut self) -> i32 {
        if self.future == EOF {
            self.future = self.getnext();
        }
        self.future
    }

    /// `advance`: consume the lookahead character.
    pub(crate) fn advance(&mut self) -> i32 {
        let c = self.nextc();
        self.lastc = self.future;
        self.future = EOF;
        c
    }

    /// Read a raw character from the current thread's command input
    /// (`rchr(runq->cmdfd)`).
    pub(crate) fn rchr_cmd(&mut self) -> i32 {
        let f = match self.runq.as_ref().and_then(|q| q.cmdfd.clone()) {
            Some(f) => f,
            None => return EOF,
        };
        let c = {
            let mut fb = f.borrow_mut();
            if fb.fd() < 0 {
                fb.rchr_core()
            } else {
                fb.rchr_with(|fd, buf| self.read(fd, buf))
            }
        };
        c
    }

    /// `getnext`: read a character from the input stream.
    pub(crate) fn getnext(&mut self) -> i32 {
        let mut c;
        if self.peekc != EOF {
            c = self.peekc;
            self.peekc = EOF;
            return c;
        }
        if self.qref().eof != 0 {
            return EOF;
        }
        if self.doprompt {
            self.pprompt();
        }
        c = self.rchr_cmd();
        if !self.inquote && c == b'\\' as i32 {
            c = self.rchr_cmd();
            if c == b'\n' as i32 && !self.incomm {
                // don't continue a comment
                self.doprompt = true;
                c = b' ' as i32;
            } else {
                self.peekc = c;
                c = b'\\' as i32;
            }
        }
        self.doprompt = self.doprompt || c == b'\n' as i32 || c == EOF;
        if c == EOF {
            self.q().eof += 1;
        } else if self.flag_set(b'V') || (self.ndot >= 2 && self.flag_set(b'v')) {
            self.err.pchr(c as u8);
        }
        c
    }

    /// `pprompt`
    pub(crate) fn pprompt(&mut self) {
        if self.qref().iflag {
            let ps = self.promptstr.clone();
            self.err.pstr(&ps);
            self.err.flush();
            let prompt = self.vlook(b"prompt");
            let second = prompt.borrow().val.get(1).cloned();
            self.promptstr = match second {
                Some(w) => w,
                None => b"\t".to_vec(),
            };
        }
        self.q().lineno += 1;
        self.doprompt = false;
    }

    /// `skipwhite`
    fn skipwhite(&mut self) -> bool {
        let mut skipped = false;
        loop {
            let mut c = self.nextc();
            if c == b'#' as i32 {
                self.incomm = true;
                skipped = true;
                loop {
                    c = self.nextc();
                    if c == b'\n' as i32 || c == EOF {
                        self.incomm = false;
                        break;
                    }
                    self.advance();
                }
            }
            if c == b' ' as i32 || c == b'\t' as i32 {
                skipped = true;
                self.advance();
            } else {
                return skipped;
            }
        }
    }

    fn nextis(&mut self, c: i32) -> bool {
        if self.nextc() == c {
            self.advance();
            return true;
        }
        false
    }

    /// `addtok`
    fn addtok(&mut self, val: i32) {
        if self.tok_overflow {
            return;
        }
        if self.tok.len() == NTOK - 1 {
            self.tok_overflow = true;
            self.yyerror("token buffer too short");
            return;
        }
        self.tok.push(val as u8);
    }

    /// `addutf`
    fn addutf(&mut self, c: i32) {
        self.addtok(c);
        if twobyte(c) {
            let d = self.advance();
            self.addtok(d);
        } else if threebyte(c) {
            let d = self.advance();
            self.addtok(d);
            let d = self.advance();
            self.addtok(d);
        } else if fourbyte(c) {
            let d = self.advance();
            self.addtok(d);
            let d = self.advance();
            self.addtok(d);
            let d = self.advance();
            self.addtok(d);
        }
    }

    fn settok(&mut self, s: &[u8]) {
        self.tok.clear();
        self.tok.extend_from_slice(s);
        self.tok_overflow = false;
    }

    /// `yylex`: returns the token type; the token's tree (if any) is left
    /// in `self.yylval`.
    pub(crate) fn yylex(&mut self) -> i32 {
        let _d = self.nextc();
        self.yylval = None;
        // (The `lastword && flag['Y']` free-caret sneakiness only applies to
        // the yacc grammar, which this port does not use.)
        self.inquote = false;
        if self.skipwhite() {
            return b' ' as i32;
        }
        let mut c = self.advance();
        match c {
            EOF => {
                self.lastdol = false;
                self.settok(b"EOF");
                return EOF;
            }
            0x24 /* $ */ => {
                self.lastdol = true;
                if self.nextis(b'#' as i32) {
                    self.settok(b"$#");
                    return COUNT;
                }
                if self.nextis(b'"' as i32) {
                    self.settok(b"$\"");
                    return b'"' as i32;
                }
                self.settok(b"$");
                return b'$' as i32;
            }
            0x26 /* & */ => {
                self.lastdol = false;
                if self.nextis(b'&' as i32) {
                    self.settok(b"&&");
                    return ANDAND;
                }
                self.settok(b"&");
                return b'&' as i32;
            }
            _ => {}
        }
        if c == b'|' as i32 {
            self.lastdol = false;
            if self.nextis(c) {
                self.settok(b"||");
                return OROR;
            }
        }
        if c == b'|' as i32 || c == b'<' as i32 || c == b'>' as i32 {
            self.lastdol = false;
            /*
             * funny redirection tokens:
             *	redir:	arrow | arrow '[' fd ']'
             *	arrow:	'<' | '<<' | '>' | '>>' | '|'
             *	fd:	digit | digit '=' | digit '=' digit
             *	digit:	'0'|'1'|'2'|'3'|'4'|'5'|'6'|'7'|'8'|'9'
             * some possibilities are nonsensical and get a message.
             */
            self.settok(&[c as u8]);
            let mut t = newtree();
            match c as u8 {
                b'|' => {
                    t.ty = PIPE;
                    t.fd0 = 1;
                    t.fd1 = 0;
                }
                b'>' => {
                    t.ty = REDIR;
                    if self.nextis(c) {
                        t.rtype = APPEND;
                        self.tok.push(c as u8);
                    } else {
                        t.rtype = WRITE;
                    }
                    t.fd0 = 1;
                }
                _ => {
                    t.ty = REDIR;
                    if self.nextis(c) {
                        t.rtype = HERE;
                        self.tok.push(c as u8);
                    } else if self.nextis(b'>' as i32) {
                        t.rtype = RDWR;
                        self.tok.push(b'>');
                    } else {
                        t.rtype = READ;
                    }
                    t.fd0 = 0;
                }
            }
            if self.nextis(b'[' as i32) {
                self.tok.push(b'[');
                c = self.advance();
                // (the C code stores this character and then stores the
                // first digit again inside the loop below; the doubled
                // digit is visible in error messages, so it is kept)
                self.tok.push(c as u8);
                let redir_err = 'redir: {
                    if c < b'0' as i32 || (b'9' as i32) < c {
                        break 'redir true;
                    }
                    t.fd0 = 0;
                    loop {
                        t.fd0 = t.fd0.wrapping_mul(10).wrapping_add(c - b'0' as i32);
                        self.tok.push(c as u8);
                        c = self.advance();
                        if !(b'0' as i32 <= c && c <= b'9' as i32) {
                            break;
                        }
                    }
                    if c == b'=' as i32 {
                        self.tok.push(b'=');
                        if t.ty == REDIR {
                            t.ty = DUP;
                        }
                        c = self.advance();
                        if b'0' as i32 <= c && c <= b'9' as i32 {
                            t.rtype = DUPFD;
                            t.fd1 = t.fd0;
                            t.fd0 = 0;
                            loop {
                                t.fd0 = t.fd0.wrapping_mul(10).wrapping_add(c - b'0' as i32);
                                self.tok.push(c as u8);
                                c = self.advance();
                                if !(b'0' as i32 <= c && c <= b'9' as i32) {
                                    break;
                                }
                            }
                        } else {
                            if t.ty == PIPE {
                                break 'redir true;
                            }
                            t.rtype = CLOSE;
                        }
                    }
                    if c != b']' as i32 || (t.ty == DUP && (t.rtype == HERE || t.rtype == APPEND)) {
                        break 'redir true;
                    }
                    false
                };
                if redir_err {
                    let msg = if t.ty == PIPE { "pipe syntax" } else { "redirection syntax" };
                    self.yyerror(msg);
                    return EOF;
                }
                self.tok.push(b']');
            }
            let ty = t.ty;
            let mut t = t;
            if ty == REDIR {
                self.skipwhite();
                if self.nextc() == b'{' as i32 {
                    t.ty = REDIRW;
                }
            }
            let ty = t.ty;
            self.yylval = Some(t);
            return ty;
        }
        if c == b'\'' as i32 {
            self.lastdol = false;
            self.lastword = true;
            self.inquote = true;
            self.tok.clear();
            self.tok_overflow = false;
            loop {
                c = self.advance();
                if c == EOF {
                    break;
                }
                if c == b'\'' as i32 {
                    if self.nextc() != b'\'' as i32 {
                        break;
                    }
                    self.advance();
                }
                self.addutf(c);
            }
            let mut t = token(&self.tok, WORD);
            t.quoted = true;
            self.yylval = Some(t);
            return WORD;
        }
        if !wordchr(c) {
            self.lastdol = false;
            self.settok(&[c as u8]);
            return c;
        }
        self.tok.clear();
        self.tok_overflow = false;
        loop {
            if c == b'*' as i32 || c == b'[' as i32 || c == b'?' as i32 || c == GLOB as i32 {
                self.addtok(GLOB as i32);
            }
            self.addutf(c);
            c = self.nextc();
            let stop = if self.lastdol { !idchr(c) } else { !wordchr(c) };
            if stop {
                break;
            }
            self.advance();
        }
        self.lastword = true;
        self.lastdol = false;
        let mut t = klook(&self.tok);
        if t.ty != WORD {
            self.lastword = false;
        }
        t.quoted = false;
        let ty = t.ty;
        self.yylval = Some(t);
        ty
    }

    /// `yyerror` (from `subr.c`).
    pub(crate) fn yyerror(&mut self, m: &str) {
        self.err.pstr(b"rc: ");
        let (cmdfile, iflag, lineno) = {
            let q = self.qref();
            (q.cmdfile.clone(), q.iflag, q.lineno)
        };
        if let Some(f) = &cmdfile {
            self.err.pstr(f);
            if !iflag {
                self.err.pchr(b':');
                self.err.pdec(lineno as i64);
            }
            self.err.pstr(b": ");
        } else if !iflag {
            self.err.pstr(b"line ");
            self.err.pdec(lineno as i64);
            self.err.pstr(b": ");
        }
        if !self.tok.is_empty() && self.tok[0] != b'\n' {
            self.err.pstr(b"token ");
            let tok = self.tok.clone();
            self.err.pwrd(&tok);
            self.err.pstr(b": ");
        }
        self.err.pstr(m.as_bytes());
        self.err.pchr(b'\n');
        self.err.flush();
        self.lastword = false;
        self.lastdol = false;
        while self.lastc != b'\n' as i32 && self.lastc != EOF {
            self.advance();
        }
        self.nerror += 1;
        let mut w = crate::word::Words::new();
        w.push_back(m.as_bytes().to_vec());
        self.setvar(b"status", w);
    }
}
