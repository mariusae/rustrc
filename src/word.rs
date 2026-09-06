//! Words and word lists.
//!
//! rc words are byte strings (they may hold arbitrary bytes coming from file
//! names, environment variables or command output), so they are `Vec<u8>`
//! rather than `String`.  A word list is a `VecDeque` whose front is the
//! head of the C linked list.

use std::collections::VecDeque;

pub type Word = Vec<u8>;
pub type Words = VecDeque<Word>;

/// Port of `list2str`: join the words with single spaces.
pub fn list2str(words: &Words) -> Word {
    let mut v = Vec::new();
    for (i, w) in words.iter().enumerate() {
        if i != 0 {
            v.push(b' ');
        }
        v.extend_from_slice(w);
    }
    v
}

/// Port of `list2strcolon`: join the words with colons.
pub fn list2strcolon(words: &Words) -> Word {
    let mut v = Vec::new();
    for (i, w) in words.iter().enumerate() {
        if i != 0 {
            v.push(b':');
        }
        v.extend_from_slice(w);
    }
    v
}

/// C `atoi`: optional leading white space, optional sign, digits.
pub fn atoi(s: &[u8]) -> i32 {
    let mut i = 0;
    while i < s.len() && matches!(s[i], b' ' | b'\t' | b'\n' | b'\r' | 0x0b | 0x0c) {
        i += 1;
    }
    let mut neg = false;
    if i < s.len() && (s[i] == b'+' || s[i] == b'-') {
        neg = s[i] == b'-';
        i += 1;
    }
    let mut n: i64 = 0;
    while i < s.len() && s[i].is_ascii_digit() {
        n = n * 10 + (s[i] - b'0') as i64;
        if n > i32::MAX as i64 + 1 {
            n = i32::MAX as i64 + 1;
        }
        i += 1;
    }
    let n = if neg { -n } else { n };
    n.clamp(i32::MIN as i64, i32::MAX as i64) as i32
}

/// Port of `inttoascii`.
pub fn inttoascii(n: i64) -> Word {
    n.to_string().into_bytes()
}

/// Find the first occurrence of `c` in `s`.
pub fn strchr(s: &[u8], c: u8) -> Option<usize> {
    s.iter().position(|&b| b == c)
}
