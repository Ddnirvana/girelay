use crate::cli::CleanArgs;
use crate::{git, output, session, task, workspace_lock};
use anyhow::{Context, Result, anyhow};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

#[derive(Debug, Serialize)]
struct CleanPlan {
    schema_version: u32,
    task_id: String,
    workspace: PathBuf,
    branch: String,
    workspace_exists: bool,
    dirty: bool,
    delete_branch: bool,
    branch_tip: Option<String>,
    preserved_by: Vec<String>,
    archive_requested: bool,
    blockers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArchiveManifest {
    pub schema_version: u32,
    pub archive_id: String,
    pub task_id: String,
    pub branch: String,
    pub branch_tip: String,
    pub restore_commit: String,
    pub restore_ref: String,
    pub created_at: String,
    pub bundle: String,
    pub bundle_sha256: String,
    pub task_metadata: String,
    pub files: Vec<ArchiveFile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArchiveFile {
    pub path: String,
    pub sha256: String,
}

pub fn clean(args: CleanArgs) -> Result<()> {
    task::validate_task_id(&args.task_id)?;
    let source = git::source_repo(Path::new("."))?;
    let mut record = task::load(&source, &args.task_id)?;
    let preview = plan(&source, &record, &args)?;
    if args.dry_run || args.json {
        return if args.json {
            output::json(&preview)
        } else {
            print_plan(&preview);
            Ok(())
        };
    }
    reject(&preview)?;
    let _lock =
        workspace_lock::acquire(&source, &record.id, args.recover_stale_session, "cleanup")?;
    if args.recover_stale_session {
        if let Some(interrupted) = session::close_interrupted_session(&source, &record)? {
            record.active_session_id = None;
            record.latest_session_id = Some(interrupted);
            record.updated_at = task::timestamp();
            task::save(&source, &record)?;
        }
    }
    let locked = plan(&source, &record, &args)?;
    reject(&locked)?;

    let archive_snapshot = if locked.workspace_exists && args.archive {
        let snapshot_id = format!("cleanup-{}", task::unique_id());
        let commit = session::snapshot(&source, &record, &snapshot_id, "pre-clean")?;
        let reference = format!(
            "refs/girelay/snapshots/{}/{snapshot_id}/pre-clean",
            record.id
        );
        Some((commit, reference))
    } else {
        None
    };
    let archive = if args.archive {
        Some(create_archive(&source, &record, archive_snapshot.as_ref())?)
    } else {
        None
    };
    let final_plan = plan(&source, &record, &args)?;
    reject(&final_plan)?;
    if locked.branch_tip != final_plan.branch_tip || locked.dirty != final_plan.dirty {
        return Err(anyhow!(
            "task changed while cleanup was being prepared; nothing was removed"
        ));
    }
    if args.delete_branch {
        validate_branch_deletion(&source, &record)?;
    }
    if record.workspace_path.exists() {
        git::remove_worktree(&source, &record.workspace_path, final_plan.dirty)?;
    }
    if args.delete_branch {
        git::delete_branch(&source, &record.branch)?;
    }
    record.lifecycle = task::TaskLifecycle::Cleaned;
    record.updated_at = task::timestamp();
    task::save(&source, &record)?;
    println!("Cleaned task {}", record.id);
    println!(
        "Branch: {}",
        if args.delete_branch {
            "deleted"
        } else {
            "retained"
        }
    );
    if let Some(archive) = archive {
        println!("Archive: {}", archive.archive_id);
    }
    Ok(())
}

fn plan(source: &Path, record: &task::Task, args: &CleanArgs) -> Result<CleanPlan> {
    let workspace_exists = record.workspace_path.is_dir();
    let dirty = workspace_exists && git::is_dirty(&record.workspace_path).unwrap_or(true);
    let branch_ref = format!("refs/heads/{}", record.branch);
    let branch_tip = git::ref_exists(source, &branch_ref)
        .then(|| git::head_commit(source, &branch_ref))
        .transpose()?;
    let mut preserved_by = Vec::new();
    if branch_tip.is_some() && !args.delete_branch {
        preserved_by.push(branch_ref);
    }
    let snapshot_pattern = format!("refs/girelay/snapshots/{}/", record.id);
    if !git::run(
        source,
        &["for-each-ref", "--format=%(refname)", &snapshot_pattern],
    )?
    .stdout
    .is_empty()
    {
        preserved_by.push(snapshot_pattern);
    }
    if args.archive {
        preserved_by.push("verified cleanup archive".into());
    }
    let mut blockers = Vec::new();
    if workspace_exists {
        let actual_source = git::source_repo(&record.workspace_path).ok();
        let actual_branch = git::current_branch(&record.workspace_path).ok();
        if actual_source.as_ref() != Some(&record.source_repo)
            || actual_branch.as_deref() != Some(&record.branch)
        {
            blockers.push("worktree ownership does not match task metadata".into());
        }
    }
    if dirty && !args.discard_uncommitted && !args.archive {
        blockers.push(
            "worktree has uncommitted changes; use --archive or --discard-uncommitted".into(),
        );
    }
    if branch_tip.is_none() && !args.archive && !args.discard_unreachable {
        blockers.push("task branch is missing and no archive was requested".into());
    }
    if args.delete_branch {
        if let Err(error) = validate_branch_deletion(source, record) {
            blockers.push(error.to_string());
        }
    }
    Ok(CleanPlan {
        schema_version: task::SCHEMA_VERSION,
        task_id: record.id.clone(),
        workspace: record.workspace_path.clone(),
        branch: record.branch.clone(),
        workspace_exists,
        dirty,
        delete_branch: args.delete_branch,
        branch_tip,
        preserved_by,
        archive_requested: args.archive,
        blockers,
    })
}

fn validate_branch_deletion(source: &Path, record: &task::Task) -> Result<()> {
    let merged = record
        .merge
        .as_ref()
        .ok_or_else(|| anyhow!("--delete-branch requires a valid merge record"))?;
    if git::current_branch(source)? != merged.target_branch {
        return Err(anyhow!(
            "source checkout must be on recorded target branch '{}'",
            merged.target_branch
        ));
    }
    if git::is_dirty(source)? {
        return Err(anyhow!("source checkout is dirty"));
    }
    if git::head_commit(source, "HEAD")? != merged.source_after {
        return Err(anyhow!(
            "source branch advanced after the recorded merge; branch deletion requires manual review"
        ));
    }
    let branch_tip = git::head_commit(source, &format!("refs/heads/{}", record.branch))?;
    if branch_tip != merged.task_tip {
        return Err(anyhow!("task branch changed after the recorded merge"));
    }
    if !git::ref_exists(source, &merged.task_rollback_ref)
        || !git::ref_exists(source, &merged.source_rollback_ref)
    {
        return Err(anyhow!("recorded rollback refs are missing"));
    }
    Ok(())
}

fn reject(plan: &CleanPlan) -> Result<()> {
    if plan.blockers.is_empty() {
        return Ok(());
    }
    Err(anyhow!(
        "cleanup refused for task '{}':\n- {}",
        plan.task_id,
        plan.blockers.join("\n- ")
    ))
}

fn print_plan(plan: &CleanPlan) {
    println!("Cleanup plan for {}", plan.task_id);
    println!("Workspace exists: {}", plan.workspace_exists);
    println!("Dirty: {}", plan.dirty);
    println!(
        "Branch: {} ({})",
        plan.branch,
        if plan.delete_branch {
            "delete"
        } else {
            "retain"
        }
    );
    for value in &plan.preserved_by {
        println!("Preserved by: {value}");
    }
    for blocker in &plan.blockers {
        println!("Blocked: {blocker}");
    }
}

fn create_archive(
    source: &Path,
    record: &task::Task,
    snapshot: Option<&(String, String)>,
) -> Result<ArchiveManifest> {
    let archive_id = format!("{}-{}", record.id, task::unique_id());
    let root = source.join(".girelay/archive").join(&archive_id);
    fs::create_dir_all(&root)?;
    let bundle_path = root.join("repository.bundle");
    let status = Command::new("git")
        .args(["bundle", "create", git::path_str(&bundle_path)?, "--all"])
        .current_dir(source)
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .output()?;
    if !status.status.success() {
        let _ = fs::remove_dir_all(&root);
        return Err(anyhow!(
            "failed to create cleanup bundle: {}",
            String::from_utf8_lossy(&status.stderr).trim()
        ));
    }
    git::run_quiet(source, &["bundle", "verify", git::path_str(&bundle_path)?])
        .context("cleanup bundle verification failed")?;
    let hash = hex_sha256(&fs::read(&bundle_path)?);
    let task_metadata = "task.json".to_string();
    fs::copy(
        task::task_file(source, &record.id),
        root.join(&task_metadata),
    )?;
    copy_tree(
        &source.join(".girelay/sessions").join(&record.id),
        &root.join("sessions"),
    )?;
    copy_tree(
        &source.join(".girelay/reports").join(&record.id),
        &root.join("reports"),
    )?;
    let branch_tip = git::head_commit(source, &format!("refs/heads/{}", record.branch))?;
    let (restore_commit, restore_ref) = snapshot
        .cloned()
        .unwrap_or_else(|| (branch_tip.clone(), format!("refs/heads/{}", record.branch)));
    let files = archive_files(&root)?;
    let manifest = ArchiveManifest {
        schema_version: task::SCHEMA_VERSION,
        archive_id: archive_id.clone(),
        task_id: record.id.clone(),
        branch: record.branch.clone(),
        branch_tip,
        restore_commit,
        restore_ref,
        created_at: task::timestamp(),
        bundle: "repository.bundle".into(),
        bundle_sha256: hash,
        task_metadata,
        files,
    };
    task::atomic_write(
        &root.join("manifest.json"),
        &serde_json::to_vec_pretty(&manifest)?,
    )?;
    verify_archive(source, &archive_id)?;
    Ok(manifest)
}

pub fn load_archive(source: &Path, archive_id: &str) -> Result<(ArchiveManifest, PathBuf)> {
    if archive_id.contains('/') || archive_id.contains("..") {
        return Err(anyhow!("invalid archive id"));
    }
    let root = source.join(".girelay/archive").join(archive_id);
    let manifest: ArchiveManifest = serde_json::from_slice(&fs::read(root.join("manifest.json"))?)?;
    if manifest.archive_id != archive_id {
        return Err(anyhow!("archive manifest id mismatch"));
    }
    Ok((manifest, root))
}

pub fn verify_archive(source: &Path, archive_id: &str) -> Result<ArchiveManifest> {
    let (manifest, root) = load_archive(source, archive_id)?;
    if manifest.schema_version != task::SCHEMA_VERSION {
        return Err(anyhow!("archive schema version is not supported"));
    }
    task::validate_task_id(&manifest.task_id)?;
    validate_archive_path(&manifest.bundle)?;
    validate_archive_path(&manifest.task_metadata)?;
    let bundle = root.join(&manifest.bundle);
    if hex_sha256(&fs::read(&bundle)?) != manifest.bundle_sha256 {
        return Err(anyhow!("archive bundle checksum mismatch"));
    }
    let mut has_bundle = false;
    let mut has_task = false;
    for file in &manifest.files {
        validate_archive_path(&file.path)?;
        let actual = hex_sha256(&fs::read(root.join(&file.path))?);
        if actual != file.sha256 {
            return Err(anyhow!("archive file checksum mismatch: {}", file.path));
        }
        has_bundle |= file.path == manifest.bundle;
        has_task |= file.path == manifest.task_metadata;
    }
    if !has_bundle || !has_task {
        return Err(anyhow!("archive manifest omits required files"));
    }
    git::run_quiet(source, &["bundle", "verify", git::path_str(&bundle)?])?;
    Ok(manifest)
}

fn archive_files(root: &Path) -> Result<Vec<ArchiveFile>> {
    let mut paths = vec!["repository.bundle".to_string(), "task.json".to_string()];
    for directory in ["sessions", "reports"] {
        let path = root.join(directory);
        if !path.exists() {
            continue;
        }
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            if entry.file_type()?.is_file() {
                paths.push(format!(
                    "{directory}/{}",
                    entry.file_name().to_string_lossy()
                ));
            }
        }
    }
    paths.sort();
    paths
        .into_iter()
        .map(|path| {
            let sha256 = hex_sha256(&fs::read(root.join(&path))?);
            Ok(ArchiveFile { path, sha256 })
        })
        .collect()
}

fn validate_archive_path(path: &str) -> Result<()> {
    let path = Path::new(path);
    if path.is_absolute()
        || path
            .components()
            .any(|component| !matches!(component, std::path::Component::Normal(_)))
    {
        return Err(anyhow!(
            "archive contains an unsafe path: {}",
            path.display()
        ));
    }
    Ok(())
}

fn copy_tree(from: &Path, to: &Path) -> Result<()> {
    if !from.exists() {
        return Ok(());
    }
    fs::create_dir_all(to)?;
    for entry in fs::read_dir(from)? {
        let entry = entry?;
        if entry.file_type()?.is_file() {
            fs::copy(entry.path(), to.join(entry.file_name()))?;
        }
    }
    Ok(())
}

fn hex_sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .fold(String::with_capacity(64), |mut output, byte| {
            let _ = write!(output, "{byte:02x}");
            output
        })
}
