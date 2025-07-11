use std::{
    fmt::{self},
    ops::Range,
};

use anyhow::bail;

#[derive(Clone, Default, Eq, PartialEq)]
pub struct DiffPart {
    pub old: DiffRange,
    pub new: DiffRange,
}

impl DiffPart {
    #[cfg(test)]
    pub fn from_ranges(old: Range<usize>, new: Range<usize>) -> Self {
        Self {
            old: DiffRange::from_range(old),
            new: DiffRange::from_range(new),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.old.is_empty() && self.new.is_empty()
    }

    pub fn validate_ascending(&self) -> anyhow::Result<()> {
        self.old.validate_ascending()?;
        self.new.validate_ascending()
    }

    pub fn validate_ascending_parts(parts: &Vec<DiffPart>) -> anyhow::Result<()> {
        let mut last_old = 0;
        let mut last_new = 0;
        for part in parts {
            if part.old.line_numbers.start < last_old {
                bail!("old start isn't ascending: {:?}", part);
            }
            part.old
                .validate_ascending()
                .unwrap_or_else(|_| panic!("old start and end aren't ascending: {part:?}"));
            if part.new.line_numbers.start < last_new {
                bail!("new start isn't ascending: {:?}", part);
            }
            part.new
                .validate_ascending()
                .unwrap_or_else(|_| panic!("new start and end aren't ascending: {part:?}"));
            last_old = part.old.line_numbers.end;
            last_new = part.new.line_numbers.end;
        }
        Ok(())
    }
}

impl fmt::Debug for DiffPart {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!("{:?}->{:?}", self.old, self.new))
    }
}

#[derive(Clone, Default, Eq, PartialEq)]
pub struct DiffRange {
    pub line_numbers: Range<usize>,
}

impl fmt::Debug for DiffRange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!("{:?}", self.line_numbers))
    }
}

impl DiffRange {
    #[cfg(test)]
    pub fn from_range(line_numbers: Range<usize>) -> Self {
        Self { line_numbers }
    }

    pub fn is_empty(&self) -> bool {
        self.line_numbers.is_empty()
    }

    pub fn line_numbers(&self) -> &Range<usize> {
        &self.line_numbers
    }

    pub fn len(&self) -> usize {
        self.line_numbers.len()
    }

    pub fn is_ascending(&self) -> bool {
        self.line_numbers.start <= self.line_numbers.end
    }

    pub fn validate_ascending(&self) -> anyhow::Result<()> {
        if !self.is_ascending() {
            bail!("start > end: {:?}", self);
        }
        Ok(())
    }

    pub fn add_line(&mut self, line_number: usize) {
        if self.line_numbers.is_empty() {
            self.line_numbers = line_number..line_number + 1;
        } else {
            assert_eq!(self.line_numbers.end, line_number);
            self.line_numbers.end += 1;
        }
        self.validate_ascending().unwrap();
    }

    pub fn set_line_number_if_empty(&mut self, line_number: usize) {
        if self.line_numbers.is_empty() {
            self.line_numbers = line_number..line_number;
        }
    }
}
