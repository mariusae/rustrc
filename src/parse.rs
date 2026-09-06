//! The recursive-descent parser (port of `parse.c`).
//!
//! `parse.c` replaced the yacc grammar in `syn.y`; the two are equivalent
//! (plan9port's `checkparse` script verifies this) and only the hand
//! written parser is ported.

use crate::shell::Shell;
use crate::tree::*;

/// The `longjmp(yyjmp, 1)` of `syntax()`.
#[derive(Debug)]
pub struct Syntax;

type R = Result<(T, i32), Syntax>;

const SP: i32 = b' ' as i32;
const NL: i32 = b'\n' as i32;

fn iswordtok(tok: i32) -> bool {
    matches!(tok, FOR | IN | WHILE | IF | NOT | TWIDDLE | BANG | SUBSHELL | SWITCH | FN | COUNT | WORD | REDIRW)
        || tok == b'$' as i32
        || tok == b'"' as i32
        || tok == b'`' as i32
        || tok == b'(' as i32
        || tok == b'=' as i32
}

impl Shell {
    fn dropnl(&mut self, mut tok: i32) -> i32 {
        while tok == SP || tok == NL {
            tok = self.yylex();
        }
        tok
    }

    fn dropsp(&mut self, mut tok: i32) -> i32 {
        while tok == SP {
            tok = self.yylex();
        }
        tok
    }

    fn syntax(&mut self, _tok: i32) -> Syntax {
        self.yyerror("syntax error");
        Syntax
    }

    /// `parse`: read and compile one command line.  Returns `true` on
    /// end of file or error (like the C function's nonzero result).
    pub(crate) fn parse(&mut self) -> bool {
        let tok = self.yylex();
        let mut tok = self.dropsp(tok);
        if tok == EOF {
            return true;
        }
        let t = match self.line(tok) {
            Ok((t, tk)) => {
                tok = tk;
                t
            }
            Err(Syntax) => return true,
        };
        if tok != NL {
            self.yyerror("missing newline at end of line");
        }
        !self.compile(t.as_deref())
    }

    fn line(&mut self, tok: i32) -> R {
        self.cmds(tok, false)
    }

    fn body(&mut self, tok: i32) -> R {
        self.cmds(tok, true)
    }

    fn cmds(&mut self, mut tok: i32, nlok: bool) -> R {
        let mut items: Vec<Box<Tree>> = Vec::new();
        loop {
            let (mut t2, tk) = self.cmd(tok)?;
            tok = tk;
            if tok == b'&' as i32 {
                t2 = tree1(b'&' as i32, t2);
            }
            if let Some(t2) = t2 {
                items.push(t2);
            }
            if tok != b';' as i32 && tok != b'&' as i32 && (!nlok || tok != NL) {
                break;
            }
            tok = self.yylex();
        }
        // Build the right-nested `;' list the C code builds incrementally.
        let mut acc: T = None;
        while let Some(x) = items.pop() {
            acc = match acc {
                None => Some(x),
                Some(a) => tree2(b';' as i32, Some(x), Some(a)),
            };
        }
        Ok((acc, tok))
    }

    fn brace(&mut self, tok: i32) -> Result<Box<Tree>, Syntax> {
        let tok = self.dropsp(tok);
        if tok != b'{' as i32 {
            return Err(self.syntax(tok));
        }
        let tk = self.yylex();
        let (t, tok) = self.body(tk)?;
        if tok != b'}' as i32 {
            return Err(self.syntax(tok));
        }
        Ok(tree1(BRACE, t).unwrap())
    }

    fn paren(&mut self, tok: i32) -> Result<Box<Tree>, Syntax> {
        let tok = self.dropsp(tok);
        if tok != b'(' as i32 {
            return Err(self.syntax(tok));
        }
        let tk = self.yylex();
        let (t, tok) = self.body(tk)?;
        if tok != b')' as i32 {
            return Err(self.syntax(tok));
        }
        Ok(tree1(PCMD, t).unwrap())
    }

    fn epilog(&mut self, tok: i32) -> R {
        if tok != REDIR && tok != DUP {
            return Ok((None, tok));
        }
        let (r, tok) = self.yyredir(tok)?;
        let (t, tok) = self.epilog(tok)?;
        let mut r = r;
        r.child[1] = t;
        Ok((Some(r), tok))
    }

    fn yyredir(&mut self, tok: i32) -> Result<(Box<Tree>, i32), Syntax> {
        match tok {
            DUP => {
                let r = self.yylval.take().expect("rc: DUP token without tree");
                let tk = self.yylex();
                let tok = self.dropsp(tk);
                Ok((r, tok))
            }
            REDIR => {
                let r = self.yylval.take().expect("rc: REDIR token without tree");
                let tk = self.yylex();
                let (w, tok) = self.yyword(tk, true)?;
                let tok = self.dropsp(tok);
                let w = if r.rtype == HERE { Some(self.heredoc(w.expect("rc: redirection without word"))) } else { w };
                Ok((mung1(r, w), tok))
            }
            _ => Err(self.syntax(tok)),
        }
    }

    fn cmd(&mut self, tok: i32) -> R {
        let tok = self.dropsp(tok);
        let (mut t1, mut tok) = self.cmd2(tok)?;
        while tok == ANDAND || tok == OROR {
            let op = tok;
            let tk = self.yylex();
            let tk = self.dropnl(tk);
            let (t2, tk) = self.cmd2(tk)?;
            tok = tk;
            t1 = tree2(op, t1, t2);
        }
        Ok((t1, tok))
    }

    fn cmd2(&mut self, tok: i32) -> R {
        let (mut t1, mut tok) = self.cmd3(tok)?;
        while tok == PIPE {
            let t2 = self.yylval.take().expect("rc: PIPE token without tree");
            let tk = self.yylex();
            let tk = self.dropnl(tk);
            let (t3, tk) = self.cmd3(tk)?;
            tok = tk;
            t1 = Some(mung2(t2, t1, t3));
        }
        Ok((t1, tok))
    }

    fn cmd3(&mut self, tok: i32) -> R {
        let tok = self.dropsp(tok);
        if tok == b';' as i32 || tok == b'&' as i32 || tok == NL {
            return Ok((None, tok));
        }
        match tok {
            IF => {
                let t1 = self.yylval.take().expect("rc: IF without tree");
                let tk = self.yylex();
                let tok = self.dropsp(tk);
                if tok == NOT {
                    let t1 = self.yylval.take().expect("rc: NOT without tree");
                    let tk = self.yylex();
                    let tk = self.dropnl(tk);
                    let (t2, tok) = self.cmd(tk)?;
                    return Ok((Some(mung1(t1, t2)), tok));
                }
                let t2 = self.paren(tok)?;
                let tk = self.yylex();
                let tk = self.dropnl(tk);
                let (t3, tok) = self.cmd(tk)?;
                Ok((Some(mung2(t1, Some(t2), t3)), tok))
            }
            FOR => {
                let t1 = self.yylval.take().expect("rc: FOR without tree");
                let tk = self.yylex();
                let tok = self.dropsp(tk);
                if tok != b'(' as i32 {
                    return Err(self.syntax(tok));
                }
                let tk = self.yylex();
                let (t2, tok) = self.yyword(tk, true)?;
                let t3;
                let tok = if tok == b')' as i32 {
                    t3 = None;
                    tok
                } else if tok == IN {
                    let tk = self.yylex();
                    let (w, tok) = self.words(tk)?;
                    t3 = match w {
                        None => tree1(PAREN, None),
                        w => w,
                    };
                    if tok != b')' as i32 {
                        return Err(self.syntax(tok));
                    }
                    tok
                } else {
                    return Err(self.syntax(tok));
                };
                let _ = tok;
                let tk = self.yylex();
                let tk = self.dropnl(tk);
                let (t4, tok) = self.cmd(tk)?;
                Ok((Some(mung3(t1, t2, t3, t4)), tok))
            }
            WHILE => {
                let t1 = self.yylval.take().expect("rc: WHILE without tree");
                let tk = self.yylex();
                let t2 = self.paren(tk)?;
                let tk = self.yylex();
                let tk = self.dropnl(tk);
                let (t3, tok) = self.cmd(tk)?;
                Ok((Some(mung2(t1, Some(t2), t3)), tok))
            }
            SWITCH => {
                let tk = self.yylex();
                let (t1, tok) = self.yyword(tk, true)?;
                let tok = self.dropnl(tok); // doesn't work in yacc grammar but works here!
                let t2 = self.brace(tok)?;
                let tk = self.yylex();
                let tok = self.dropsp(tk);
                Ok((tree2(SWITCH, t1, Some(t2)), tok))
            }
            FN => {
                let tk = self.yylex();
                let (t1, tok) = self.words(tk)?;
                if tok != b'{' as i32 {
                    return Ok((tree1(FN, t1), tok));
                }
                let t2 = self.brace(tok)?;
                let tk = self.yylex();
                let tok = self.dropsp(tk);
                Ok((tree2(FN, t1, Some(t2)), tok))
            }
            TWIDDLE => {
                let t1 = self.yylval.take().expect("rc: TWIDDLE without tree");
                let tk = self.yylex();
                let (t2, tok) = self.yyword(tk, true)?;
                let (t3, tok) = self.words(tok)?;
                Ok((Some(mung2(t1, t2, t3)), tok))
            }
            BANG | SUBSHELL => {
                // Note: cmd2: ! x | y is !{x | y} not {!x} | y.
                let t1 = self.yylval.take().expect("rc: BANG/SUBSHELL without tree");
                let tk = self.yylex();
                let (t2, tok) = self.cmd2(tk)?;
                Ok((Some(mung1(t1, t2)), tok))
            }
            REDIR | DUP => {
                // Note: cmd2: {>x echo a | tr a-z A-Z} writes A to x.
                let (mut t1, tok) = self.yyredir(tok)?;
                let (t2, tok) = self.cmd2(tok)?;
                t1.child[1] = t2;
                Ok((Some(t1), tok))
            }
            _ if tok == b'{' as i32 => {
                let t1 = self.brace(tok)?;
                let tk = self.yylex();
                let tok = self.dropsp(tk);
                let (t2, tok) = self.epilog(tok)?;
                Ok((Some(epimung(t1, t2)), tok))
            }
            _ => {
                if !iswordtok(tok) {
                    return Ok((None, tok));
                }
                // Note: first is same as word except for disallowing all the
                // leading keywords, but all those keywords have been picked
                // off in the switch above.
                let (t1, tok) = self.yyword(tok, false)?;
                if tok == b'=' as i32 {
                    // assignment
                    // Note: cmd2: {x=1 true | echo $x} echoes 1.
                    let tk = self.yylex();
                    let (w, tok) = self.yyword(tk, true)?;
                    let mut t1 = tree2(b'=' as i32, t1, w).unwrap();
                    let (t2, tok) = self.cmd2(tok)?;
                    t1.child[2] = t2;
                    return Ok((Some(t1), tok));
                }
                let mut t1 = t1;
                let mut tok = tok;
                loop {
                    if tok == REDIR || tok == DUP {
                        let (r, tk) = self.yyredir(tok)?;
                        tok = tk;
                        t1 = tree2(ARGLIST, t1, Some(r));
                    } else if iswordtok(tok) {
                        let (w, tk) = self.yyword(tok, true)?;
                        tok = tk;
                        t1 = tree2(ARGLIST, t1, w);
                    } else {
                        break;
                    }
                }
                Ok((Some(simplemung(t1.expect("rc: empty simple command"))), tok))
            }
        }
    }

    fn words(&mut self, tok: i32) -> R {
        let mut t: T = None;
        let mut tok = self.dropsp(tok);
        while iswordtok(tok) {
            let (w, tk) = self.yyword(tok, true)?;
            tok = tk;
            t = tree2(WORDS, t, w);
        }
        Ok((t, tok))
    }

    fn yyword(&mut self, tok: i32, eqok: bool) -> R {
        let (mut t, mut tok) = self.word1(tok)?;
        if !(tok == b'=' as i32 && !eqok) {
            loop {
                if iswordtok(tok) {
                    // No free carats around parens.
                    if t.as_ref().map(|t| t.ty == PAREN).unwrap_or(false) || tok == b'(' as i32 {
                        return Err(self.syntax(tok));
                    }
                    let (w, tk) = self.word1(tok)?;
                    tok = tk;
                    t = tree2(b'^' as i32, t, w);
                    continue;
                }
                tok = self.dropsp(tok);
                if tok == b'^' as i32 {
                    let tk = self.yylex();
                    let (w, tk) = self.word1(tk)?;
                    tok = tk;
                    t = tree2(b'^' as i32, t, w);
                    continue;
                }
                break;
            }
        }
        let tok = self.dropsp(tok);
        Ok((t, tok))
    }

    fn word1(&mut self, tok: i32) -> R {
        let tok = self.dropsp(tok);
        match tok {
            WORD | FOR | IN | WHILE | IF | NOT | TWIDDLE | BANG | SUBSHELL | SWITCH | FN => {
                let mut t = self.yylval.take().expect("rc: word token without tree");
                t.ty = WORD;
                let tok = self.yylex();
                Ok((Some(t), tok))
            }
            _ if tok == b'=' as i32 => {
                let tok = self.yylex();
                Ok((Some(token(b"=", WORD)), tok))
            }
            _ if tok == b'$' as i32 => {
                let tk = self.yylex();
                let (w, tok) = self.word1(tk)?;
                if tok == b'(' as i32 {
                    let tk = self.yylex();
                    let (sub, tok) = self.words(tk)?;
                    if tok != b')' as i32 {
                        return Err(self.syntax(tok));
                    }
                    let tok = self.yylex();
                    return Ok((tree2(SUB, w, sub), tok));
                }
                Ok((tree1(b'$' as i32, w), tok))
            }
            _ if tok == b'"' as i32 => {
                let tk = self.yylex();
                let (w, tok) = self.word1(tk)?;
                Ok((tree1(b'"' as i32, w), tok))
            }
            COUNT => {
                let tk = self.yylex();
                let (w, tok) = self.word1(tk)?;
                Ok((tree1(COUNT, w), tok))
            }
            _ if tok == b'`' as i32 => {
                let mut w: T = None;
                let tk = self.yylex();
                let mut tok = self.dropsp(tk);
                if iswordtok(tok) {
                    let (ww, tk) = self.yyword(tok, true)?;
                    w = ww;
                    tok = tk;
                }
                let b = self.brace(tok)?;
                let t = tree2(b'`' as i32, w, Some(b));
                let tok = self.yylex();
                Ok((t, tok))
            }
            _ if tok == b'(' as i32 => {
                let tk = self.yylex();
                let (ws, tok) = self.words(tk)?;
                let t = tree1(PAREN, ws);
                if tok != b')' as i32 {
                    return Err(self.syntax(tok));
                }
                let tok = self.yylex();
                Ok((t, tok))
            }
            REDIRW => {
                let t = self.yylval.take().expect("rc: REDIRW without tree");
                let tk = self.yylex();
                let b = self.brace(tk)?;
                let mut t = mung1(t, Some(b));
                t.ty = PIPEFD;
                let tok = self.yylex();
                Ok((Some(t), tok))
            }
            _ => Err(self.syntax(tok)),
        }
    }
}
