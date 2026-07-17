use anyhow::{Context, Result, anyhow};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub const SCHEMA_VERSION: u32 = 2;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum TaskLifecycle {
    Active,
    Merged,
    Cleaned,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MergeRecord {
    pub strategy: String,
    pub target_branch: String,
    pub source_before: String,
    pub source_after: String,
    pub task_tip: String,
    pub task_rollback_ref: String,
    pub source_rollback_ref: String,
    pub merged_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Task {
    pub schema_version: u32,
    pub id: String,
    pub intent: String,
    pub source_repo: PathBuf,
    pub workspace_path: PathBuf,
    pub base_branch: String,
    pub base_commit: String,
    pub branch: String,
    pub created_at: String,
    pub updated_at: String,
    pub lifecycle: TaskLifecycle,
    pub active_session_id: Option<String>,
    pub latest_session_id: Option<String>,
    pub merge: Option<MergeRecord>,
}

impl Task {
    pub fn new(
        id: String,
        intent: String,
        source_repo: PathBuf,
        workspace_path: PathBuf,
        base_branch: String,
        base_commit: String,
        branch: String,
    ) -> Self {
        let now = timestamp();
        Self {
            schema_version: SCHEMA_VERSION,
            id,
            intent,
            source_repo,
            workspace_path,
            base_branch,
            base_commit,
            branch,
            created_at: now.clone(),
            updated_at: now,
            lifecycle: TaskLifecycle::Active,
            active_session_id: None,
            latest_session_id: None,
            merge: None,
        }
    }
}

pub fn validate_task_id(id: &str) -> Result<()> {
    if id.is_empty()
        || !id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
    {
        return Err(anyhow!(
            "invalid task id '{id}'; use ASCII letters, digits, '.', '_' and '-'"
        ));
    }
    if id == "." || id == ".." || id.starts_with('.') || id.ends_with('.') {
        return Err(anyhow!(
            "invalid task id '{id}'; leading, trailing, and path-only dots are not allowed"
        ));
    }
    Ok(())
}

pub fn validate_intent(intent: &str) -> Result<()> {
    if intent.trim().is_empty() {
        return Err(anyhow!("task intent cannot be empty"));
    }
    Ok(())
}

pub fn task_file(source: &Path, id: &str) -> PathBuf {
    source.join(".girelay/tasks").join(format!("{id}.json"))
}

pub fn save(source: &Path, task: &Task) -> Result<()> {
    atomic_write(
        &task_file(source, &task.id),
        &serde_json::to_vec_pretty(task)?,
    )
}

pub fn load(source: &Path, id: &str) -> Result<Task> {
    let path = task_file(source, id);
    let body = fs::read(&path).with_context(|| format!("unknown girelay task '{id}'"))?;
    let task = parse(&body).with_context(|| format!("failed to parse {}", path.display()))?;
    if task.schema_version != SCHEMA_VERSION {
        return Err(anyhow!(
            "task '{id}' uses metadata schema {}; expected {}; recreate or migrate the task",
            task.schema_version,
            SCHEMA_VERSION
        ));
    }
    Ok(task)
}

pub fn list(source: &Path) -> Result<Vec<Task>> {
    let dir = source.join(".girelay/tasks");
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut tasks: Vec<Task> = Vec::new();
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if path.extension().and_then(|x| x.to_str()) == Some("json") {
            let task = parse(&fs::read(&path)?)
                .with_context(|| format!("failed to parse {}", path.display()))?;
            let expected_id = path.file_stem().and_then(|value| value.to_str());
            if task.schema_version != SCHEMA_VERSION || expected_id != Some(task.id.as_str()) {
                return Err(anyhow!(
                    "task registry identity or schema does not match {}",
                    path.display()
                ));
            }
            tasks.push(task);
        }
    }
    tasks.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(tasks)
}

fn parse(body: &[u8]) -> Result<Task> {
    let value: serde_json::Value = serde_json::from_slice(body)?;
    let object = value
        .as_object()
        .ok_or_else(|| anyhow!("task metadata must be a JSON object"))?;
    for field in [
        "schema_version",
        "id",
        "intent",
        "source_repo",
        "workspace_path",
        "base_branch",
        "base_commit",
        "branch",
        "created_at",
        "updated_at",
        "lifecycle",
        "active_session_id",
        "latest_session_id",
        "merge",
    ] {
        if !object.contains_key(field) {
            return Err(anyhow!("task metadata is missing required field '{field}'"));
        }
    }
    Ok(serde_json::from_value(value)?)
}

pub fn atomic_write(path: &Path, body: &[u8]) -> Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| anyhow!("path has no parent: {}", path.display()))?;
    fs::create_dir_all(parent)?;
    let name = path
        .file_name()
        .and_then(|x| x.to_str())
        .ok_or_else(|| anyhow!("invalid metadata path"))?;
    let temporary = parent.join(format!(".{name}.{}.tmp", unique_id()));
    fs::write(&temporary, body)?;
    if let Err(error) = fs::rename(&temporary, path) {
        let _ = fs::remove_file(&temporary);
        return Err(error)
            .with_context(|| format!("failed to atomically replace {}", path.display()));
    }
    Ok(())
}

pub fn timestamp() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        .to_string()
}

pub fn unique_id() -> String {
    format!(
        "{}-{}",
        timestamp(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .subsec_nanos()
    )
}
