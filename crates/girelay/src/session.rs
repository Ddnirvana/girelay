use crate::cli::RelayArgs;
use crate::{errors, git, task, workspace_lock};
use anyhow::{Context, Result, anyhow};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SessionRecord {
    pub schema_version: u32,
    pub task_id: String,
    pub session_id: String,
    pub agent: String,
    pub command: Vec<String>,
    pub started_at: String,
    pub finished_at: Option<String>,
    pub exit_code: Option<i32>,
    pub state: String,
    pub start_snapshot: String,
    pub end_snapshot: Option<String>,
    pub changed_files: Vec<String>,
    pub report_path: Option<PathBuf>,
    pub trust: SessionTrust,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SessionTrust {
    pub git_state: String,
    pub process_result: String,
    pub semantic_report: String,
}

pub fn relay(args: RelayArgs) -> Result<()> {
    task::validate_task_id(&args.task_id)?;
    let source = git::source_repo(Path::new("."))?;
    let record = task::load(&source, &args.task_id)?;
    run_session(&source, record, args.command, args.recover_stale_session)
}

pub fn run_session(
    source: &Path,
    mut record: task::Task,
    command: Vec<String>,
    recover_stale: bool,
) -> Result<()> {
    if record.merge.is_some() {
        return Err(anyhow!(
            "task '{}' is already merged; start a new task to make more changes",
            record.id
        ));
    }
    if !record.workspace_path.is_dir() {
        return Err(anyhow!(
            "task '{}' worktree is missing at {}; use `girelay recover list {}`",
            record.id,
            record.workspace_path.display(),
            record.id
        ));
    }
    verify_worktree(&record)?;
    let executable = command
        .first()
        .ok_or_else(|| anyhow!("missing agent command after '--'"))?;
    let _lock = workspace_lock::acquire(source, &record.id, recover_stale, "agent-session")?;
    if recover_stale {
        if let Some(interrupted) = close_interrupted_session(source, &record)? {
            record.active_session_id = None;
            record.latest_session_id = Some(interrupted);
        }
    }
    let session_id = task::unique_id();
    let start_snapshot = snapshot(source, &record, &session_id, "start")?;
    let session_path = session_file(source, &record.id, &session_id);
    let previous_report = record
        .latest_session_id
        .as_ref()
        .map(|id| crate::report::report_file(source, &record.id, id))
        .filter(|path| path.exists());
    let mut session = SessionRecord {
        schema_version: task::SCHEMA_VERSION,
        task_id: record.id.clone(),
        session_id: session_id.clone(),
        agent: agent_name(executable),
        command: sanitize_command(&command),
        started_at: task::timestamp(),
        finished_at: None,
        exit_code: None,
        state: "running".into(),
        start_snapshot,
        end_snapshot: None,
        changed_files: Vec::new(),
        report_path: None,
        trust: SessionTrust {
            git_state: "observed-by-girelay".into(),
            process_result: "observed-by-girelay".into(),
            semantic_report: "not-reported".into(),
        },
    };
    record.active_session_id = Some(session_id.clone());
    record.updated_at = task::timestamp();
    save_session(&session_path, &session)?;
    task::save(source, &record)?;

    println!("Relaying task {} to {}", record.id, session.agent);
    println!("Workspace: {}", record.workspace_path.display());
    let mut child = Command::new(executable);
    child
        .args(&command[1..])
        .current_dir(&record.workspace_path)
        .env("GIRELAY_TASK_ID", &record.id)
        .env("GIRELAY_SESSION_ID", &session_id)
        .env("GIRELAY_INTENT", &record.intent)
        .env("GIRELAY_SOURCE_REPO", source)
        .env("GIRELAY_START_SNAPSHOT", &session.start_snapshot)
        .env(
            "GIRELAY_REPORT_COMMAND",
            format!("girelay report --session {session_id} --file"),
        )
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    if let Some(path) = &previous_report {
        child.env("GIRELAY_PREVIOUS_REPORT", path);
    }
    let status = child.status();

    let (status, launch_error) = match status {
        Ok(status) => (Some(status), None),
        Err(error) => (None, Some(error)),
    };
    session.finished_at = Some(task::timestamp());
    session.exit_code = Some(status.as_ref().map(exit_code).unwrap_or(127));
    session.state = match (&status, &launch_error) {
        (_, Some(_)) => "failed-to-start",
        (Some(status), None) if status.success() => "completed",
        _ => "failed",
    }
    .into();
    session.end_snapshot = Some(snapshot(source, &record, &session_id, "end")?);
    session.changed_files = changed_files(&record.workspace_path)?;
    let report_path = crate::report::report_file(source, &record.id, &session_id);
    if report_path.exists() {
        session.report_path = Some(report_path);
        session.trust.semantic_report = "reported-by-agent".into();
    }
    save_session(&session_path, &session)?;
    record.active_session_id = None;
    record.latest_session_id = Some(session_id.clone());
    record.updated_at = task::timestamp();
    task::save(source, &record)?;
    println!("Session: {} ({})", session_id, session.state);
    println!("Changed files: {}", session.changed_files.len());
    if session.report_path.is_none() {
        println!(
            "Semantic report: not reported (Git state and process result were still captured)"
        );
    }
    if let Some(error) = launch_error {
        return Err(anyhow!(errors::ChildExit {
            code: 127,
            message: format!("failed to start agent command '{executable}': {error}")
        }));
    }
    let status = status.expect("status exists without launch error");
    if !status.success() {
        let code = exit_code(&status);
        return Err(anyhow!(errors::ChildExit {
            code,
            message: format!("agent command exited with code {code}; task state was preserved")
        }));
    }
    Ok(())
}

pub fn session_file(source: &Path, task_id: &str, session_id: &str) -> PathBuf {
    source
        .join(".girelay/sessions")
        .join(task_id)
        .join(format!("{session_id}.json"))
}

pub fn load(source: &Path, task_id: &str, session_id: &str) -> Result<SessionRecord> {
    let path = session_file(source, task_id, session_id);
    let body = fs::read(&path).with_context(|| format!("unknown session '{session_id}'"))?;
    parse_session(&body).with_context(|| format!("failed to parse {}", path.display()))
}

fn save_session(path: &Path, session: &SessionRecord) -> Result<()> {
    task::atomic_write(path, &serde_json::to_vec_pretty(session)?)
}

pub(crate) fn close_interrupted_session(
    source: &Path,
    record: &task::Task,
) -> Result<Option<String>> {
    let session_id = match &record.active_session_id {
        Some(session_id) => Some(session_id.clone()),
        None => discover_orphaned_running_session(source, &record.id)?,
    };
    let Some(session_id) = session_id else {
        return Ok(None);
    };
    let mut previous = load(source, &record.id, &session_id)?;
    if previous.state != "running" {
        return Err(anyhow!(
            "task metadata points to session '{}' but that session is not running",
            session_id
        ));
    }
    previous.finished_at = Some(task::timestamp());
    previous.state = "interrupted".into();
    previous.end_snapshot = Some(snapshot(source, record, &session_id, "interrupted")?);
    previous.changed_files = changed_files(&record.workspace_path)?;
    save_session(&session_file(source, &record.id, &session_id), &previous)?;
    Ok(Some(session_id))
}

fn discover_orphaned_running_session(source: &Path, task_id: &str) -> Result<Option<String>> {
    let directory = source.join(".girelay/sessions").join(task_id);
    if !directory.exists() {
        return Ok(None);
    }
    let mut running = Vec::new();
    for entry in fs::read_dir(directory)? {
        let path = entry?.path();
        if path.extension().and_then(|value| value.to_str()) != Some("json") {
            continue;
        }
        let candidate = parse_session(&fs::read(&path)?)?;
        if candidate.state == "running" {
            running.push(candidate.session_id);
        }
    }
    match running.len() {
        0 => Ok(None),
        1 => Ok(running.pop()),
        _ => Err(anyhow!(
            "task '{}' has multiple running session records; refusing ambiguous stale recovery",
            task_id
        )),
    }
}

fn parse_session(body: &[u8]) -> Result<SessionRecord> {
    let value: serde_json::Value = serde_json::from_slice(body)?;
    let object = value
        .as_object()
        .ok_or_else(|| anyhow!("session metadata must be a JSON object"))?;
    for field in [
        "schema_version",
        "task_id",
        "session_id",
        "agent",
        "command",
        "started_at",
        "finished_at",
        "exit_code",
        "state",
        "start_snapshot",
        "end_snapshot",
        "changed_files",
        "report_path",
        "trust",
    ] {
        if !object.contains_key(field) {
            return Err(anyhow!(
                "session metadata is missing required field '{field}'"
            ));
        }
    }
    let session: SessionRecord = serde_json::from_value(value)?;
    if session.schema_version != task::SCHEMA_VERSION {
        return Err(anyhow!("session metadata schema version is not supported"));
    }
    Ok(session)
}

fn verify_worktree(record: &task::Task) -> Result<()> {
    let branch = git::current_branch(&record.workspace_path)?;
    let source = git::source_repo(&record.workspace_path)?;
    if source != fs::canonicalize(&record.source_repo)? || branch != record.branch {
        return Err(anyhow!(
            "task '{}' worktree ownership does not match its metadata",
            record.id
        ));
    }
    Ok(())
}

pub(crate) fn snapshot(
    source: &Path,
    record: &task::Task,
    session_id: &str,
    phase: &str,
) -> Result<String> {
    let index = source
        .join(".girelay/tmp")
        .join(format!("index-{session_id}-{phase}"));
    let _ = fs::remove_file(&index);
    let run = |args: &[&str]| -> Result<String> {
        let output = Command::new("git")
            .args(args)
            .current_dir(&record.workspace_path)
            .env("GIT_INDEX_FILE", &index)
            .env("GIT_AUTHOR_NAME", "girelay")
            .env("GIT_AUTHOR_EMAIL", "noreply@girelay.dev")
            .env("GIT_COMMITTER_NAME", "girelay")
            .env("GIT_COMMITTER_EMAIL", "noreply@girelay.dev")
            .output()?;
        if !output.status.success() {
            return Err(anyhow!(
                "git {} failed while capturing snapshot: {}",
                args.join(" "),
                String::from_utf8_lossy(&output.stderr).trim()
            ));
        }
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    };
    let result = (|| {
        run(&["read-tree", "HEAD"])?;
        run(&["add", "-A"])?;
        let tree = run(&["write-tree"])?;
        let head = git::head_commit(&record.workspace_path, "HEAD")?;
        let commit = run(&[
            "commit-tree",
            &tree,
            "-p",
            &head,
            "-m",
            &format!("girelay {phase} snapshot for {}", record.id),
        ])?;
        let reference = format!("refs/girelay/snapshots/{}/{session_id}/{phase}", record.id);
        git::update_ref(source, &reference, &commit)?;
        Ok(commit)
    })();
    let _ = fs::remove_file(&index);
    result
}

fn changed_files(repo: &Path) -> Result<Vec<String>> {
    git::working_tree_files(repo)
}

fn sanitize_command(command: &[String]) -> Vec<String> {
    let mut redact = false;
    command
        .iter()
        .map(|arg| {
            if redact {
                redact = false;
                return "[REDACTED]".into();
            }
            let lower = arg.to_ascii_lowercase();
            if ["--api-key", "--token", "--password", "--secret", "-p"].contains(&lower.as_str()) {
                redact = true;
                return arg.clone();
            }
            for prefix in ["--api-key=", "--token=", "--password=", "--secret="] {
                if lower.starts_with(prefix) {
                    return format!("{}[REDACTED]", &arg[..prefix.len()]);
                }
            }
            arg.clone()
        })
        .collect()
}

fn agent_name(command: &str) -> String {
    Path::new(command)
        .file_name()
        .and_then(|x| x.to_str())
        .unwrap_or(command)
        .to_string()
}

fn exit_code(status: &ExitStatus) -> i32 {
    if let Some(code) = status.code() {
        return code;
    }
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        128 + status.signal().unwrap_or(2)
    }
    #[cfg(not(unix))]
    {
        1
    }
}
