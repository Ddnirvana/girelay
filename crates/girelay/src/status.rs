use crate::cli::StatusArgs;
use crate::{analysis, config, git, output, recover, report, session, task, workspace_lock};
use anyhow::Result;
use serde::Serialize;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize)]
struct StatusOutput {
    schema_version: u32,
    repo: PathBuf,
    tasks: Vec<TaskStatus>,
}

#[derive(Debug, Serialize)]
struct LatestSession {
    id: String,
    agent: String,
    state: String,
    exit_code: Option<i32>,
    started_at: String,
    finished_at: Option<String>,
}

#[derive(Debug, Serialize)]
struct LatestReport {
    available: bool,
    trust: String,
    summary: Option<String>,
}

#[derive(Debug, Serialize)]
struct TaskStatus {
    id: String,
    intent: String,
    intent_source: String,
    source_branch: String,
    branch: String,
    workspace: PathBuf,
    lifecycle: String,
    activity: String,
    workspace_present: bool,
    state: String,
    dirty: Option<bool>,
    changed_files: Vec<String>,
    active_session_id: Option<String>,
    active_session: Option<LatestSession>,
    latest_session_id: Option<String>,
    latest_session: Option<LatestSession>,
    report_available: bool,
    latest_report: LatestReport,
    merged_commit: Option<String>,
    recovery_points: usize,
    divergence: analysis::Divergence,
    overlaps: Vec<analysis::TaskOverlap>,
    warnings: Vec<analysis::Warning>,
    blockers: Vec<String>,
    next_action: String,
}

pub fn status(args: StatusArgs) -> Result<()> {
    let source = git::source_repo(Path::new("."))?;
    let all_records = task::list(&source)?;
    let mut changed = BTreeMap::new();
    for record in &all_records {
        changed.insert(
            record.id.clone(),
            analysis::task_changed_paths(&source, record)?,
        );
    }
    let overlaps = analysis::overlaps(&all_records, &changed);
    let cfg = config::load(&source)?;
    let records: Vec<_> = match &args.task_id {
        Some(id) => vec![task::load(&source, id)?],
        None => all_records,
    };
    let tasks: Vec<_> = records
        .into_iter()
        .map(|record| {
            let files = changed.get(&record.id);
            inspect(
                &source,
                record,
                files,
                &overlaps,
                cfg.merge
                    .run_checks
                    .then_some(cfg.merge.check_commands.len()),
            )
        })
        .collect::<Result<_>>()?;
    let result = StatusOutput {
        schema_version: task::SCHEMA_VERSION,
        repo: source,
        tasks,
    };
    if args.json {
        return output::json(&result);
    }
    if args.task_id.is_some() {
        print_detail(&result.tasks[0]);
    } else {
        print_dashboard(&result.tasks);
    }
    Ok(())
}

fn inspect(
    source: &Path,
    record: task::Task,
    changed_files: Option<&Vec<String>>,
    all_overlaps: &BTreeMap<String, Vec<analysis::TaskOverlap>>,
    configured_checks: Option<usize>,
) -> Result<TaskStatus> {
    let workspace_present = record.workspace_path.is_dir();
    let dirty = workspace_present
        .then(|| git::is_dirty(&record.workspace_path))
        .transpose()?;
    let changed_files = changed_files.cloned().unwrap_or_default();
    let mut blockers = Vec::new();
    let lock_present = workspace_lock::path(source, &record.id).exists();
    if record.active_session_id.is_some() && !lock_present {
        blockers.push("metadata says a session is active but its lock is missing".into());
    }
    let activity = if lock_present { "locked" } else { "idle" }.to_string();
    let state = if record.lifecycle == task::TaskLifecycle::Cleaned && !workspace_present {
        "cleaned"
    } else if !workspace_present {
        "missing"
    } else if lock_present {
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
    let latest = record
        .latest_session_id
        .as_ref()
        .map(|id| session::load(source, &record.id, id))
        .transpose()?;
    let active = record
        .active_session_id
        .as_ref()
        .map(|id| session::load(source, &record.id, id))
        .transpose()?;
    let report_value = record
        .latest_session_id
        .as_ref()
        .filter(|id| report::report_file(source, &record.id, id).exists())
        .map(|id| report::load(source, &record.id, id))
        .transpose()?;
    let latest_report = LatestReport {
        available: report_value.is_some(),
        trust: if report_value.is_some() {
            "reported-by-agent"
        } else {
            "not-reported"
        }
        .into(),
        summary: report_value.map(|value| value.summary),
    };
    let latest_session = latest.as_ref().map(|value| LatestSession {
        id: value.session_id.clone(),
        agent: value.agent.clone(),
        state: value.state.clone(),
        exit_code: value.exit_code,
        started_at: value.started_at.clone(),
        finished_at: value.finished_at.clone(),
    });
    let active_session = active.as_ref().map(|value| LatestSession {
        id: value.session_id.clone(),
        agent: value.agent.clone(),
        state: value.state.clone(),
        exit_code: value.exit_code,
        started_at: value.started_at.clone(),
        finished_at: value.finished_at.clone(),
    });
    let divergence = analysis::divergence(source, &record)?;
    let overlaps = all_overlaps.get(&record.id).cloned().unwrap_or_default();
    let mut warnings = analysis::overlap_warnings(&record.id, &overlaps);
    if let Some(warning) = analysis::divergence_warning(&record.id, &divergence) {
        warnings.push(warning);
    }
    if dirty == Some(true) {
        warnings.push(analysis::Warning::new(
            "dirty-task-state",
            "task worktree contains uncommitted changes",
            changed_files.clone(),
            "review the task diff before relay, merge, or cleanup",
        ));
    }
    if let Some(latest) = &latest {
        if matches!(
            latest.state.as_str(),
            "failed" | "failed-to-start" | "interrupted"
        ) {
            warnings.push(analysis::Warning::new(
                "latest-session-incomplete",
                format!("latest agent session ended as '{}'", latest.state),
                vec![format!("session_id={}", latest.session_id)],
                format!(
                    "review the task before continuing session {}",
                    latest.session_id
                ),
            ));
        }
        if !latest_report.available {
            warnings.push(analysis::Warning::new(
                "semantic-report-missing",
                "latest session did not submit semantic context",
                vec![
                    format!("session_id={}", latest.session_id),
                    "trust=not-reported".into(),
                ],
                "verify decisions, tests, and remaining work directly",
            ));
        }
    }
    if configured_checks.is_some_and(|count| count > 0) {
        warnings.push(analysis::Warning::new(
            "checks-pending",
            "configured checks run only during a real merge",
            vec![format!(
                "configured_checks={}",
                configured_checks.unwrap_or(0)
            )],
            format!("preview with `girelay merge {} --dry-run`", record.id),
        ));
    }
    if record.lifecycle == task::TaskLifecycle::Active {
        let conflicts = analysis::confirmed_conflicts(source, &record)?;
        if !conflicts.is_empty() {
            warnings.push(analysis::Warning::new(
                "confirmed-merge-conflict",
                "Git preflight found conflicts in committed task state",
                conflicts,
                "resolve source/task divergence before merging",
            ));
        }
    }
    let next_action = next_action(&record, workspace_present, lock_present, &blockers);
    Ok(TaskStatus {
        id: record.id.clone(),
        intent: record.intent,
        intent_source: match record.intent_source {
            task::IntentSource::Explicit => "explicit",
            task::IntentSource::TaskId => "task-id",
        }
        .into(),
        source_branch: record.base_branch,
        branch: record.branch,
        workspace: record.workspace_path,
        lifecycle: lifecycle_name(&record.lifecycle).into(),
        activity,
        workspace_present,
        state,
        dirty,
        changed_files,
        active_session_id: record.active_session_id,
        active_session,
        latest_session_id: record.latest_session_id,
        latest_session,
        report_available: latest_report.available,
        latest_report,
        merged_commit: record.merge.map(|merge| merge.source_after),
        recovery_points: recover::count_for_task(source, &record.id),
        divergence,
        overlaps,
        warnings,
        blockers,
        next_action,
    })
}

fn next_action(
    record: &task::Task,
    workspace_present: bool,
    lock_present: bool,
    blockers: &[String],
) -> String {
    if lock_present {
        return format!("girelay recover unlock {}", record.id);
    }
    if !blockers.is_empty() || !workspace_present {
        return format!("girelay recover list {}", record.id);
    }
    if record.merge.is_some() {
        return format!("girelay clean {} --dry-run", record.id);
    }
    if record.latest_session_id.is_some() {
        return format!("girelay merge {} --dry-run", record.id);
    }
    format!("girelay relay {} -- <agent>", record.id)
}

fn lifecycle_name(value: &task::TaskLifecycle) -> &'static str {
    match value {
        task::TaskLifecycle::Active => "active",
        task::TaskLifecycle::Merged => "merged",
        task::TaskLifecycle::Cleaned => "cleaned",
    }
}

fn print_dashboard(tasks: &[TaskStatus]) {
    println!(
        "{:<20} {:<10} {:<6} {:<8} {:<8} Branch",
        "Task", "State", "Dirty", "Report", "Warnings"
    );
    for row in tasks {
        println!(
            "{:<20} {:<10} {:<6} {:<8} {:<8} {}",
            row.id,
            row.state,
            row.dirty
                .map(|dirty| if dirty { "yes" } else { "no" })
                .unwrap_or("n/a"),
            if row.report_available { "yes" } else { "no" },
            row.warnings.len(),
            row.branch
        );
        for overlap in &row.overlaps {
            println!(
                "  overlaps {}: {}",
                overlap.task_id,
                overlap.paths.join(", ")
            );
        }
        for blocker in &row.blockers {
            println!("  blocked: {blocker}");
        }
    }
}

fn print_detail(row: &TaskStatus) {
    println!("Task: {}", row.id);
    println!("Intent: {} ({})", row.intent, row.intent_source);
    println!("State: {}", row.state);
    println!("Lifecycle: {}", row.lifecycle);
    println!("Activity: {}", row.activity);
    println!("Source branch: {}", row.source_branch);
    println!("Task branch: {}", row.branch);
    println!("Workspace: {}", row.workspace.display());
    println!(
        "Workspace present: {}",
        if row.workspace_present { "yes" } else { "no" }
    );
    println!(
        "Dirty: {}",
        row.dirty
            .map(|value| if value { "yes" } else { "no" })
            .unwrap_or("n/a")
    );
    println!("Changed files: {}", row.changed_files.len());
    for path in &row.changed_files {
        println!("  {path}");
    }
    if let Some(active) = &row.active_session {
        println!("Active session: {}", active.id);
        println!("Active agent: {}", active.agent);
        println!("Active result: {}", active.state);
        println!("Active started: {}", active.started_at);
    } else {
        println!("Active session: none");
    }
    if let Some(latest) = &row.latest_session {
        println!("Latest session: {}", latest.id);
        println!("Latest agent: {}", latest.agent);
        println!("Latest result: {}", latest.state);
    } else {
        println!("Latest session: none");
    }
    println!("Semantic report: {}", row.latest_report.trust);
    if let Some(summary) = &row.latest_report.summary {
        println!("Agent-reported summary: {summary}");
    }
    println!("Source state: {}", row.divergence.source_state);
    println!(
        "Task relation to source: {}",
        row.divergence.task_relation_to_source
    );
    println!("Recovery points: {}", row.recovery_points);
    for warning in &row.warnings {
        println!("Warning [{}]: {}", warning.code, warning.message);
        for evidence in &warning.evidence {
            println!("  Evidence: {evidence}");
        }
        println!("  Next: {}", warning.next_action);
    }
    for blocker in &row.blockers {
        println!("Blocked: {blocker}");
    }
    println!("Next: {}", row.next_action);
}
