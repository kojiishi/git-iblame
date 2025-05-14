use std::{
    borrow::Cow,
    io::{self, BufRead},
};

use log::warn;

#[derive(Debug, Default)]
pub struct LineReadBuffer {
    buffer: Vec<u8>,
    valid_len: usize,
    invalid_len: usize,
}

impl LineReadBuffer {
    pub fn new() -> Self {
        Self {
            buffer: vec![],
            valid_len: 0,
            invalid_len: 0,
        }
    }

    fn as_str(&self) -> &str {
        unsafe { std::str::from_utf8_unchecked(&self.buffer[..self.valid_len]) }
    }

    pub fn to_string_lossy(&self) -> Cow<'_, str> {
        String::from_utf8_lossy(&self.buffer[..self.total_len()])
    }

    pub fn invalid_len(&self) -> usize {
        self.invalid_len
    }

    fn total_len(&self) -> usize {
        self.valid_len + self.invalid_len
    }

    pub fn error(&self) -> anyhow::Error {
        assert!(self.invalid_len > 0);
        anyhow::anyhow!(
            "Invalid UTF-8 at {}: \"{}\"",
            self.valid_len,
            self.to_string_lossy()
        )
    }

    pub fn read_line_from(&mut self, reader: &mut impl BufRead) -> io::Result<bool> {
        self.buffer.clear();
        let mut len = reader.read_until(b'\n', &mut self.buffer)?;
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
                warn!("{}", self.error());
            }
        }
        Ok(true)
    }
}

impl AsRef<str> for LineReadBuffer {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}
