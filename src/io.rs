//! Buffered I/O (port of `io.c`/`io.h`).
//!
//! The buffer size and flushing behaviour are reproduced exactly: rc reads
//! its input in 512-byte chunks, which is observable when child processes
//! share the same input stream.

use crate::word::Word;

pub const NBUF: usize = 512;
pub const EOF: i32 = -1;

#[derive(Debug)]
enum Kind {
    /// Backed by a file descriptor (read or write, never both).
    Fd(i32),
    /// An in-memory output string (`openstr`).
    Str,
    /// An in-memory input buffer (`opencore`); EOF after the buffer.
    Core,
}

#[derive(Debug)]
pub struct Io {
    kind: Kind,
    buf: Vec<u8>,
    bufp: usize,
    ebuf: usize,
}

impl Io {
    /// `openfd`
    pub fn openfd(fd: i32) -> Io {
        Io { kind: Kind::Fd(fd), buf: vec![0; NBUF], bufp: 0, ebuf: 0 }
    }

    /// `openstr`
    pub fn openstr() -> Io {
        Io { kind: Kind::Str, buf: Vec::new(), bufp: 0, ebuf: 0 }
    }

    /// `opencore`: EOF occurs after reading all of `s`.
    pub fn opencore(s: &[u8]) -> Io {
        let buf = s.to_vec();
        let n = buf.len();
        Io { kind: Kind::Core, buf, bufp: 0, ebuf: n }
    }

    pub fn fd(&self) -> i32 {
        match self.kind {
            Kind::Fd(fd) => fd,
            _ => -1,
        }
    }

    /// The accumulated contents of an `openstr` buffer.
    pub fn strp(&self) -> &[u8] {
        match self.kind {
            Kind::Str => &self.buf,
            _ => &[],
        }
    }

    pub fn into_string(self) -> Word {
        match self.kind {
            Kind::Str => self.buf,
            _ => Vec::new(),
        }
    }

    /// `pchr`
    pub fn pchr(&mut self, c: u8) {
        match self.kind {
            Kind::Str => self.buf.push(c),
            _ => {
                if self.bufp == self.ebuf {
                    self.fullbuf(c);
                } else {
                    self.buf[self.bufp] = c;
                    self.bufp += 1;
                }
            }
        }
    }

    fn fullbuf(&mut self, c: u8) {
        self.flush();
        self.buf[self.bufp] = c;
        self.bufp += 1;
    }

    /// `flush`
    pub fn flush(&mut self) {
        match self.kind {
            Kind::Str => {}
            Kind::Core => {}
            Kind::Fd(fd) => {
                let n = self.bufp;
                if n != 0 && crate::unix::write_all(fd, &self.buf[..n]) < 0 {
                    crate::unix::write_all(3, b"Write error\n");
                }
                self.bufp = 0;
                self.ebuf = NBUF;
            }
        }
    }

    /// `rchr` for buffers that need no trap processing (core buffers).
    /// Returns EOF at the end.  For fd-backed buffers the caller must use
    /// [`Io::rchr_with`] so that reads can be interrupted properly.
    pub fn rchr_core(&mut self) -> i32 {
        if self.bufp == self.ebuf {
            return EOF;
        }
        let c = self.buf[self.bufp];
        self.bufp += 1;
        c as i32
    }

    /// `rchr`: `read_fd` performs the actual read on a file descriptor and
    /// returns the number of bytes read (or a negative value on error).
    pub fn rchr_with<F: FnMut(i32, &mut [u8]) -> isize>(&mut self, mut read_fd: F) -> i32 {
        if self.bufp == self.ebuf {
            return self.emptybuf(&mut read_fd);
        }
        let c = self.buf[self.bufp];
        self.bufp += 1;
        c as i32
    }

    fn emptybuf<F: FnMut(i32, &mut [u8]) -> isize>(&mut self, read_fd: &mut F) -> i32 {
        let fd = match self.kind {
            Kind::Fd(fd) => fd,
            _ => return EOF,
        };
        loop {
            let n = read_fd(fd, &mut self.buf[..NBUF]);
            if n < 0 && crate::unix::errno() == libc::EINTR {
                continue;
            }
            if n <= 0 {
                return EOF;
            }
            self.bufp = 0;
            self.ebuf = n as usize;
            let c = self.buf[0];
            self.bufp = 1;
            return c as i32;
        }
    }

    /// `closeio`
    pub fn close(self) {
        if let Kind::Fd(fd) = self.kind {
            if fd >= 0 {
                unsafe {
                    libc::close(fd);
                }
            }
        }
    }

    // ---- formatting helpers (port of pfmt's conversions) ----

    /// `%s`
    pub fn pstr(&mut self, s: &[u8]) {
        for &c in s {
            self.pchr(c);
        }
    }

    /// `%d`
    pub fn pdec(&mut self, n: i64) {
        self.pstr(n.to_string().as_bytes());
    }

    /// `%o`
    pub fn poct(&mut self, n: u32) {
        self.pstr(format!("{:o}", n).as_bytes());
    }

    /// `%p`
    pub fn pptr(&mut self, p: usize) {
        let s = if p >> 32 != 0 {
            format!("{:016X}", p)
        } else {
            format!("{:08X}", p)
        };
        self.pstr(s.as_bytes());
    }

    /// `%Q`: always quote.
    pub fn pquo(&mut self, s: &[u8]) {
        self.pchr(b'\'');
        for &c in s {
            if c == b'\'' {
                self.pstr(b"''");
            } else {
                self.pchr(c);
            }
        }
        self.pchr(b'\'');
    }

    /// `%q`: quote if necessary.
    pub fn pwrd(&mut self, s: &[u8]) {
        // The C code examines each byte as a (signed) char; a 0xff byte is
        // therefore indistinguishable from EOF and forces quoting.
        let needs = s.is_empty() || s.iter().any(|&c| !crate::lex::wordchr(c as i8 as i32));
        if needs {
            self.pquo(s);
        } else {
            self.pstr(s);
        }
    }

    /// `%v`
    pub fn pval(&mut self, a: &crate::word::Words) {
        let n = a.len();
        for (i, w) in a.iter().enumerate() {
            self.pwrd(w);
            if i + 1 < n {
                self.pchr(b' ');
            }
        }
    }
}
