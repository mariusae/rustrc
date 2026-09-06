//! Parse tree nodes and token types (port of `rc.h`/`tree.c`).
//!
//! Token type numbers follow the values Plan 9 yacc would assign to the
//! `%term` declarations in `syn.y` (starting at 57346), so that any debug
//! output that prints raw type numbers matches the C implementation.
//! Single-character tokens use their byte value.

use crate::word::Word;

pub const EOF: i32 = -1;

pub const FOR: i32 = 57346;
pub const IN: i32 = 57347;
pub const WHILE: i32 = 57348;
pub const IF: i32 = 57349;
pub const NOT: i32 = 57350;
pub const TWIDDLE: i32 = 57351;
pub const BANG: i32 = 57352;
pub const SUBSHELL: i32 = 57353;
pub const SWITCH: i32 = 57354;
pub const FN: i32 = 57355;
pub const WORD: i32 = 57356;
pub const REDIR: i32 = 57357;
pub const REDIRW: i32 = 57358;
pub const DUP: i32 = 57359;
pub const PIPE: i32 = 57360;
pub const SUB: i32 = 57361;
pub const SIMPLE: i32 = 57362;
pub const ARGLIST: i32 = 57363;
pub const WORDS: i32 = 57364;
pub const BRACE: i32 = 57365;
pub const PAREN: i32 = 57366;
pub const PCMD: i32 = 57367;
pub const PIPEFD: i32 = 57368;
pub const ANDAND: i32 = 57369;
pub const OROR: i32 = 57370;
pub const COUNT: i32 = 57371;

/// Redirection kinds (`t->rtype`).
pub const APPEND: i32 = 1;
pub const WRITE: i32 = 2;
pub const READ: i32 = 3;
pub const HERE: i32 = 4;
pub const DUPFD: i32 = 5;
pub const CLOSE: i32 = 6;
pub const RDWR: i32 = 7;

/// The glob escape byte: in a word, GLOB must be followed by `*`, `?`, `[`
/// or another GLOB.
pub const GLOB: u8 = 0x01;

#[derive(Debug, Clone)]
pub struct Tree {
    pub ty: i32,
    /// details of REDIR PIPE DUP tokens
    pub rtype: i32,
    pub fd0: i32,
    pub fd1: i32,
    pub s: Option<Word>,
    pub quoted: bool,
    pub iskw: bool,
    pub child: [T; 3],
}

pub type T = Option<Box<Tree>>;

pub fn newtree() -> Box<Tree> {
    Box::new(Tree {
        ty: 0,
        rtype: 0,
        fd0: 0,
        fd1: 0,
        s: None,
        quoted: false,
        iskw: false,
        child: [None, None, None],
    })
}

pub fn tree1(ty: i32, c0: T) -> T {
    tree3(ty, c0, None, None)
}

pub fn tree2(ty: i32, c0: T, c1: T) -> T {
    tree3(ty, c0, c1, None)
}

pub fn tree3(ty: i32, c0: T, c1: T, c2: T) -> T {
    if ty == b';' as i32 {
        if c0.is_none() {
            return c1;
        }
        if c1.is_none() {
            return c0;
        }
    }
    let mut t = newtree();
    t.ty = ty;
    t.child = [c0, c1, c2];
    Some(t)
}

pub fn mung1(mut t: Box<Tree>, c0: T) -> Box<Tree> {
    t.child[0] = c0;
    t
}

pub fn mung2(mut t: Box<Tree>, c0: T, c1: T) -> Box<Tree> {
    t.child[0] = c0;
    t.child[1] = c1;
    t
}

pub fn mung3(mut t: Box<Tree>, c0: T, c1: T, c2: T) -> Box<Tree> {
    t.child[0] = c0;
    t.child[1] = c1;
    t.child[2] = c2;
    t
}

/// Attach the compound command `comp` at the bottom of the redirection
/// chain `epi` (whose nodes are linked through `child[1]`).
pub fn epimung(comp: Box<Tree>, epi: T) -> Box<Tree> {
    match epi {
        None => comp,
        Some(mut epi) => {
            {
                let mut p: &mut Tree = &mut epi;
                while p.child[1].is_some() {
                    p = p.child[1].as_mut().unwrap();
                }
                p.child[1] = Some(comp);
            }
            epi
        }
    }
}

/// Add a SIMPLE node at the root of `t` and percolate all the redirections
/// up to the root.
pub fn simplemung(t: Box<Tree>) -> Box<Tree> {
    let mut t = tree1(SIMPLE, Some(t)).unwrap();
    // Collect the redirections out of the ARGLIST chain, in the order the
    // C code encounters them (last argument first).
    let mut redirs: Vec<Box<Tree>> = Vec::new();
    {
        let mut u: &mut Tree = t.child[0].as_mut().unwrap();
        while u.ty == ARGLIST {
            let is_redir = matches!(&u.child[1], Some(c) if c.ty == DUP || c.ty == REDIR);
            if is_redir {
                redirs.push(u.child[1].take().unwrap());
            }
            u = u.child[0].as_mut().unwrap();
        }
    }
    for mut r in redirs {
        r.child[1] = Some(t);
        t = r;
    }
    t
}

pub fn token(s: &[u8], ty: i32) -> Box<Tree> {
    let mut t = newtree();
    t.ty = ty;
    t.s = Some(s.to_vec());
    t
}

/// Look up a keyword (port of `klook` in `var.c`).
pub fn klook(name: &[u8]) -> Box<Tree> {
    let mut t = token(name, WORD);
    let kw = match name {
        b"for" => FOR,
        b"in" => IN,
        b"while" => WHILE,
        b"if" => IF,
        b"not" => NOT,
        b"~" => TWIDDLE,
        b"!" => BANG,
        b"@" => SUBSHELL,
        b"switch" => SWITCH,
        b"fn" => FN,
        _ => WORD,
    };
    if kw != WORD {
        t.ty = kw;
        t.iskw = true;
    }
    t
}
