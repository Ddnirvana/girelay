use crate::task;
use anyhow::{Context, Result, anyhow};
use serde::{Deserialize, Serialize};
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
#[cfg(unix)]
use std::process::Stdio;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LockRecord {
    pub schema_version: u32,
    pub operation: String,
    pub parent_pid: u32,
    pub child_pid: Option<u32>,
    pub created_at: String,
}

pub struct TaskLock {
    path: PathBuf,
    record: LockRecord,
    active: bool,
}

pub struct StaleLockClaim {
    lock: TaskLock,
    original_path: PathBuf,
    stale_path: PathBuf,
    finished: bool,
}

pub fn path(source: &Path, task_id: &str) -> PathBuf {
    source
        .join(".girelay/locks")
        .join(format!("{task_id}.lock"))
}

pub fn acquire(source: &Path, task_id: &str, operation: &str) -> Result<TaskLock> {
    let path = path(source, task_id);
    let record = LockRecord {
        schema_version: 1,
        operation: operation.into(),
        parent_pid: std::process::id(),
        child_pid: None,
        created_at: task::timestamp(),
    };
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .map_err(|error| {
            if error.kind() == std::io::ErrorKind::AlreadyExists {
                anyhow!(
                    "task '{task_id}' has an active or stale operation; inspect it with `girelay recover unlock {task_id}`"
                )
            } else {
                anyhow!(error).context(format!("failed to create task lock {}", path.display()))
            }
        })?;
    if let Err(error) = write_record(&mut file, &record) {
        let _ = fs::remove_file(&path);
        return Err(error);
    }
    Ok(TaskLock {
        path,
        record,
        active: true,
    })
}

fn write_record(file: &mut File, record: &LockRecord) -> Result<()> {
    serde_json::to_writer_pretty(&mut *file, record)?;
    file.write_all(b"\n")?;
    file.flush()?;
    Ok(())
}

pub fn read(source: &Path, task_id: &str) -> Result<LockRecord> {
    let path = path(source, task_id);
    let body = fs::read_to_string(&path)
        .with_context(|| format!("task '{}' has no operation lock", task_id))?;
    if let Ok(record) = serde_json::from_str::<LockRecord>(&body) {
        if record.schema_version != 1 {
            return Err(anyhow!("task lock schema version is not supported"));
        }
        return Ok(record);
    }
    parse_legacy(&path, &body)
}

fn parse_legacy(path: &Path, body: &str) -> Result<LockRecord> {
    let mut operation = None;
    let mut parent_pid = None;
    for line in body.lines() {
        if let Some(value) = line.strip_prefix("owner=") {
            operation = Some(value.to_string());
        } else if let Some(value) = line.strip_prefix("pid=") {
            parent_pid = Some(value.parse()?);
        }
    }
    let created_at = fs::metadata(path)
        .and_then(|metadata| metadata.modified())
        .ok()
        .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|duration| duration.as_secs().to_string())
        .unwrap_or_else(task::timestamp);
    Ok(LockRecord {
        schema_version: 1,
        operation: operation.ok_or_else(|| anyhow!("legacy task lock omits owner"))?,
        parent_pid: parent_pid.ok_or_else(|| anyhow!("legacy task lock omits pid"))?,
        child_pid: None,
        created_at,
    })
}

impl TaskLock {
    pub fn set_child_pid(&mut self, pid: u32) -> Result<()> {
        self.record.child_pid = Some(pid);
        let mut file = OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(&self.path)?;
        write_record(&mut file, &self.record)
    }

    fn release(&mut self) {
        if self.active {
            let _ = fs::remove_file(&self.path);
            self.active = false;
        }
    }
}

impl Drop for TaskLock {
    fn drop(&mut self) {
        self.release();
    }
}

pub fn claim_stale(source: &Path, task_id: &str, expected: &LockRecord) -> Result<StaleLockClaim> {
    let original_path = path(source, task_id);
    if &read(source, task_id)? != expected {
        return Err(anyhow!(
            "task lock changed during recovery; inspect it again before retrying"
        ));
    }
    let stale_path = original_path.with_extension(format!("stale-{}", task::unique_id()));
    fs::rename(&original_path, &stale_path)
        .context("failed to claim the stale task lock for recovery")?;
    let lock = match acquire(source, task_id, "lock-recovery") {
        Ok(lock) => lock,
        Err(error) => {
            let _ = fs::rename(&stale_path, &original_path);
            return Err(error).context("failed to serialize stale-lock recovery");
        }
    };
    Ok(StaleLockClaim {
        lock,
        original_path,
        stale_path,
        finished: false,
    })
}

impl StaleLockClaim {
    pub fn finish(mut self) -> Result<()> {
        fs::remove_file(&self.stale_path).context("failed to retire stale task lock")?;
        self.finished = true;
        Ok(())
    }
}

impl Drop for StaleLockClaim {
    fn drop(&mut self) {
        if self.finished {
            return;
        }
        self.lock.release();
        if self.stale_path.exists() && !self.original_path.exists() {
            let _ = fs::rename(&self.stale_path, &self.original_path);
        }
    }
}

#[cfg(unix)]
pub fn process_alive(pid: u32) -> bool {
    // Unix process IDs are positive pid_t values. Values outside that range can
    // wrap to special negative kill(2) selectors on some platforms.
    if pid == 0 || pid > i32::MAX as u32 {
        return false;
    }
    let pid = pid.to_string();
    Command::new("kill")
        .args(["-0", &pid])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

#[cfg(all(test, unix))]
mod tests {
    use super::process_alive;

    #[test]
    fn invalid_unsigned_pid_is_not_treated_as_a_live_process() {
        assert!(!process_alive(0));
        assert!(!process_alive(u32::MAX));
    }
}

#[cfg(windows)]
pub fn process_alive(pid: u32) -> bool {
    let filter = format!("PID eq {pid}");
    let expected_pid = pid.to_string();
    Command::new("tasklist")
        .args(["/FI", &filter, "/FO", "CSV", "/NH"])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .is_some_and(|output| {
            String::from_utf8_lossy(&output.stdout).lines().any(|line| {
                line.split(',').nth(1).map(|value| value.trim_matches('"'))
                    == Some(expected_pid.as_str())
            })
        })
}
