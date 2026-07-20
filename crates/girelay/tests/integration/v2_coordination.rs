use crate::common::{Repo, git, run_ok, write};
use serde_json::Value;
use std::fs;

#[test]
fn overlap_detection_covers_committed_dirty_renamed_deleted_and_untracked_paths() {
    let repo = Repo::new();
    for (path, body) in [
        ("shared old.txt", "shared\n"),
        ("delete me.txt", "delete\n"),
        ("staged.txt", "base\n"),
    ] {
        write(&repo.root.join(path), body);
    }
    git(&repo.root, &["add", "."]);
    git(&repo.root, &["commit", "-m", "shared fixtures"]);

    let alpha = repo.start("alpha");
    let beta = repo.start("beta");
    let gamma = repo.start("gamma");

    git(&alpha, &["mv", "shared old.txt", "shared new.txt"]);
    git(&alpha, &["commit", "-m", "rename shared file"]);
    fs::remove_file(alpha.join("delete me.txt")).unwrap();
    write(&alpha.join("staged.txt"), "alpha staged\n");
    git(&alpha, &["add", "staged.txt"]);
    write(&alpha.join("common untracked.txt"), "alpha\n");

    write(&beta.join("shared old.txt"), "beta\n");
    write(&beta.join("delete me.txt"), "beta\n");
    write(&beta.join("staged.txt"), "beta staged\n");
    write(&beta.join("common untracked.txt"), "beta\n");

    write(&gamma.join("gamma only.txt"), "gamma\n");

    let status: Value = serde_json::from_str(&run_ok(&repo.root, &["status", "--json"])).unwrap();
    let tasks = status["tasks"].as_array().unwrap();
    let alpha_status = tasks.iter().find(|task| task["id"] == "alpha").unwrap();
    let beta_status = tasks.iter().find(|task| task["id"] == "beta").unwrap();
    let gamma_status = tasks.iter().find(|task| task["id"] == "gamma").unwrap();

    let alpha_files = alpha_status["changed_files"].as_array().unwrap();
    for expected in [
        "shared old.txt",
        "shared new.txt",
        "delete me.txt",
        "staged.txt",
        "common untracked.txt",
    ] {
        assert!(
            alpha_files.iter().any(|path| path == expected),
            "missing {expected}"
        );
    }
    let alpha_overlap = &alpha_status["overlaps"][0];
    assert_eq!(alpha_overlap["task_id"], "beta");
    for expected in [
        "shared old.txt",
        "delete me.txt",
        "staged.txt",
        "common untracked.txt",
    ] {
        assert!(
            alpha_overlap["paths"]
                .as_array()
                .unwrap()
                .iter()
                .any(|path| path == expected),
            "missing overlap {expected}"
        );
    }
    assert_eq!(beta_status["overlaps"][0]["task_id"], "alpha");
    assert!(gamma_status["overlaps"].as_array().unwrap().is_empty());
    let human = run_ok(&repo.root, &["status"]);
    assert!(human.contains("overlaps beta"));
    assert!(!human.contains("conflict") || human.contains("Warning"));
}

#[test]
fn status_and_merge_preview_report_source_advancement_and_confirmed_conflict() {
    let repo = Repo::new();
    let source_task = repo.start("source-change");
    let pending = repo.start("pending-change");

    write(&source_task.join("README.md"), "source version\n");
    run_ok(
        &repo.root,
        &["merge", "source-change", "--message", "source change"],
    );
    write(&pending.join("README.md"), "task version\n");
    git(&pending, &["add", "README.md"]);
    git(&pending, &["commit", "-m", "task change"]);

    let status: Value =
        serde_json::from_str(&run_ok(&repo.root, &["status", "pending-change", "--json"])).unwrap();
    let task = &status["tasks"][0];
    assert_eq!(task["divergence"]["source_state"], "advanced");
    assert_eq!(task["divergence"]["source_ahead_of_base"], 1);
    assert_eq!(task["divergence"]["task_relation_to_source"], "diverged");
    assert!(
        task["warnings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|warning| warning["code"] == "confirmed-merge-conflict")
    );

    let preview: Value = serde_json::from_str(&run_ok(
        &repo.root,
        &[
            "merge",
            "pending-change",
            "--strategy",
            "preserve",
            "--dry-run",
            "--json",
        ],
    ))
    .unwrap();
    assert_eq!(preview["divergence"]["source_state"], "advanced");
    assert!(
        preview["confirmed_conflicts"]
            .as_array()
            .unwrap()
            .iter()
            .any(|path| path == "README.md")
    );
}

#[test]
fn sequential_parallel_tasks_report_advancement_without_forcing_rebase() {
    let repo = Repo::new();
    let first = repo.start("first-task");
    let second = repo.start("second-task");
    write(&first.join("first.txt"), "first\n");
    write(&second.join("second.txt"), "second\n");
    run_ok(&repo.root, &["merge", "first-task"]);

    let preview: Value = serde_json::from_str(&run_ok(
        &repo.root,
        &["merge", "second-task", "--dry-run", "--json"],
    ))
    .unwrap();
    assert_eq!(preview["divergence"]["source_state"], "advanced");
    assert_eq!(preview["divergence"]["task_relation_to_source"], "behind");
    let task_head = git(&second, &["rev-parse", "HEAD"]);
    run_ok(&repo.root, &["merge", "second-task"]);
    assert_eq!(git(&second, &["rev-parse", "HEAD^1"]), task_head);
    assert!(repo.root.join("first.txt").exists());
    assert!(repo.root.join("second.txt").exists());
}
