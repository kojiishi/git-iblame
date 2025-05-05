use std::{
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
    process,
    sync::mpsc,
    thread,
};

use log::*;

#[derive(Debug, Default)]
pub struct CommitIterator {
    path: PathBuf,
    repository_path: PathBuf,
    log_child: Option<process::Child>,
    receive_thread: Option<thread::JoinHandle<anyhow::Result<()>>>,
    rx: Option<mpsc::Receiver<git2::Oid>>,
}

impl CommitIterator {
    pub fn new(path: &Path, repository_path: &Path) -> Self {
        assert!(path.is_relative());
        Self {
            path: path.to_path_buf(),
            repository_path: repository_path.to_path_buf(),
            log_child: None,
            receive_thread: None,
            rx: None,
        }
    }

    pub fn start(&mut self) -> anyhow::Result<()> {
        // self.commits_by_thread()?;
        self.commits_by_process()?;
        Ok(())
    }

    pub fn join(&mut self) -> anyhow::Result<()> {
        if let Some(mut child) = self.log_child.take() {
            let status = child.wait()?;
            debug!("Child process exited with: {status}");
        }
        if let Some(receiver) = self.receive_thread.take() {
            receiver.join().unwrap()?;
        }
        Ok(())
    }

    fn commits_by_process(&mut self) -> anyhow::Result<()> {
        let mut child = process::Command::new("git")
            .args(["log", "--format=%H", "--follow", "--"])
            .arg(&self.path)
            .current_dir(&self.repository_path)
            .stdout(process::Stdio::piped())
            .spawn()?;
        let stdout = child.stdout.take().unwrap();
        self.log_child = Some(child);

        let (tx, rx) = mpsc::channel::<git2::Oid>();
        let receiver = thread::spawn(move || -> anyhow::Result<()> {
            trace!("receiver thread start");
            let reader = BufReader::new(stdout);
            for line in reader.lines() {
                match line {
                    Ok(line) => {
                        let commit_id = git2::Oid::from_str(&line).unwrap();
                        trace!("THREAD: {commit_id}");
                        tx.send(commit_id)?;
                    }
                    Err(error) => {
                        println!("Error: {error}");
                        break;
                    }
                }
            }
            trace!("receiver thread end");
            Ok(())
        });
        self.receive_thread = Some(receiver);
        self.rx = Some(rx);
        Ok(())
    }
}

impl Iterator for CommitIterator {
    type Item = git2::Oid;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(rx) = self.rx.as_mut() {
            rx.recv().ok()
        } else {
            None
        }
    }
}
