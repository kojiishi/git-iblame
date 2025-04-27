use std::ops::Range;

use git2::{BlameHunk, Oid, Time};

#[derive(Debug)]
pub struct DiffPart {
    pub range: Range<usize>,
    pub commit_id: Oid,
    pub when: Time,
    pub email: String,
    pub name: String,
}

impl Default for DiffPart {
    fn default() -> Self {
        Self {
            range: Range::default(),
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
            range: hunk.final_start_line()..(hunk.final_start_line() + hunk.lines_in_hunk()),
            commit_id: hunk.final_commit_id(),
            when: signature.when(),
            email: signature.email().map_or(String::new(), String::from),
            name: signature.name().map_or(String::new(), String::from),
        }
    }
}
