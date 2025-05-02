use git2::{Oid, Signature, Time};

#[derive(Debug)]
pub struct BlameCommit {
    pub index: usize,
    pub commit_id: Oid,
    pub when: Time,
    pub email: Option<String>,
    pub name: Option<String>,
}

impl Default for BlameCommit {
    fn default() -> Self {
        Self {
            index: 0,
            commit_id: Oid::zero(),
            when: Time::new(0, 0),
            email: None,
            name: None,
        }
    }
}

impl BlameCommit {
    pub fn new_with_commit_id(commit_id: Oid) -> Self {
        Self {
            commit_id,
            ..Default::default()
        }
    }

    pub fn new_with_signature(commit_id: Oid, signature: &Signature) -> Self {
        Self {
            commit_id,
            when: signature.when(),
            email: signature.email().map(String::from),
            name: signature.name().map(String::from),
            ..Default::default()
        }
    }
}
