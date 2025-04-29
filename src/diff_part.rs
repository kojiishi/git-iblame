use std::ops::Range;

use git2::{BlameHunk, Oid, Time};

#[derive(Debug)]
pub struct DiffPart {
    pub line_number: Range<usize>,
    pub orig_start_line_number: usize,
    pub commit_id: Oid,
    pub when: Time,
    pub email: String,
    pub name: String,
}

impl Default for DiffPart {
    fn default() -> Self {
        Self {
            line_number: Range::default(),
            orig_start_line_number: 0,
            commit_id: Oid::zero(),
            when: Time::new(0, 0),
            email: String::new(),
            name: String::new(),
        }
    }
}

impl DiffPart {
    pub fn new(hunk: BlameHunk) -> Self {
        let signature = hunk.final_signature();
        Self {
            line_number: hunk.final_start_line()..(hunk.final_start_line() + hunk.lines_in_hunk()),
            orig_start_line_number: hunk.orig_start_line(),
            commit_id: hunk.final_commit_id(),
            when: signature.when(),
            email: signature.email().map_or(String::new(), String::from),
            name: signature.name().map_or(String::new(), String::from),
        }
    }
}
