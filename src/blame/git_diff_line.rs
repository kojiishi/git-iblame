use std::{
    borrow::Cow,
    io::{self, BufRead},
};

#[derive(Debug)]
pub struct GitDiffLine<R: BufRead> {
    buffer: Vec<u8>,
    valid_len: usize,
    invalid_len: usize,
    inner: R,
}

impl<R: BufRead> GitDiffLine<R> {
    pub fn new(inner: R) -> Self {
        Self {
            buffer: vec![],
            valid_len: 0,
            invalid_len: 0,
            inner,
        }
    }

    pub fn invalid_len(&self) -> usize {
        self.invalid_len
    }

    pub fn as_str(&self) -> &str {
        unsafe { std::str::from_utf8_unchecked(&self.buffer[..self.valid_len]) }
    }

    pub fn to_lossy_string(&self) -> Cow<'_, str> {
        String::from_utf8_lossy(&self.buffer[..self.valid_len])
    }

    pub fn next_line(&mut self) -> io::Result<bool> {
        self.buffer.clear();
        let mut len = self.inner.read_until(b'\n', &mut self.buffer)?;
        if len == 0 {
            return Ok(false);
        }

        if len > 0 && self.buffer[len - 1] == b'\n' {
            len -= 1;
        }
        match std::str::from_utf8(&self.buffer[..len]) {
            Ok(s) => {
                self.valid_len = s.len();
                self.invalid_len = 0;
            }
            Err(error) => {
                self.valid_len = error.valid_up_to();
                self.invalid_len = len - self.valid_len;
            }
        }
        Ok(true)
    }
}
