use std::collections::HashMap;
use std::ops::{Deref, Index};
use std::slice::{self, SliceIndex};

use super::FileCommit;

/// A collection of `FileCommit` objects, providing efficient lookup by OID.
#[derive(Debug, Default)]
pub struct FileCommits {
    items: Vec<FileCommit>,
    index_map: HashMap<git2::Oid, usize>,
}

impl FileCommits {
    /// Creates a new, empty `FileCommits` collection.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a `FileCommit` to the collection.
    ///
    /// The `FileCommit`'s internal index will be updated to its position in this collection.
    pub fn push(&mut self, mut commit: FileCommit) {
        let index = self.items.len();
        commit.set_index(index); // Update the commit's own index
        self.index_map.insert(commit.commit_id(), index);
        self.items.push(commit);
    }

    /// Returns the number of commits in the collection.
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Returns `true` if the collection contains no commits.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Returns a reference to the `FileCommit` at the given index, or `None` if the index is out of bounds.
    pub fn get(&self, index: usize) -> Option<&FileCommit> {
        self.items.get(index)
    }

    /// Returns the index of the `FileCommit` with the given `Oid`, or `None` if not found.
    fn index_from_commit_id_opt(&self, commit_id: git2::Oid) -> Option<usize> {
        if commit_id.is_zero() {
            if self.items.is_empty() {
                return Some(0);
            }
            return None;
        }
        self.index_map.get(&commit_id).copied()
    }

    pub fn index_from_commit_id(&self, commit_id: git2::Oid) -> anyhow::Result<usize> {
        self.index_from_commit_id_opt(commit_id)
            .ok_or_else(|| anyhow::anyhow!("Commit {commit_id:?} not found"))
    }

    /// Returns a reference to the `FileCommit` with the given `Oid`, or `None` if not found.
    fn get_by_commit_id_opt(&self, commit_id: git2::Oid) -> Option<&FileCommit> {
        self.index_from_commit_id_opt(commit_id)
            .and_then(|index| self.items.get(index))
    }

    pub fn get_by_commit_id(&self, commit_id: git2::Oid) -> anyhow::Result<&FileCommit> {
        self.get_by_commit_id_opt(commit_id)
            .ok_or_else(|| anyhow::anyhow!("Commit {commit_id:?} not found"))
    }

    /// Returns an iterator over the commits in the collection.
    pub fn iter(&self) -> slice::Iter<'_, FileCommit> {
        self.items.iter()
    }

    /// Returns a reference to the first `FileCommit` in the collection, or `None` if it's empty.
    pub fn first(&self) -> Option<&FileCommit> {
        self.items.first()
    }

    /// Returns a slice containing all commits.
    pub fn as_slice(&self) -> &[FileCommit] {
        self.items.as_slice()
    }
}

/// Allows `&FileCommits` to be automatically dereferenced to `&[FileCommit]`.
impl Deref for FileCommits {
    type Target = [FileCommit];

    fn deref(&self) -> &Self::Target {
        &self.items
    }
}

/// Allows indexing `FileCommits` by types that implement `SliceIndex`
/// (e.g., `usize`, `Range<usize>`, `RangeFrom<usize>`, etc.).
/// This provides direct access to the underlying `Vec<FileCommit>`'s indexing capabilities,
/// allowing for retrieval of single `FileCommit` references or slices (`&[FileCommit]`).
///
/// # Panics
/// Panics if the index is out of bounds, consistent with slice indexing.
impl<I: SliceIndex<[FileCommit]>> Index<I> for FileCommits {
    type Output = I::Output;

    #[inline]
    fn index(&self, index: I) -> &Self::Output {
        &self.items[index]
    }
}

/// Allows iterating over `&FileCommits` to get `&FileCommit`.
impl<'a> IntoIterator for &'a FileCommits {
    type Item = &'a FileCommit;
    type IntoIter = slice::Iter<'a, FileCommit>;

    fn into_iter(self) -> Self::IntoIter {
        self.items.iter()
    }
}
