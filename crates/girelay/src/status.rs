use crate::cli::StatusArgs;
use crate::{git, output, report, task, workspace_lock};
use anyhow::Result;
use serde::Serialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize)]
struct StatusOutput {
    schema_version: u32,
    repo: PathBuf,
    tasks: Vec<TaskStatus>,
}

#[derive(Debug, Serialize)]
struct TaskStatus {
    id: String,
    intent: String,
    branch: String,
    workspace: PathBuf,
    state: String,
    dirty: Option<bool>,
    changed_files: Vec<String>,
    active_session_id: Option<String>,
    latest_session_id: Option<String>,
    report_available: bool,
    merged_commit: Option<String>,
    blockers: Vec<String>,
}

pub fn status(args: StatusArgs) -> Result<()> {
    let source = git::source_repo(Path::new("."))?;
    let records = if let Some(id) = &args.task_id {
        vec![task::load(&source, id)?]
    } else {
        task::list(&source)?
    };
    let tasks: Vec<_> = records
        .into_iter()
        .map(|record| inspect(&source, record))
        .collect();
    let result = StatusOutput {
        schema_version: task::SCHEMA_VERSION,
        repo: source,
        tasks,
    };
    if args.json {
        return output::json(&result);
    }
    println!(
        "{:<20} {:<10} {:<6} {:<8} Branch",
        "Task", "State", "Dirty", "Report"
    );
    for row in result.tasks {
        println!(
            "{:<20} {:<10} {:<6} {:<8} {}",
            row.id,
            row.state,
            row.dirty
                .map(|x| if x { "yes" } else { "no" })
                .unwrap_or("n/a"),
            if row.report_available { "yes" } else { "no" },
            row.branch
        );
        for blocker in row.blockers {
            println!("  blocked: {blocker}");
        }
    }
    Ok(())
}

fn inspect(source: &Path, record: task::Task) -> TaskStatus {
    let exists = record.workspace_path.is_dir();
    let dirty = exists.then(|| git::is_dirty(&record.workspace_path).unwrap_or(true));
    let changed_files = if exists {
        git::working_tree_files(&record.workspace_path).unwrap_or_default()
    } else {
        Vec::new()
    };
    let mut blockers = Vec::new();
    if record.active_session_id.is_some() && !workspace_lock::path(source, &record.id).exists() {
        blockers.push("metadata says a session is active but its lock is missing".into());
    }
    let running = workspace_lock::path(source, &record.id).exists();
    let state = if record.lifecycle == task::TaskLifecycle::Cleaned && !exists {
        "cleaned"
    } else if !exists {
        "missing"
    } else if running {
        "running"
    } else if !blockers.is_empty() {
        "blocked"
    } else if record.merge.is_some() {
        "merged"
    } else if record.latest_session_id.is_some() {
        "paused"
    } else {
        "created"
    }
    .into();
    let report_available = record
        .latest_session_id
        .as_ref()
        .is_some_and(|id| report::report_file(source, &record.id, id).exists());
    TaskStatus {
        id: record.id,
        intent: record.intent,
        branch: record.branch,
        workspace: record.workspace_path,
        state,
        dirty,
        changed_files,
        active_session_id: record.active_session_id,
        latest_session_id: record.latest_session_id,
        report_available,
        merged_commit: record.merge.map(|m| m.source_after),
        blockers,
    }
}
