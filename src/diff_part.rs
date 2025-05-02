use std::{ops::Range, path::PathBuf, rc::Rc};

use git2::{BlameHunk, Oid, Signature};

use crate::BlameCommit;

#[derive(Debug)]
pub struct DiffPart {
    pub line_number: Range<usize>,
    pub orig_start_line_number: usize,
    pub orig_path: Option<PathBuf>,
    pub commit: Rc<BlameCommit>,
}

impl Default for DiffPart {
    fn default() -> Self {
        Self {
            line_number: Range::default(),
            orig_start_line_number: 0,
            orig_path: None,
            commit: Rc::new(BlameCommit::default()),
        }
    }
}

impl DiffPart {
    pub fn new<F>(hunk: BlameHunk, mut get_commit: F) -> Self
    where
        F: FnMut(Oid, &Signature) -> Rc<BlameCommit>,
    {
        let signature = hunk.final_signature();
        let commit = get_commit(hunk.final_commit_id(), &signature);
        Self {
            line_number: hunk.final_start_line()..(hunk.final_start_line() + hunk.lines_in_hunk()),
            orig_start_line_number: hunk.orig_start_line(),
            orig_path: hunk.path().map(PathBuf::from),
            commit,
        }
    }

    pub fn commit_id(&self) -> Oid {
        self.commit.commit_id
    }
}
