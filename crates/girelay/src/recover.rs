use crate::cli::{RecoverArgs, RecoverCommand};
use crate::{clean, git, output, session, task, workspace_lock};
use anyhow::{Context, Result, anyhow};
use serde::Serialize;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize)]
struct RecoveryPoint {
    recovery_id: String,
    task_id: String,
    kind: String,
    object: Option<String>,
    restorable: bool,
    created_at: Option<String>,
    size_bytes: u64,
    note: String,
}

#[derive(Debug, Serialize)]
struct RecoveryList {
    schema_version: u32,
    count: usize,
    oldest_created_at: Option<String>,
    disk_usage_bytes: u64,
    recovery_points: Vec<RecoveryPoint>,
}

#[derive(Debug, Serialize)]
struct LockInspection {
    schema_version: u32,
    task_id: String,
    operation: String,
    parent_pid: u32,
    parent_alive: bool,
    child_pid: Option<u32>,
    child_alive: bool,
    created_at: String,
    active_session_id: Option<String>,
    recoverable: bool,
    unlocked: bool,
}

pub fn recover(args: RecoverArgs) -> Result<()> {
    let source = git::source_repo(Path::new("."))?;
    match args.command {
        RecoverCommand::List { task_id, json } => list(&source, task_id.as_deref(), json),
        RecoverCommand::Show { recovery_id, json } => show(&source, &recovery_id, json),
        RecoverCommand::Restore {
            recovery_id,
            confirm,
        } => restore(&source, &recovery_id, confirm),
        RecoverCommand::Unlock {
            task_id,
            confirm,
            json,
        } => unlock(&source, &task_id, confirm, json),
    }
}

fn list(source: &Path, filter: Option<&str>, json: bool) -> Result<()> {
    let mut points = ref_points(source)?;
    points.extend(archive_points(source)?);
    for point in &mut points {
        assess(source, point);
    }
    if let Some(task_id) = filter {
        points.retain(|point| point.task_id == task_id);
    }
    points.sort_by(|a, b| {
        a.created_at
            .cmp(&b.created_at)
            .then_with(|| a.recovery_id.cmp(&b.recovery_id))
    });
    let summary = recovery_list(points);
    if json {
        return output::json(&summary);
    }
    if summary.recovery_points.is_empty() {
        println!("No recovery points found.");
        return Ok(());
    }
    println!(
        "{:<18} {:<12} {:<11} Description",
        "Type", "Created", "Restorable"
    );
    for point in &summary.recovery_points {
        println!(
            "{:<18} {:<12} {:<11} {}",
            point.kind,
            relative_age(point.created_at.as_deref()),
            if point.restorable { "yes" } else { "no" },
            point.note
        );
        println!("  id: {}", point.recovery_id);
    }
    println!("Recovery points: {}", summary.count);
    println!(
        "Oldest: {}",
        summary
            .oldest_created_at
            .as_deref()
            .map(|value| relative_age(Some(value)))
            .unwrap_or_else(|| "unknown".into())
    );
    println!(
        "Approximate storage: {}",
        human_bytes(summary.disk_usage_bytes)
    );
    println!("Use `girelay recover show <recovery-id>` for exact refs and objects.");
    Ok(())
}

fn show(source: &Path, id: &str, json: bool) -> Result<()> {
    let mut point = find(source, id)?;
    assess(source, &mut point);
    if json {
        return output::json(&point);
    }
    println!("Recovery: {}", point.recovery_id);
    println!("Task: {}", point.task_id);
    println!("Type: {}", point.kind);
    println!(
        "Created: {}",
        point.created_at.as_deref().unwrap_or("unknown")
    );
    println!("Approximate size: {}", human_bytes(point.size_bytes));
    if let Some(object) = point.object {
        println!("Object: {object}");
    }
    println!("Restore: {}", point.note);
    Ok(())
}

fn recovery_list(points: Vec<RecoveryPoint>) -> RecoveryList {
    let oldest_created_at = points
        .iter()
        .filter_map(|point| point.created_at.as_ref())
        .min_by_key(|value| value.parse::<u64>().unwrap_or(u64::MAX))
        .cloned();
    RecoveryList {
        schema_version: task::SCHEMA_VERSION,
        count: points.len(),
        oldest_created_at,
        disk_usage_bytes: points.iter().map(|point| point.size_bytes).sum(),
        recovery_points: points,
    }
}

fn unlock(source: &Path, task_id: &str, confirm: bool, json: bool) -> Result<()> {
    task::validate_task_id(task_id)?;
    let mut record = task::load(source, task_id)?;
    let lock = workspace_lock::read(source, task_id)?;
    let parent_alive = workspace_lock::process_alive(lock.parent_pid);
    let child_alive = lock.child_pid.is_some_and(workspace_lock::process_alive);
    let mut inspection = LockInspection {
        schema_version: task::SCHEMA_VERSION,
        task_id: task_id.into(),
        operation: lock.operation.clone(),
        parent_pid: lock.parent_pid,
        parent_alive,
        child_pid: lock.child_pid,
        child_alive,
        created_at: lock.created_at.clone(),
        active_session_id: record.active_session_id.clone(),
        recoverable: !parent_alive && !child_alive,
        unlocked: false,
    };
    if !confirm {
        if json {
            return output::json(&inspection);
        }
        print_lock(&inspection);
        if inspection.recoverable {
            println!("Next: girelay recover unlock {task_id} --confirm");
        } else {
            println!("No unlock is available while a recorded process is alive.");
        }
        return Ok(());
    }
    if !inspection.recoverable {
        return Err(anyhow!(
            "refusing to unlock task '{}': a recorded parent or child process is still alive",
            task_id
        ));
    }
    let claim = workspace_lock::claim_stale(source, task_id, &lock)?;
    let interrupted_session = session::close_interrupted_session(source, &record)?;
    if let Some(interrupted) = interrupted_session.as_ref() {
        record.active_session_id = None;
        record.latest_session_id = Some(interrupted.clone());
        record.updated_at = task::timestamp();
        task::save(source, &record)?;
    }
    claim.finish()?;
    inspection.unlocked = true;
    inspection.active_session_id = None;
    if json {
        output::json(&inspection)
    } else {
        print_lock(&inspection);
        if interrupted_session.is_some() {
            println!("Unlocked task {task_id}; interrupted session state was preserved.");
        } else {
            println!("Unlocked stale {} lock for task {task_id}.", lock.operation);
        }
        Ok(())
    }
}

fn print_lock(value: &LockInspection) {
    println!("Task: {}", value.task_id);
    println!("Operation: {}", value.operation);
    println!("Created: {}", value.created_at);
    println!(
        "Parent PID: {} ({})",
        value.parent_pid,
        if value.parent_alive {
            "alive"
        } else {
            "not running"
        }
    );
    if let Some(pid) = value.child_pid {
        println!(
            "Child PID: {} ({})",
            pid,
            if value.child_alive {
                "alive"
            } else {
                "not running"
            }
        );
    }
    println!(
        "Recoverable: {}",
        if value.recoverable { "yes" } else { "no" }
    );
}

fn restore(source: &Path, id: &str, confirm: bool) -> Result<()> {
    if !confirm {
        return Err(anyhow!(
            "refusing recovery without --confirm; inspect first with `girelay recover show {id}`"
        ));
    }
    if let Some(archive_id) = id.strip_prefix("archive/") {
        return restore_archive(source, archive_id);
    }
    let point = find(source, id)?;
    let object = point
        .object
        .as_deref()
        .ok_or_else(|| anyhow!("recovery point has no Git object"))?;
    if id.starts_with("refs/girelay/rollback/source/") {
        return restore_source(source, &point.task_id, id, object);
    }
    restore_to_new_worktree(source, &point.task_id, id, object)
}

fn restore_source(source: &Path, task_id: &str, id: &str, object: &str) -> Result<()> {
    let mut record = task::load(source, task_id)?;
    let _lock = workspace_lock::acquire(source, task_id, "source-recovery")?;
    let merged = record
        .merge
        .clone()
        .ok_or_else(|| anyhow!("task has no merge record"))?;
    if merged.source_rollback_ref != id {
        return Err(anyhow!(
            "recovery ref does not match the task's current merge record"
        ));
    }
    if git::current_branch(source)? != merged.target_branch {
        return Err(anyhow!(
            "source checkout is not on recorded target branch '{}'",
            merged.target_branch
        ));
    }
    if git::is_dirty(source)? {
        return Err(anyhow!(
            "source checkout is dirty; recovery will not overwrite it"
        ));
    }
    if git::head_commit(source, "HEAD")? != merged.source_after {
        return Err(anyhow!(
            "source branch changed after merge; stale rollback is refused"
        ));
    }
    git::run_quiet(source, &["reset", "--hard", object])?;
    record.merge = None;
    record.lifecycle = task::TaskLifecycle::Active;
    record.updated_at = task::timestamp();
    if let Err(error) = task::save(source, &record) {
        git::run_quiet(source, &["reset", "--hard", &merged.source_after])?;
        return Err(error).context("failed to update task metadata; source rollback was reversed");
    }
    println!(
        "Restored source branch {} to {}",
        merged.target_branch, object
    );
    Ok(())
}

fn restore_to_new_worktree(source: &Path, task_id: &str, id: &str, object: &str) -> Result<()> {
    let suffix = task::unique_id();
    let branch = format!("recovery/{task_id}/{suffix}");
    let workspace = source
        .join(".girelay/recovered")
        .join(format!("{task_id}-{suffix}"));
    git::run_quiet(source, &["branch", &branch, object])?;
    if let Err(error) = git::add_existing_worktree(source, &workspace, &branch) {
        let _ = git::delete_branch(source, &branch);
        return Err(error)
            .context("failed to create recovery worktree; recovery branch was rolled back");
    }
    println!("Restored {id}");
    println!("Branch: {branch}");
    println!("Workspace: {}", workspace.display());
    Ok(())
}

fn restore_archive(source: &Path, archive_id: &str) -> Result<()> {
    let manifest = clean::verify_archive(source, archive_id)?;
    let (_, root) = clean::load_archive(source, archive_id)?;
    let archived: task::Task =
        serde_json::from_slice(&fs::read(root.join(&manifest.task_metadata))?)?;
    if archived.source_repo != source {
        return Err(anyhow!("archive belongs to a different source repository"));
    }
    let mut record = if task::task_file(source, &archived.id).exists() {
        let current = task::load(source, &archived.id)?;
        if current.source_repo != archived.source_repo
            || current.workspace_path != archived.workspace_path
            || current.branch != archived.branch
        {
            return Err(anyhow!(
                "current task metadata does not match the archived task identity"
            ));
        }
        current
    } else {
        archived
    };
    let _lock = workspace_lock::acquire(source, &record.id, "archive-recovery")?;
    if record.workspace_path.exists() {
        return Err(anyhow!(
            "workspace already exists at {}",
            record.workspace_path.display()
        ));
    }
    let branch_ref = format!("refs/heads/{}", record.branch);
    if !git::succeeds(
        source,
        &[
            "cat-file",
            "-e",
            &format!("{}^{{commit}}", manifest.restore_commit),
        ],
    )? {
        let bundle = root.join(&manifest.bundle);
        git::run_quiet(
            source,
            &[
                "fetch",
                git::path_str(&bundle)?,
                &format!("{}:{}", manifest.restore_ref, manifest.restore_ref),
            ],
        )?;
    }
    let original_branch = if git::ref_exists(source, &branch_ref) {
        let current = git::head_commit(source, &branch_ref)?;
        if current != manifest.branch_tip && current != manifest.restore_commit {
            return Err(anyhow!("existing task branch does not match archived tip"));
        }
        Some(current)
    } else {
        None
    };
    git::update_ref(source, &branch_ref, &manifest.restore_commit)?;
    if let Err(error) = git::add_existing_worktree(source, &record.workspace_path, &record.branch) {
        restore_branch_state(source, &branch_ref, original_branch.as_deref())?;
        return Err(error)
            .context("failed to recreate archived worktree; task branch was restored");
    }
    if manifest.restore_commit != manifest.branch_tip {
        if let Err(error) = git::run_quiet(
            &record.workspace_path,
            &["reset", "--mixed", &manifest.branch_tip],
        ) {
            let _ = git::remove_worktree(source, &record.workspace_path, true);
            restore_branch_state(source, &branch_ref, original_branch.as_deref())?;
            return Err(error).context(
                "failed to restore archived file state; worktree and branch were rolled back",
            );
        }
    }
    record.lifecycle = if record.merge.is_some() {
        task::TaskLifecycle::Merged
    } else {
        task::TaskLifecycle::Active
    };
    record.updated_at = task::timestamp();
    if let Err(error) = task::save(source, &record) {
        let _ = git::remove_worktree(source, &record.workspace_path, true);
        restore_branch_state(source, &branch_ref, original_branch.as_deref())?;
        return Err(error).context(
            "failed to publish restored task metadata; worktree and branch were rolled back",
        );
    }
    restore_metadata_dir(
        &root.join("sessions"),
        &source.join(".girelay/sessions").join(&record.id),
    )?;
    restore_metadata_dir(
        &root.join("reports"),
        &source.join(".girelay/reports").join(&record.id),
    )?;
    println!(
        "Restored archive {} to {}",
        archive_id,
        record.workspace_path.display()
    );
    Ok(())
}

fn restore_branch_state(source: &Path, branch_ref: &str, original: Option<&str>) -> Result<()> {
    if let Some(object) = original {
        git::update_ref(source, branch_ref, object)
    } else {
        git::run_quiet(source, &["update-ref", "-d", branch_ref])
    }
}

fn restore_metadata_dir(from: &Path, to: &Path) -> Result<()> {
    if !from.exists() {
        return Ok(());
    }
    fs::create_dir_all(to)?;
    for entry in fs::read_dir(from)? {
        let entry = entry?;
        let destination = to.join(entry.file_name());
        if !destination.exists() {
            fs::copy(entry.path(), destination)?;
        }
    }
    Ok(())
}

fn find(source: &Path, id: &str) -> Result<RecoveryPoint> {
    ref_points(source)?
        .into_iter()
        .chain(archive_points(source)?)
        .find(|point| point.recovery_id == id)
        .ok_or_else(|| anyhow!("unknown recovery point '{id}'"))
}

fn ref_points(source: &Path) -> Result<Vec<RecoveryPoint>> {
    let out = git::run(
        source,
        &[
            "for-each-ref",
            "--format=%(refname)%09%(objectname)",
            "refs/girelay/snapshots/",
            "refs/girelay/rollback/",
        ],
    )?;
    let mut points = Vec::new();
    for line in out.stdout.lines() {
        let Some((reference, object)) = line.split_once('\t') else {
            continue;
        };
        let Some(mut point) = classify_ref(reference, object) else {
            continue;
        };
        point.size_bytes = git::object_size(source, object).unwrap_or(0);
        points.push(point);
    }
    Ok(points)
}

fn classify_ref(reference: &str, object: &str) -> Option<RecoveryPoint> {
    let (kind, task_id, timestamp_id, note) =
        if let Some(rest) = reference.strip_prefix("refs/girelay/snapshots/") {
            let mut components = rest.split('/');
            (
                "relay-snapshot",
                components.next()?,
                components.next()?,
                "restores into a new recovery branch and worktree",
            )
        } else if let Some(rest) = reference.strip_prefix("refs/girelay/rollback/task/") {
            let mut components = rest.split('/');
            (
                "task-rollback",
                components.next()?,
                components.next()?,
                "restores into a new recovery branch and worktree",
            )
        } else {
            let rest = reference.strip_prefix("refs/girelay/rollback/source/")?;
            let mut components = rest.split('/');
            (
                "source-pre-merge",
                components.next()?,
                components.next()?,
                "restores only when the source still matches the recorded merge result",
            )
        };
    Some(RecoveryPoint {
        recovery_id: reference.into(),
        task_id: task_id.into(),
        kind: kind.into(),
        object: Some(object.into()),
        restorable: true,
        created_at: created_from_unique_id(timestamp_id),
        size_bytes: 0,
        note: note.into(),
    })
}

fn archive_points(source: &Path) -> Result<Vec<RecoveryPoint>> {
    let root = source.join(".girelay/archive");
    if !root.exists() {
        return Ok(Vec::new());
    }
    let mut points = Vec::new();
    for entry in fs::read_dir(&root)? {
        let id = entry?.file_name().to_string_lossy().to_string();
        let directory = root.join(&id);
        let size_bytes = directory_size(&directory).unwrap_or(0);
        match clean::verify_archive(source, &id) {
            Ok(manifest) => points.push(RecoveryPoint {
                recovery_id: format!("archive/{id}"),
                task_id: manifest.task_id,
                kind: "cleanup-archive".into(),
                object: Some(manifest.branch_tip),
                restorable: true,
                created_at: Some(manifest.created_at),
                size_bytes,
                note: "restores the recorded task worktree and metadata".into(),
            }),
            Err(error) => points.push(RecoveryPoint {
                recovery_id: format!("archive/{id}"),
                task_id: "unknown".into(),
                kind: "cleanup-archive".into(),
                object: None,
                restorable: false,
                created_at: created_from_unique_id(&id),
                size_bytes,
                note: format!("archive verification failed: {error:#}"),
            }),
        }
    }
    Ok(points)
}

fn created_from_unique_id(value: &str) -> Option<String> {
    let mut numeric_parts = value.rsplit('-').filter(|part| {
        !part.is_empty() && part.chars().all(|character| character.is_ascii_digit())
    });
    numeric_parts.next()?;
    numeric_parts.next().map(ToString::to_string)
}

fn directory_size(path: &Path) -> Result<u64> {
    let mut size = 0;
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let metadata = entry.metadata()?;
        if metadata.is_dir() {
            size += directory_size(&entry.path())?;
        } else {
            size += metadata.len();
        }
    }
    Ok(size)
}

fn relative_age(created_at: Option<&str>) -> String {
    let Some(created) = created_at.and_then(|value| value.parse::<u64>().ok()) else {
        return "unknown".into();
    };
    let now = task::timestamp().parse::<u64>().unwrap_or(created);
    let seconds = now.saturating_sub(created);
    match seconds {
        0..=59 => format!("{seconds}s ago"),
        60..=3_599 => format!("{}m ago", seconds / 60),
        3_600..=86_399 => format!("{}h ago", seconds / 3_600),
        _ => format!("{}d ago", seconds / 86_400),
    }
}

fn human_bytes(bytes: u64) -> String {
    const KIB: u64 = 1024;
    const MIB: u64 = KIB * 1024;
    if bytes >= MIB {
        format!("{:.1} MiB", bytes as f64 / MIB as f64)
    } else if bytes >= KIB {
        format!("{:.1} KiB", bytes as f64 / KIB as f64)
    } else {
        format!("{bytes} B")
    }
}

fn assess(source: &Path, point: &mut RecoveryPoint) {
    match point.kind.as_str() {
        "relay-snapshot" | "task-rollback" => {
            point.restorable = point.object.as_ref().is_some_and(|object| {
                git::succeeds(source, &["cat-file", "-e", &format!("{object}^{{commit}}")])
                    .unwrap_or(false)
            });
            if !point.restorable {
                point.note = "Git object is missing or invalid".into();
            }
        }
        "source-pre-merge" => {
            let valid = task::load(source, &point.task_id).ok().and_then(|record| {
                let merged = record.merge?;
                Some(
                    merged.source_rollback_ref == point.recovery_id
                        && git::current_branch(source).ok().as_deref()
                            == Some(merged.target_branch.as_str())
                        && git::head_commit(source, "HEAD").ok().as_deref()
                            == Some(merged.source_after.as_str())
                        && !git::is_dirty(source).unwrap_or(true),
                )
            });
            point.restorable = valid.unwrap_or(false);
            if !point.restorable {
                point.note =
                    "stale or blocked: source no longer matches the recorded merge result".into();
            }
        }
        "cleanup-archive" if point.restorable => {
            point.restorable = task::load(source, &point.task_id)
                .map(|record| !record.workspace_path.exists())
                .unwrap_or(true);
            if !point.restorable {
                point.note = "task workspace already exists".into();
            }
        }
        _ => {}
    }
}
