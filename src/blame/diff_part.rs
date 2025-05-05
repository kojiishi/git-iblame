use std::ops::Range;

#[derive(Clone, Debug, Default)]
pub struct DiffPart {
    pub old: DiffLines,
    pub new: DiffLines,
}

impl DiffPart {
    pub fn is_empty(&self) -> bool {
        self.old.is_empty() && self.new.is_empty()
    }
}

#[derive(Clone, Debug, Default)]
pub struct DiffLines {
    pub line_numbers: Range<usize>,
}

impl DiffLines {
    pub fn is_empty(&self) -> bool {
        self.line_numbers.is_empty()
    }

    pub fn start_line_number(&self) -> usize {
        self.line_numbers.start
    }

    pub fn line_numbers(&self) -> &Range<usize> {
        &self.line_numbers
    }

    pub fn add_line(&mut self, line_number: usize) {
        if self.line_numbers.is_empty() {
            self.line_numbers = line_number..line_number + 1;
        } else {
            assert_eq!(self.line_numbers.end, line_number);
            self.line_numbers.end += 1;
        }
    }
}
