use crate::cli::ReportArgs;
use crate::{git, session, task};
use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentReport {
    pub schema_version: u32,
    pub task_id: String,
    pub session_id: String,
    pub agent: String,
    pub start_snapshot: String,
    pub end_snapshot: Option<String>,
    pub summary: String,
    pub completed: Vec<String>,
    pub remaining: Vec<String>,
    pub decisions: Vec<String>,
    pub failed_attempts: Vec<String>,
    pub blockers: Vec<String>,
    pub tests: Vec<String>,
    pub risks: Vec<String>,
    pub next_action: String,
    pub trust: String,
}

pub fn report(args: ReportArgs) -> Result<()> {
    let source = git::source_repo(Path::new("."))?;
    let body = fs::read(&args.file)?;
    if body.len() > 1024 * 1024 {
        return Err(anyhow!("semantic report exceeds the 1 MiB limit"));
    }
    let value: serde_json::Value = serde_json::from_slice(&body)?;
    require_fields(
        &value,
        &[
            "schema_version",
            "task_id",
            "session_id",
            "agent",
            "start_snapshot",
            "end_snapshot",
            "summary",
            "completed",
            "remaining",
            "decisions",
            "failed_attempts",
            "blockers",
            "tests",
            "risks",
            "next_action",
            "trust",
        ],
    )?;
    let report: AgentReport = serde_json::from_value(value)?;
    if report.schema_version != task::SCHEMA_VERSION {
        return Err(anyhow!(
            "report schema_version must be {}",
            task::SCHEMA_VERSION
        ));
    }
    if report.session_id != args.session {
        return Err(anyhow!("report session_id does not match --session"));
    }
    let record = task::load(&source, &report.task_id)?;
    if record.active_session_id.as_deref() != Some(&args.session) {
        return Err(anyhow!(
            "session '{}' is not the active session for task '{}'",
            args.session,
            report.task_id
        ));
    }
    let observed = session::load(&source, &report.task_id, &args.session)?;
    if report.start_snapshot != observed.start_snapshot {
        return Err(anyhow!(
            "report start_snapshot does not match the observed session snapshot"
        ));
    }
    if report.agent != observed.agent {
        return Err(anyhow!(
            "report agent does not match the observed session command"
        ));
    }
    if report.trust != "reported-by-agent" {
        return Err(anyhow!("report trust must be 'reported-by-agent'"));
    }
    if report.end_snapshot.is_some() {
        return Err(anyhow!(
            "report end_snapshot must be null; girelay captures it after the agent exits"
        ));
    }
    if report.summary.trim().is_empty() || report.next_action.trim().is_empty() {
        return Err(anyhow!("report summary and next_action must not be empty"));
    }
    let destination = report_file(&source, &report.task_id, &args.session);
    if destination.exists() {
        return Err(anyhow!(
            "session report already exists; reports are immutable"
        ));
    }
    task::atomic_write(&destination, &serde_json::to_vec_pretty(&report)?)?;
    println!("Recorded semantic report for session {}", args.session);
    Ok(())
}

fn require_fields(value: &serde_json::Value, fields: &[&str]) -> Result<()> {
    let object = value
        .as_object()
        .ok_or_else(|| anyhow!("semantic report must be a JSON object"))?;
    for field in fields {
        if !object.contains_key(*field) {
            return Err(anyhow!(
                "semantic report is missing required field '{field}'"
            ));
        }
    }
    Ok(())
}

pub fn report_file(source: &Path, task_id: &str, session_id: &str) -> PathBuf {
    source
        .join(".girelay/reports")
        .join(task_id)
        .join(format!("{session_id}.json"))
}
