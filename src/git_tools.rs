use std::path::{Path, PathBuf};

use anyhow::*;
use chrono::TimeZone;
use git2::{Oid, Repository, RepositoryOpenFlags, Time};

pub struct GitTools {
    repository: Repository,
    root_path: PathBuf,
}

impl GitTools {
    /// Construct from the `Path` to a file in the repository.
    pub fn from_path(path: &Path) -> anyhow::Result<Self> {
        let repository = Repository::open_ext(
            &path,
            RepositoryOpenFlags::empty(),
            &[] as &[&std::ffi::OsStr],
        )?;

        let git_path = repository.path().canonicalize()?;
        let root_path = git_path.parent().unwrap();

        Ok(Self {
            repository: repository,
            root_path: root_path.to_path_buf(),
        })
    }

    /// Get `git2::Repository`.
    pub fn repository(&self) -> &Repository {
        &self.repository
    }

    /// Get the canonicalized root directory.
    pub fn root_path(&self) -> &Path {
        &self.root_path
    }

    /// Get the content of a `path` at the tree of the `commit_id` as a string.
    pub fn content_as_string(&self, commit_id: Oid, path: &Path) -> anyhow::Result<String> {
        let commit = self.repository.find_commit(commit_id)?;
        let tree = commit.tree()?;
        let entry = tree.get_path(path)?;
        let object = entry.to_object(&self.repository)?;
        // https://github.com/rust-lang/git2-rs/issues/1156
        let blob = object.into_blob().unwrap();
        let content = std::str::from_utf8(blob.content())?.to_string();
        Ok(content)
    }

    /// Get the commit id of one older commit of `commit_id`.
    pub fn older_commit_id(&self, commit_id: Oid) -> Result<Option<Oid>, git2::Error> {
        let mut revwalk = self.repository.revwalk()?;
        revwalk.push(commit_id)?;
        let first_id = revwalk.next().unwrap()?;
        assert_eq!(commit_id, first_id);
        revwalk.next().transpose()
    }

    /// Convert `git2::Time` to `chrono::DateTime<chrono::FixedOffset>`.
    /// The time zone is set to the one in the given `git2::Time`.
    fn to_fixed_date_time(time: Time) -> anyhow::Result<chrono::DateTime<chrono::FixedOffset>> {
        let tz = chrono::FixedOffset::east_opt(time.offset_minutes() * 60);
        if tz.is_none() {
            bail!("Invalid TimeZone {}", time.offset_minutes());
        }
        match tz.unwrap().timestamp_opt(time.seconds(), 0) {
            chrono::MappedLocalTime::Single(datetime) => Ok(datetime),
            chrono::MappedLocalTime::Ambiguous(_, latest) => Ok(latest),
            chrono::MappedLocalTime::None => bail!(
                "Time {} isn't mappable to {}",
                time.seconds(),
                time.offset_minutes()
            ),
        }
    }

    fn to_date_time_in<Tz: TimeZone>(time: Time, tz: &Tz) -> anyhow::Result<chrono::DateTime<Tz>> {
        Self::to_fixed_date_time(time).map(|datetime| datetime.with_timezone(tz))
    }

    /// Convert `git2::Time` to `chrono::DateTime<chrono::Local>`.
    /// The time zone is converted to the local time zone.
    pub fn to_local_date_time(time: Time) -> anyhow::Result<chrono::DateTime<chrono::Local>> {
        Self::to_date_time_in(time, &chrono::Local)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn to_date_time_east() {
        let time = Time::new(1745693791, 540);
        let datetime = GitTools::to_fixed_date_time(time);
        assert!(datetime.is_ok());
        assert_eq!(datetime.unwrap().to_string(), "2025-04-27 03:56:31 +09:00");
    }

    #[test]
    fn to_date_time_west() {
        let time = Time::new(1745196130, -420);
        let datetime = GitTools::to_fixed_date_time(time);
        assert!(datetime.is_ok());
        assert_eq!(datetime.unwrap().to_string(), "2025-04-20 17:42:10 -07:00");
    }

    #[test]
    fn to_date_time_offset_invalid() {
        let time = Time::new(0, 100_000);
        let datetime = GitTools::to_fixed_date_time(time);
        assert!(datetime.is_err());
        assert_eq!(datetime.unwrap_err().to_string(), "Invalid TimeZone 100000");
    }
}
