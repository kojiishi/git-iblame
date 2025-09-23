#[derive(thiserror::Error, Debug)]
pub enum BlameError {
    #[error("The file was deleted at {0:?}")]
    FileDeleted(git2::Oid),
}
