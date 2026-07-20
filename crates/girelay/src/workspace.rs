use crate::cli::StartArgs;
use crate::{config, git, session, task};
use anyhow::{Context, Result, anyhow};
use std::fs;
use std::path::Path;

pub fn start(args: StartArgs) -> Result<()> {
    task::validate_task_id(&args.task_id)?;
    let (intent, intent_source) = match args.intent {
        Some(intent) => {
            task::validate_intent(&intent)?;
            (intent, task::IntentSource::Explicit)
        }
        None => (args.task_id.clone(), task::IntentSource::TaskId),
    };
    let source = git::source_repo(Path::new("."))
        .context("girelay start must run inside the source Git repository")?;
    if git::repo_root(Path::new("."))? != source {
        return Err(anyhow!(
            "girelay start must run from the source checkout, not a linked worktree"
        ));
    }
    config::ensure_layout(&source)?;
    if task::task_file(&source, &args.task_id).exists() {
        return Err(anyhow!(
            "task '{}' already exists; use `girelay relay {}` to continue it",
            args.task_id,
            args.task_id
        ));
    }
    if git::is_dirty(&source)? {
        return Err(anyhow!(
            "source checkout has uncommitted changes; commit or stash them before starting a task"
        ));
    }
    let cfg = config::load(&source)?;
    let base = args.base.unwrap_or_else(|| cfg.workspace.base.clone());
    let base_commit = git::head_commit(&source, &base)
        .with_context(|| format!("failed to resolve base branch '{base}'"))?;
    let workspace = config::workspace_root(&source, &cfg).join(&args.task_id);
    if workspace.exists() {
        return Err(anyhow!(
            "workspace already exists at {}",
            workspace.display()
        ));
    }
    if let Some(parent) = workspace.parent() {
        fs::create_dir_all(parent)?;
    }
    let branch = format!("{}{}", cfg.workspace.branch_prefix, args.task_id);
    if git::ref_exists(&source, &format!("refs/heads/{branch}")) {
        return Err(anyhow!(
            "branch '{branch}' already exists; choose another task id"
        ));
    }
    git::add_worktree(&source, &workspace, &branch, &base)?;
    let record = task::Task::new(
        args.task_id.clone(),
        task::TaskIntent {
            value: intent,
            source: intent_source,
        },
        source.clone(),
        workspace.clone(),
        base,
        base_commit,
        branch,
    );
    if let Err(error) = task::save(&source, &record) {
        let _ = git::remove_worktree(&source, &workspace, true);
        return Err(error).context("failed to publish task metadata; worktree was rolled back");
    }
    println!("Created task {}", record.id);
    println!("Workspace: {}", record.workspace_path.display());
    println!("Branch: {}", record.branch);
    if args.command.is_empty() {
        println!("Next: girelay relay {} -- <agent>", record.id);
        Ok(())
    } else {
        session::run_session(&source, record, args.command, false)
    }
}
