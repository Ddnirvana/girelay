use anyhow::{Context, Result, anyhow};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

pub struct TaskLock {
    path: PathBuf,
}

pub fn path(source: &Path, task_id: &str) -> PathBuf {
    source
        .join(".girelay/locks")
        .join(format!("{task_id}.lock"))
}

pub fn acquire(source: &Path, task_id: &str, recover_stale: bool, owner: &str) -> Result<TaskLock> {
    let path = path(source, task_id);
    if recover_stale && path.exists() {
        fs::remove_file(&path)
            .with_context(|| format!("failed to remove stale task lock {}", path.display()))?;
    }
    let mut file = OpenOptions::new().write(true).create_new(true).open(&path).map_err(|error| {
        if error.kind() == std::io::ErrorKind::AlreadyExists {
            anyhow!("task '{task_id}' has an active or stale operation; confirm no agent, merge, or cleanup process is running before recovering the lock")
        } else { anyhow!(error).context(format!("failed to create task lock {}", path.display())) }
    })?;
    writeln!(file, "owner={owner}")?;
    writeln!(file, "pid={}", std::process::id())?;
    Ok(TaskLock { path })
}

impl Drop for TaskLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}
