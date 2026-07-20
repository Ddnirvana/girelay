use crate::cli::{MergeArgs, MergeStrategy};
use crate::{config, git, output, session, task, workspace_lock};
use anyhow::{Context, Result, anyhow};
use serde::Serialize;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

#[derive(Debug, Serialize)]
struct MergeOutput {
    schema_version: u32,
    task_id: String,
    strategy: String,
    target_branch: String,
    source_before: String,
    source_after: String,
    task_tip: String,
    changed_files: Vec<String>,
    checks: Vec<String>,
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
    let mut record = task::load(&source, &args.task_id)?;
    if args
        .message
        .as_deref()
        .unwrap_or(&record.intent)
        .trim()
        .is_empty()
    {
        return Err(anyhow!("merge message cannot be empty"));
    }
    if record.merge.is_some() {
        return Err(anyhow!("task '{}' already has a merge record", record.id));
    }
    if !record.workspace_path.is_dir() {
        return Err(anyhow!("task '{}' worktree is missing", record.id));
    }
    if git::current_branch(&source)? != record.base_branch {
        return Err(anyhow!(
            "source checkout must have target branch '{}' checked out",
            record.base_branch
        ));
    }
    if git::is_dirty(&source)? {
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
    let _lock = workspace_lock::acquire(&source, &record.id, "merge")?;
    let cfg = config::load(&source)?;
    git::ensure_commit_identity(&source)?;
    let source_before = git::head_commit(&source, "HEAD")?;
    let task_before = git::head_commit(&record.workspace_path, "HEAD")?;
    let task_state_before = git::working_tree_state(&record.workspace_path)?;
    let merge_id = task::unique_id();
    let _snapshot = session::snapshot(&source, &record, &format!("merge-{merge_id}"), "pre-merge")?;
    let checks = if cfg.merge.run_checks && !args.no_checks {
        cfg.merge.check_commands.clone()
    } else {
        Vec::new()
    };
    for check in &checks {
        run_check(&record.workspace_path, check, args.json)?;
    }
    revalidate_task(
        &record,
        &task_before,
        &task_state_before,
        "configured checks",
    )?;
    revalidate_source(&source, &record, &source_before)?;

    let task_rollback_ref = format!("refs/girelay/rollback/task/{}/{}", record.id, merge_id);
    git::update_ref(&source, &task_rollback_ref, &task_before)?;
    if git::is_dirty(&record.workspace_path)? {
        git::run_quiet(&record.workspace_path, &["add", "-A"])?;
        let message = args.message.as_deref().unwrap_or(&record.intent);
        git::run_quiet(&record.workspace_path, &["commit", "-m", message])
            .context("failed to create the final task commit; task rollback ref was preserved")?;
        if git::is_dirty(&record.workspace_path)? {
            return Err(anyhow!(
                "task worktree is still dirty after final commit; source was not changed"
            ));
        }
    }
    let task_tip = git::head_commit(&record.workspace_path, "HEAD")?;
    let changed_files = git::changed_files(&source, &source_before, &task_tip)?;
    if changed_files.is_empty() {
        return Err(anyhow!("task has no changes relative to the source branch"));
    }
    revalidate_source(&source, &record, &source_before)?;
    let source_rollback_ref = format!("refs/girelay/rollback/source/{}/{}", record.id, merge_id);
    git::update_ref(&source, &source_rollback_ref, &source_before)?;
    let message = args
        .message
        .clone()
        .unwrap_or_else(|| record.intent.clone());
    let strategy = match args.strategy {
        MergeStrategy::Squash => "squash",
        MergeStrategy::Preserve => "preserve",
    };
    let apply = match args.strategy {
        MergeStrategy::Squash => git::run_quiet(&source, &["merge", "--squash", &record.branch])
            .and_then(|_| git::run_quiet(&source, &["commit", "-m", &message])),
        MergeStrategy::Preserve => git::run_quiet(
            &source,
            &["merge", "--no-ff", &record.branch, "-m", &message],
        ),
    };
    if let Err(error) = apply {
        if let Err(rollback) = rollback_source(&source, &source_before) {
            return Err(anyhow!(
                "merge failed: {error:#}; automatic source rollback also failed: {rollback:#}"
            ));
        }
        return Err(error)
            .context("merge failed; source checkout was restored and rollback refs were retained");
    }
    let source_after = git::head_commit(&source, "HEAD")?;
    record.lifecycle = task::TaskLifecycle::Merged;
    record.updated_at = task::timestamp();
    record.merge = Some(task::MergeRecord {
        strategy: strategy.into(),
        target_branch: record.base_branch.clone(),
        source_before: source_before.clone(),
        source_after: source_after.clone(),
        task_tip: task_tip.clone(),
        task_rollback_ref: task_rollback_ref.clone(),
        source_rollback_ref: source_rollback_ref.clone(),
        merged_at: task::timestamp(),
    });
    if let Err(error) = task::save(&source, &record) {
        rollback_source(&source, &source_before)?;
        return Err(error)
            .context("failed to publish merge record; source checkout was rolled back");
    }
    let result = MergeOutput {
        schema_version: task::SCHEMA_VERSION,
        task_id: record.id,
        strategy: strategy.into(),
        target_branch: record.base_branch,
        source_before,
        source_after,
        task_tip,
        changed_files,
        checks,
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

fn run_check(workspace: &Path, check: &str, json: bool) -> Result<()> {
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
