use crate::common::{Repo, git, run_fail, run_ok, write};
use serde_json::Value;
use std::fs;

#[test]
fn clean_refuses_dirty_work_without_an_explicit_preservation_choice() {
    let repo = Repo::new();
    let workspace = repo.start("dirty");
    write(&workspace.join("valuable.txt"), "valuable\n");
    let error = run_fail(&repo.root, &["clean", "dirty"]);
    assert!(error.contains("uncommitted changes"));
    assert!(workspace.exists());
}

#[test]
fn default_clean_removes_worktree_but_retains_branch() {
    let repo = Repo::new();
    let workspace = repo.start("retain");
    run_ok(&repo.root, &["clean", "retain"]);
    assert!(!workspace.exists());
    assert!(
        git(
            &repo.root,
            &["show-ref", "--verify", "refs/heads/agent/retain"]
        )
        .contains("refs/heads/agent/retain")
    );
    let status: Value =
        serde_json::from_str(&run_ok(&repo.root, &["status", "retain", "--json"])).unwrap();
    assert_eq!(status["tasks"][0]["state"], "cleaned");
}

#[test]
fn discard_uncommitted_is_explicit_and_retains_committed_branch() {
    let repo = Repo::new();
    let workspace = repo.start("discard-dirty");
    write(&workspace.join("discard-me.txt"), "temporary\n");
    run_ok(
        &repo.root,
        &["clean", "discard-dirty", "--discard-uncommitted"],
    );
    assert!(!workspace.exists());
    assert!(
        git(
            &repo.root,
            &["show-ref", "--verify", "refs/heads/agent/discard-dirty"]
        )
        .contains("refs/heads/agent/discard-dirty")
    );
}

#[test]
fn discard_unreachable_handles_an_already_missing_worktree_and_branch() {
    let repo = Repo::new();
    let workspace = repo.start("missing-everything");
    git(
        &repo.root,
        &["worktree", "remove", workspace.to_str().unwrap()],
    );
    git(&repo.root, &["branch", "-D", "agent/missing-everything"]);
    let error = run_fail(&repo.root, &["clean", "missing-everything"]);
    assert!(error.contains("task branch is missing"));
    run_ok(
        &repo.root,
        &["clean", "missing-everything", "--discard-unreachable"],
    );
}

#[test]
fn archive_preserves_dirty_state_and_can_restore_the_task() {
    let repo = Repo::new();
    let workspace = repo.start("archived");
    write(&workspace.join("valuable.txt"), "valuable\n");
    let stdout = run_ok(&repo.root, &["clean", "archived", "--archive"]);
    let archive_id = stdout
        .lines()
        .find_map(|line| line.strip_prefix("Archive: "))
        .unwrap();
    assert!(!workspace.exists());
    let listed: Value = serde_json::from_str(&run_ok(
        &repo.root,
        &["recover", "list", "archived", "--json"],
    ))
    .unwrap();
    assert!(
        listed["recovery_points"]
            .as_array()
            .unwrap()
            .iter()
            .any(|point| point["recovery_id"] == format!("archive/{archive_id}"))
    );
    run_ok(
        &repo.root,
        &[
            "recover",
            "restore",
            &format!("archive/{archive_id}"),
            "--confirm",
        ],
    );
    assert_eq!(
        fs::read_to_string(workspace.join("valuable.txt")).unwrap(),
        "valuable\n"
    );
}

#[test]
fn branch_deletion_requires_an_unchanged_recorded_merge() {
    let repo = Repo::new();
    let workspace = repo.start("delete-safe");
    write(&workspace.join("done.txt"), "done\n");
    run_ok(&repo.root, &["merge", "delete-safe", "--message", "done"]);
    run_ok(&repo.root, &["clean", "delete-safe", "--delete-branch"]);
    assert!(!workspace.exists());
    assert!(git(&repo.root, &["branch", "--list", "agent/delete-safe"]).is_empty());
}

#[test]
fn branch_deletion_refuses_when_source_advanced_after_merge() {
    let repo = Repo::new();
    let workspace = repo.start("advanced");
    write(&workspace.join("done.txt"), "done\n");
    run_ok(&repo.root, &["merge", "advanced", "--message", "done"]);
    write(&repo.root.join("later.txt"), "later\n");
    git(&repo.root, &["add", "later.txt"]);
    git(&repo.root, &["commit", "-m", "later"]);
    let error = run_fail(&repo.root, &["clean", "advanced", "--delete-branch"]);
    assert!(error.contains("advanced after the recorded merge"));
    assert!(workspace.exists());
}

#[test]
fn source_recovery_refuses_stale_rollback_and_restores_exact_merge() {
    let repo = Repo::new();
    let workspace = repo.start("undo");
    write(&workspace.join("undo.txt"), "undo\n");
    let before = git(&repo.root, &["rev-parse", "HEAD"]);
    let result: Value =
        serde_json::from_str(&run_ok(&repo.root, &["merge", "undo", "--json"])).unwrap();
    let recovery = result["source_rollback_ref"].as_str().unwrap();
    run_ok(&repo.root, &["recover", "restore", recovery, "--confirm"]);
    assert_eq!(git(&repo.root, &["rev-parse", "HEAD"]), before);
    let second = run_fail(&repo.root, &["recover", "restore", recovery, "--confirm"]);
    assert!(second.contains("no merge record"));
}

#[test]
fn failed_merge_restores_clean_source_checkout() {
    let repo = Repo::new();
    let workspace = repo.start("conflict");
    write(&workspace.join("README.md"), "agent\n");
    git(&workspace, &["add", "README.md"]);
    git(&workspace, &["commit", "-m", "agent edit"]);
    write(&repo.root.join("README.md"), "source\n");
    git(&repo.root, &["add", "README.md"]);
    git(&repo.root, &["commit", "-m", "source edit"]);
    let source_head = git(&repo.root, &["rev-parse", "HEAD"]);
    let error = run_fail(&repo.root, &["merge", "conflict", "--strategy", "preserve"]);
    assert!(error.contains("source checkout was restored"));
    assert_eq!(git(&repo.root, &["rev-parse", "HEAD"]), source_head);
    assert_eq!(git(&repo.root, &["status", "--short"]), "");
}

#[test]
fn configured_check_failure_leaves_source_and_task_history_unchanged() {
    let repo = Repo::new();
    let workspace = repo.start("check-fails");
    write(&workspace.join("change.txt"), "change\n");
    let config_path = repo.root.join(".girelay/config.toml");
    let config = fs::read_to_string(&config_path).unwrap();
    fs::write(
        &config_path,
        config.replace("check_commands = []", "check_commands = [\"exit 17\"]"),
    )
    .unwrap();
    let source_before = git(&repo.root, &["rev-parse", "HEAD"]);
    let task_before = git(&workspace, &["rev-parse", "HEAD"]);
    let error = run_fail(&repo.root, &["merge", "check-fails"]);
    assert!(error.contains("check failed"));
    assert_eq!(git(&repo.root, &["rev-parse", "HEAD"]), source_before);
    assert_eq!(git(&workspace, &["rev-parse", "HEAD"]), task_before);
    assert!(workspace.join("change.txt").exists());
}

#[test]
fn configured_check_that_mutates_reviewed_files_is_refused() {
    let repo = Repo::new();
    let workspace = repo.start("mutating-check");
    write(&workspace.join("change.txt"), "change\n");
    let config_path = repo.root.join(".girelay/config.toml");
    let config = fs::read_to_string(&config_path).unwrap();
    fs::write(
        &config_path,
        config.replace(
            "check_commands = []",
            "check_commands = [\"printf generated > generated.txt\"]",
        ),
    )
    .unwrap();
    let source_before = git(&repo.root, &["rev-parse", "HEAD"]);
    let error = run_fail(&repo.root, &["merge", "mutating-check"]);
    assert!(error.contains("task worktree changed during configured checks"));
    assert_eq!(git(&repo.root, &["rev-parse", "HEAD"]), source_before);
    assert!(workspace.join("generated.txt").exists());
}

#[test]
fn merge_json_stays_parseable_when_checks_write_stdout() {
    let repo = Repo::new();
    let workspace = repo.start("json-check");
    write(&workspace.join("change.txt"), "change\n");
    let config_path = repo.root.join(".girelay/config.toml");
    let config = fs::read_to_string(&config_path).unwrap();
    fs::write(
        &config_path,
        config.replace(
            "check_commands = []",
            "check_commands = [\"printf check-output\"]",
        ),
    )
    .unwrap();
    let output = crate::common::run(&repo.root, &["merge", "json-check", "--json"]);
    assert!(output.status.success());
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["task_id"], "json-check");
    assert!(String::from_utf8_lossy(&output.stderr).contains("check-output"));
}

#[test]
fn merge_refuses_dirty_source_before_touching_task_history() {
    let repo = Repo::new();
    let workspace = repo.start("dirty-source");
    write(&workspace.join("task.txt"), "task\n");
    write(&repo.root.join("source.txt"), "source\n");
    let task_before = git(&workspace, &["rev-parse", "HEAD"]);
    let error = run_fail(&repo.root, &["merge", "dirty-source"]);
    assert!(error.contains("source checkout is dirty"));
    assert_eq!(git(&workspace, &["rev-parse", "HEAD"]), task_before);
    assert!(workspace.join("task.txt").exists());
}

#[test]
fn merge_refuses_missing_git_identity_before_finalizing_task() {
    let repo = Repo::new();
    let workspace = repo.start("identity");
    write(&workspace.join("task.txt"), "task\n");
    git(&repo.root, &["config", "user.useConfigOnly", "true"]);
    git(&repo.root, &["config", "--unset", "user.name"]);
    git(&repo.root, &["config", "--unset", "user.email"]);
    let task_before = git(&workspace, &["rev-parse", "HEAD"]);
    let global = tempfile::NamedTempFile::new().unwrap();
    let output = std::process::Command::new(crate::common::girelay())
        .args(["merge", "identity"])
        .env("GIT_CONFIG_GLOBAL", global.path())
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .current_dir(&repo.root)
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("commit identity"));
    assert_eq!(git(&workspace, &["rev-parse", "HEAD"]), task_before);
    assert!(workspace.join("task.txt").exists());
}

#[test]
fn cleanup_dry_run_is_non_mutating() {
    let repo = Repo::new();
    let workspace = repo.start("preview");
    let value: Value = serde_json::from_str(&run_ok(
        &repo.root,
        &["clean", "preview", "--dry-run", "--json"],
    ))
    .unwrap();
    assert_eq!(value["workspace_exists"], true);
    assert!(workspace.exists());
    assert!(!repo.root.join(".girelay/locks/preview.lock").exists());
}

#[test]
fn tampered_archive_is_refused_before_restore() {
    let repo = Repo::new();
    let workspace = repo.start("tamper");
    write(&workspace.join("valuable.txt"), "valuable\n");
    let stdout = run_ok(&repo.root, &["clean", "tamper", "--archive"]);
    let archive_id = stdout
        .lines()
        .find_map(|line| line.strip_prefix("Archive: "))
        .unwrap();
    let bundle = repo
        .root
        .join(".girelay/archive")
        .join(archive_id)
        .join("repository.bundle");
    fs::write(&bundle, b"tampered").unwrap();
    let error = run_fail(
        &repo.root,
        &[
            "recover",
            "restore",
            &format!("archive/{archive_id}"),
            "--confirm",
        ],
    );
    assert!(error.contains("checksum mismatch"));
    assert!(!workspace.exists());
}

#[test]
fn tampered_archived_metadata_is_refused_before_restore() {
    let repo = Repo::new();
    let workspace = repo.start("metadata-tamper");
    write(&workspace.join("valuable.txt"), "valuable\n");
    let stdout = run_ok(&repo.root, &["clean", "metadata-tamper", "--archive"]);
    let archive_id = stdout
        .lines()
        .find_map(|line| line.strip_prefix("Archive: "))
        .unwrap();
    let metadata = repo
        .root
        .join(".girelay/archive")
        .join(archive_id)
        .join("task.json");
    fs::write(&metadata, b"{}").unwrap();
    let error = run_fail(
        &repo.root,
        &[
            "recover",
            "restore",
            &format!("archive/{archive_id}"),
            "--confirm",
        ],
    );
    assert!(error.contains("archive file checksum mismatch"));
    assert!(!workspace.exists());
}

#[test]
fn snapshot_recovery_creates_a_new_worktree_without_overwriting_task() {
    let repo = Repo::new();
    let workspace = repo.start("snapshot");
    run_ok(
        &repo.root,
        &[
            "relay",
            "snapshot",
            "--",
            "sh",
            "-c",
            "printf recovered > recovered.txt",
        ],
    );
    let refs = git(
        &repo.root,
        &[
            "for-each-ref",
            "--format=%(refname)",
            "refs/girelay/snapshots/snapshot/",
        ],
    );
    let end = refs.lines().find(|line| line.ends_with("/end")).unwrap();
    let task_head = git(&workspace, &["rev-parse", "HEAD"]);
    let output = run_ok(&repo.root, &["recover", "restore", end, "--confirm"]);
    let recovery_path = output
        .lines()
        .find_map(|line| line.strip_prefix("Workspace: "))
        .unwrap();
    assert!(
        std::path::Path::new(recovery_path)
            .join("recovered.txt")
            .exists()
    );
    assert_eq!(git(&workspace, &["rev-parse", "HEAD"]), task_head);
    assert!(workspace.join("recovered.txt").exists());
}

#[test]
fn branch_deletion_refuses_when_task_tip_changed_after_merge() {
    let repo = Repo::new();
    let workspace = repo.start("task-advanced");
    write(&workspace.join("done.txt"), "done\n");
    run_ok(&repo.root, &["merge", "task-advanced", "--message", "done"]);
    write(&workspace.join("later.txt"), "later\n");
    git(&workspace, &["add", "later.txt"]);
    git(&workspace, &["commit", "-m", "later task change"]);
    let error = run_fail(&repo.root, &["clean", "task-advanced", "--delete-branch"]);
    assert!(error.contains("task branch changed after the recorded merge"));
    assert!(workspace.exists());
}
