use crate::cli::{MergeArgs, MergeStrategy};
use crate::{analysis, config, git, output, report, session, task, workspace_lock};
use anyhow::{Context, Result, anyhow};
use serde::Serialize;
use std::collections::BTreeMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

#[derive(Debug, Clone, Serialize)]
struct PlannedCheck {
    command: String,
    state: String,
}

#[derive(Debug, Serialize)]
struct MergePlan {
    schema_version: u32,
    kind: String,
    task_id: String,
    strategy: String,
    target_branch: String,
    source_before: String,
    task_before: String,
    proposed_message: String,
    message_source: String,
    dirty: bool,
    final_task_commit_required: bool,
    changed_files: Vec<String>,
    commits: Vec<analysis::CommitSummary>,
    checks: Vec<PlannedCheck>,
    divergence: analysis::Divergence,
    overlaps: Vec<analysis::TaskOverlap>,
    confirmed_conflicts: Vec<String>,
    warnings: Vec<analysis::Warning>,
    task_rollback_ref: String,
    source_rollback_ref: String,
}

#[derive(Debug, Serialize)]
struct MergeOutput {
    schema_version: u32,
    task_id: String,
    strategy: String,
    target_branch: String,
    source_before: String,
    source_after: String,
    task_tip: String,
    message: String,
    changed_files: Vec<String>,
    checks: Vec<String>,
    warnings: Vec<analysis::Warning>,
    task_rollback_ref: String,
    source_rollback_ref: String,
}

pub fn merge(args: MergeArgs) -> Result<()> {
    task::validate_task_id(&args.task_id)?;
    let cwd = git::repo_root(Path::new("."))?;
    let source = git::source_repo(&cwd)?;
    if std::fs::canonicalize(&cwd)? != source {
        return Err(anyhow!("girelay merge must run in the source checkout"));
    }
    let record = task::load(&source, &args.task_id)?;
    if args.dry_run {
        let plan = build_plan(&source, &record, &args, true)?;
        return if args.json {
            output::json(&plan)
        } else {
            print_plan(&plan);
            Ok(())
        };
    }

    let _lock = workspace_lock::acquire(&source, &record.id, "merge")?;
    let plan = build_plan(&source, &record, &args, false)?;
    apply(&source, record, &args, plan)
}

fn build_plan(
    source: &Path,
    record: &task::Task,
    args: &MergeArgs,
    include_lock_warning: bool,
) -> Result<MergePlan> {
    if record.merge.is_some() {
        return Err(anyhow!("task '{}' already has a merge record", record.id));
    }
    if !record.workspace_path.is_dir() {
        return Err(anyhow!("task '{}' worktree is missing", record.id));
    }
    if git::current_branch(source)? != record.base_branch {
        return Err(anyhow!(
            "source checkout must have target branch '{}' checked out",
            record.base_branch
        ));
    }
    if git::is_dirty(source)? {
        return Err(anyhow!(
            "source checkout is dirty; commit or stash changes before merge"
        ));
    }
    if git::current_branch(&record.workspace_path)? != record.branch {
        return Err(anyhow!(
            "task worktree is not on recorded branch '{}'",
            record.branch
        ));
    }
    let source_before = git::head_commit(source, "HEAD")?;
    let task_before = git::head_commit(&record.workspace_path, "HEAD")?;
    let dirty = git::is_dirty(&record.workspace_path)?;
    let changed_files = analysis::task_changed_paths(source, record)?;
    if changed_files.is_empty() {
        return Err(anyhow!("task has no changes relative to its recorded base"));
    }
    let cfg = config::load(source)?;
    let checks_enabled = cfg.merge.run_checks && !args.no_checks;
    let checks: Vec<_> = cfg
        .merge
        .check_commands
        .iter()
        .map(|command| PlannedCheck {
            command: command.clone(),
            state: if checks_enabled { "pending" } else { "skipped" }.into(),
        })
        .collect();
    let divergence = analysis::divergence(source, record)?;
    let overlaps = task_overlaps(source, &record.id)?;
    let confirmed_conflicts = analysis::confirmed_conflicts(source, record)?;
    let (proposed_message, message_source) = merge_message(record, args.message.as_deref())?;
    let mut warnings = analysis::overlap_warnings(&record.id, &overlaps);
    if let Some(warning) = analysis::divergence_warning(&record.id, &divergence) {
        warnings.push(warning);
    }
    if dirty {
        warnings.push(analysis::Warning::new(
            "dirty-task-state",
            "uncommitted task work will become a final task commit",
            changed_files.clone(),
            "review the task worktree diff before merging",
        ));
    }
    if let Some(session_id) = &record.latest_session_id {
        let latest = session::load(source, &record.id, session_id)?;
        if matches!(
            latest.state.as_str(),
            "failed" | "failed-to-start" | "interrupted"
        ) {
            warnings.push(analysis::Warning::new(
                "latest-session-incomplete",
                format!("latest agent session ended as '{}'", latest.state),
                vec![format!("session_id={session_id}")],
                format!("review `girelay status {}` before integration", record.id),
            ));
        }
        if !report::report_file(source, &record.id, session_id).exists() {
            warnings.push(analysis::Warning::new(
                "semantic-report-missing",
                "latest session did not submit semantic context",
                vec![
                    format!("session_id={session_id}"),
                    "trust=not-reported".into(),
                ],
                "verify decisions, tests, and remaining work directly",
            ));
        }
    }
    if checks_enabled && !checks.is_empty() {
        warnings.push(analysis::Warning::new(
            "checks-pending",
            format!(
                "{} configured check(s) have not run in preview",
                checks.len()
            ),
            checks.iter().map(|check| check.command.clone()).collect(),
            "run the real merge to execute checks before integration",
        ));
    }
    if !confirmed_conflicts.is_empty() {
        warnings.push(analysis::Warning::new(
            "confirmed-merge-conflict",
            "Git preflight found conflicts in committed task state",
            confirmed_conflicts.clone(),
            "resolve source/task divergence before merging",
        ));
    }
    if include_lock_warning && workspace_lock::path(source, &record.id).exists() {
        warnings.push(analysis::Warning::new(
            "task-locked",
            "another task operation currently owns the workspace",
            vec![format!(
                "lock={}",
                workspace_lock::path(source, &record.id).display()
            )],
            format!(
                "inspect `girelay recover unlock {}` before merging",
                record.id
            ),
        ));
    }
    let strategy = strategy_name(args.strategy);
    Ok(MergePlan {
        schema_version: task::SCHEMA_VERSION,
        kind: "merge-plan".into(),
        task_id: record.id.clone(),
        strategy: strategy.into(),
        target_branch: record.base_branch.clone(),
        source_before,
        task_before,
        proposed_message,
        message_source,
        dirty,
        final_task_commit_required: dirty,
        changed_files,
        commits: analysis::commit_summaries(source, record)?,
        checks,
        divergence,
        overlaps,
        confirmed_conflicts,
        warnings,
        task_rollback_ref: format!("refs/girelay/rollback/task/{}/<merge-id>", record.id),
        source_rollback_ref: format!("refs/girelay/rollback/source/{}/<merge-id>", record.id),
    })
}

fn apply(source: &Path, mut record: task::Task, args: &MergeArgs, plan: MergePlan) -> Result<()> {
    git::ensure_commit_identity(source)?;
    let task_state_before = git::working_tree_state(&record.workspace_path)?;
    let merge_id = task::unique_id();
    let _snapshot = session::snapshot(source, &record, &format!("merge-{merge_id}"), "pre-merge")?;
    let checks: Vec<_> = plan
        .checks
        .iter()
        .filter(|check| check.state == "pending")
        .map(|check| check.command.clone())
        .collect();
    for check in &checks {
        run_check(&record.workspace_path, check, args.json)?;
    }
    revalidate_task(
        &record,
        &plan.task_before,
        &task_state_before,
        "configured checks",
    )?;
    revalidate_source(source, &record, &plan.source_before)?;

    let task_rollback_ref = format!("refs/girelay/rollback/task/{}/{}", record.id, merge_id);
    git::update_ref(source, &task_rollback_ref, &plan.task_before)?;
    if plan.final_task_commit_required {
        git::run_quiet(&record.workspace_path, &["add", "-A"])?;
        git::run_quiet(
            &record.workspace_path,
            &["commit", "-m", &plan.proposed_message],
        )
        .context("failed to create the final task commit; task rollback ref was preserved")?;
        if git::is_dirty(&record.workspace_path)? {
            return Err(anyhow!(
                "task worktree is still dirty after final commit; source was not changed"
            ));
        }
    }
    let task_tip = git::head_commit(&record.workspace_path, "HEAD")?;
    let changed_files = git::changed_files(source, &plan.source_before, &task_tip)?;
    if changed_files.is_empty() {
        return Err(anyhow!("task has no changes relative to the source branch"));
    }
    revalidate_source(source, &record, &plan.source_before)?;
    let source_rollback_ref = format!("refs/girelay/rollback/source/{}/{}", record.id, merge_id);
    git::update_ref(source, &source_rollback_ref, &plan.source_before)?;
    let apply = match args.strategy {
        MergeStrategy::Squash => git::run_quiet(source, &["merge", "--squash", &record.branch])
            .and_then(|_| git::run_quiet(source, &["commit", "-m", &plan.proposed_message])),
        MergeStrategy::Preserve => git::run_quiet(
            source,
            &[
                "merge",
                "--no-ff",
                &record.branch,
                "-m",
                &plan.proposed_message,
            ],
        ),
    };
    if let Err(error) = apply {
        if let Err(rollback) = rollback_source(source, &plan.source_before) {
            return Err(anyhow!(
                "merge failed: {error:#}; automatic source rollback also failed: {rollback:#}"
            ));
        }
        return Err(error)
            .context("merge failed; source checkout was restored and rollback refs were retained");
    }
    let source_after = git::head_commit(source, "HEAD")?;
    record.lifecycle = task::TaskLifecycle::Merged;
    record.updated_at = task::timestamp();
    record.merge = Some(task::MergeRecord {
        strategy: plan.strategy.clone(),
        target_branch: record.base_branch.clone(),
        source_before: plan.source_before.clone(),
        source_after: source_after.clone(),
        task_tip: task_tip.clone(),
        task_rollback_ref: task_rollback_ref.clone(),
        source_rollback_ref: source_rollback_ref.clone(),
        merged_at: task::timestamp(),
    });
    if let Err(error) = task::save(source, &record) {
        rollback_source(source, &plan.source_before)?;
        return Err(error)
            .context("failed to publish merge record; source checkout was rolled back");
    }
    let result = MergeOutput {
        schema_version: task::SCHEMA_VERSION,
        task_id: record.id,
        strategy: plan.strategy,
        target_branch: record.base_branch,
        source_before: plan.source_before,
        source_after,
        task_tip,
        message: plan.proposed_message,
        changed_files,
        checks,
        warnings: plan.warnings,
        task_rollback_ref,
        source_rollback_ref,
    };
    if args.json {
        output::json(&result)
    } else {
        println!(
            "Merged task {} with {} strategy",
            result.task_id, result.strategy
        );
        println!("Source commit: {}", result.source_after);
        println!("Rollback: {}", result.source_rollback_ref);
        Ok(())
    }
}

fn merge_message(record: &task::Task, provided: Option<&str>) -> Result<(String, String)> {
    if let Some(message) = provided {
        if message.trim().is_empty() {
            return Err(anyhow!("merge message cannot be empty"));
        }
        return Ok((message.into(), "argument".into()));
    }
    if record.intent_source == task::IntentSource::Explicit {
        return Ok((record.intent.clone(), "intent".into()));
    }
    Ok((format!("agent: complete {}", record.id), "task-id".into()))
}

fn task_overlaps(source: &Path, task_id: &str) -> Result<Vec<analysis::TaskOverlap>> {
    let records = task::list(source)?;
    let mut changed = BTreeMap::new();
    for record in &records {
        changed.insert(
            record.id.clone(),
            analysis::task_changed_paths(source, record)?,
        );
    }
    Ok(analysis::overlaps(&records, &changed)
        .remove(task_id)
        .unwrap_or_default())
}

fn strategy_name(strategy: MergeStrategy) -> &'static str {
    match strategy {
        MergeStrategy::Squash => "squash",
        MergeStrategy::Preserve => "preserve",
    }
}

fn print_plan(plan: &MergePlan) {
    println!("Merge preview for {}", plan.task_id);
    println!("Target: {}", plan.target_branch);
    println!("Strategy: {}", plan.strategy);
    println!(
        "Message: {} ({})",
        plan.proposed_message, plan.message_source
    );
    println!("Task commits: {}", plan.commits.len());
    println!("Changed files: {}", plan.changed_files.len());
    println!(
        "Dirty task state: {}",
        if plan.dirty { "yes" } else { "no" }
    );
    println!("Source state: {}", plan.divergence.source_state);
    println!("Checks:");
    if plan.checks.is_empty() {
        println!("  none configured");
    } else {
        for check in &plan.checks {
            println!("  {}: {}", check.state, check.command);
        }
    }
    for warning in &plan.warnings {
        println!("Warning [{}]: {}", warning.code, warning.message);
        println!("  Next: {}", warning.next_action);
    }
    println!("Planned task rollback: {}", plan.task_rollback_ref);
    println!("Planned source rollback: {}", plan.source_rollback_ref);
    println!("No files, refs, commits, locks, or metadata were changed.");
}

fn revalidate_task(
    record: &task::Task,
    expected_head: &str,
    expected_state: &[u8],
    boundary: &str,
) -> Result<()> {
    if git::source_repo(&record.workspace_path)? != record.source_repo
        || git::current_branch(&record.workspace_path)? != record.branch
        || git::head_commit(&record.workspace_path, "HEAD")? != expected_head
        || git::working_tree_state(&record.workspace_path)? != expected_state
    {
        return Err(anyhow!(
            "task worktree changed during {boundary}; source was not changed"
        ));
    }
    Ok(())
}

fn rollback_source(source: &Path, commit: &str) -> Result<()> {
    let _ = git::run_quiet(source, &["merge", "--abort"]);
    git::run_quiet(source, &["reset", "--hard", commit])?;
    git::run_quiet(source, &["clean", "-fd"])
}

fn revalidate_source(source: &Path, record: &task::Task, expected_head: &str) -> Result<()> {
    if git::current_branch(source)? != record.base_branch
        || git::head_commit(source, "HEAD")? != expected_head
        || git::is_dirty(source)?
    {
        return Err(anyhow!(
            "source checkout changed while merge was being prepared; nothing was merged"
        ));
    }
    Ok(())
}

fn run_check(workspace: &PathBuf, check: &str, json: bool) -> Result<()> {
    let mut command = Command::new("sh");
    command
        .args(["-c", check])
        .current_dir(workspace)
        .stdin(Stdio::null());
    let status = if json {
        let output = command
            .output()
            .with_context(|| format!("failed to start check `{check}`"))?;
        let mut stderr = std::io::stderr().lock();
        stderr.write_all(&output.stdout)?;
        stderr.write_all(&output.stderr)?;
        stderr.flush()?;
        output.status
    } else {
        command
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()
            .with_context(|| format!("failed to start check `{check}`"))?
    };
    if !status.success() {
        return Err(anyhow!("check failed: {check}"));
    }
    Ok(())
}
