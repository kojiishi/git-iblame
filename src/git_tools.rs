use std::path::{Path, PathBuf};

use chrono::TimeZone;
use git2::{Oid, Repository, RepositoryOpenFlags, Time};

pub struct GitTools {
    repository: Repository,
    root_path: PathBuf,
}

impl GitTools {
    pub fn new(path: &Path) -> anyhow::Result<Self> {
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

    pub fn repository(&mut self) -> &mut Repository {
        &mut self.repository
    }

    pub fn root_path(&self) -> &Path {
        &self.root_path
    }

    pub fn content_as_string(&mut self, commit_id: Oid, path: &Path) -> anyhow::Result<String> {
        let commit = self.repository.find_commit(commit_id)?;
        let tree = commit.tree()?;
        let entry = tree.get_path(path)?;
        let object = entry.to_object(&self.repository)?;
        let blob = object.into_blob().unwrap();
        let content = std::str::from_utf8(blob.content())?.to_string();
        Ok(content)
    }

    pub fn older_commit_id(&self, id: Oid) -> anyhow::Result<Oid> {
        let mut revwalk = self.repository.revwalk()?;
        revwalk.push(id)?;
        let _ = revwalk.next().unwrap()?;
        let previous_id = revwalk.next().unwrap()?;
        Ok(previous_id)
    }

    pub fn to_date_time(time: Time) -> Option<chrono::DateTime<chrono::FixedOffset>> {
        if let Some(tz) = chrono::FixedOffset::east_opt(time.offset_minutes() * 60) {
            let result = tz.timestamp_opt(time.seconds(), 0);
            return match result {
                chrono::MappedLocalTime::Single(datetime) => Some(datetime),
                chrono::MappedLocalTime::Ambiguous(_, latest) => Some(latest),
                chrono::MappedLocalTime::None => None,
            };
        }
        None
    }

    pub fn to_local_date_time(time: Time) -> Option<chrono::DateTime<chrono::Local>> {
        GitTools::to_date_time(time).map(|datetime| datetime.with_timezone(&chrono::Local))
    }

    pub fn to_short_debug_str(time: Time) -> String {
        format!("{} {}", time.seconds(), time.offset_minutes())
    }
}
