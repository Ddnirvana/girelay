use crate::common::{Repo, git, run_ok, write};
use serde_json::Value;
use std::fs;

#[test]
fn start_creates_a_native_linked_worktree_inside_the_source() {
    let repo = Repo::new();
    let source_head = git(&repo.root, &["rev-parse", "HEAD"]);
    let workspace = repo.start("native");
    assert!(
        workspace.join(".git").is_file(),
        "linked worktree .git should be a file"
    );
    assert_eq!(
        git(&workspace, &["branch", "--show-current"]),
        "agent/native"
    );
    assert_eq!(git(&workspace, &["rev-parse", "HEAD"]), source_head);
    assert_eq!(git(&repo.root, &["status", "--short"]), "");
    let task: Value = serde_json::from_str(
        &fs::read_to_string(repo.root.join(".girelay/tasks/native.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(task["schema_version"], 2);
    assert_eq!(
        fs::canonicalize(task["workspace_path"].as_str().unwrap()).unwrap(),
        fs::canonicalize(&workspace).unwrap()
    );
    assert!(
        !workspace.join(".girelay/task.json").exists(),
        "metadata must remain source-owned"
    );
}

#[test]
fn start_can_launch_the_first_session_and_capture_snapshots() {
    let repo = Repo::new();
    run_ok(
        &repo.root,
        &[
            "start",
            "first",
            "--intent",
            "write output",
            "--",
            "sh",
            "-c",
            "printf hello > hello.txt",
        ],
    );
    let workspace = repo.workspace("first");
    assert_eq!(
        fs::read_to_string(workspace.join("hello.txt")).unwrap(),
        "hello"
    );
    let task: Value = serde_json::from_str(
        &fs::read_to_string(repo.root.join(".girelay/tasks/first.json")).unwrap(),
    )
    .unwrap();
    let session_id = task["latest_session_id"].as_str().unwrap();
    let session: Value = serde_json::from_str(
        &fs::read_to_string(
            repo.root
                .join(".girelay/sessions/first")
                .join(format!("{session_id}.json")),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(session["state"], "completed");
    assert_eq!(session["trust"]["git_state"], "observed-by-girelay");
    assert!(session["start_snapshot"].as_str().unwrap().len() >= 40);
    assert!(session["end_snapshot"].as_str().unwrap().len() >= 40);
    let refs = git(
        &repo.root,
        &[
            "for-each-ref",
            "--format=%(refname)",
            "refs/girelay/snapshots/first/",
        ],
    );
    assert!(refs.contains("/start"));
    assert!(refs.contains("/end"));
    assert_eq!(
        git(&workspace, &["log", "--oneline", "main..HEAD"]),
        "",
        "snapshots must not alter task branch history"
    );
}

#[test]
fn relay_reuses_the_worktree_and_records_distinct_sessions() {
    let repo = Repo::new();
    let workspace = repo.start("relay-task");
    run_ok(
        &repo.root,
        &[
            "relay",
            "relay-task",
            "--",
            "sh",
            "-c",
            "printf one > one.txt",
        ],
    );
    run_ok(
        &repo.root,
        &[
            "relay",
            "relay-task",
            "--",
            "sh",
            "-c",
            "printf two > two.txt",
        ],
    );
    assert!(workspace.join("one.txt").exists());
    assert!(workspace.join("two.txt").exists());
    assert_eq!(
        fs::read_dir(repo.root.join(".girelay/sessions/relay-task"))
            .unwrap()
            .count(),
        2
    );
    let state: Value =
        serde_json::from_str(&run_ok(&repo.root, &["status", "relay-task", "--json"])).unwrap();
    assert_eq!(state["tasks"][0]["state"], "paused");
}

#[test]
fn squash_merge_integrates_dirty_agent_work_and_records_rollbacks() {
    let repo = Repo::new();
    let workspace = repo.start("squash");
    write(&workspace.join("feature.txt"), "feature\n");
    let before = git(&repo.root, &["rev-parse", "HEAD"]);
    let result: Value = serde_json::from_str(&run_ok(
        &repo.root,
        &[
            "merge",
            "squash",
            "--strategy",
            "squash",
            "--message",
            "feat: squash",
            "--json",
        ],
    ))
    .unwrap();
    assert_eq!(result["strategy"], "squash");
    assert_eq!(
        fs::read_to_string(repo.root.join("feature.txt")).unwrap(),
        "feature\n"
    );
    assert_eq!(
        git(&repo.root, &["log", "-1", "--pretty=%s"]),
        "feat: squash"
    );
    assert_eq!(
        git(&repo.root, &["log", "-1", "--pretty=%an <%ae>"]),
        "Agent Test <agent@example.com>"
    );
    assert_ne!(git(&repo.root, &["rev-parse", "HEAD"]), before);
    assert!(
        git(
            &repo.root,
            &["show-ref", result["source_rollback_ref"].as_str().unwrap()]
        )
        .contains(&before)
    );
    let state: Value =
        serde_json::from_str(&run_ok(&repo.root, &["status", "squash", "--json"])).unwrap();
    assert_eq!(state["tasks"][0]["state"], "merged");
}

#[test]
fn preserve_merge_keeps_agent_commits() {
    let repo = Repo::new();
    let workspace = repo.start("preserve");
    write(&workspace.join("one.txt"), "one\n");
    git(&workspace, &["add", "one.txt"]);
    git(&workspace, &["commit", "-m", "agent: one"]);
    write(&workspace.join("two.txt"), "two\n");
    git(&workspace, &["add", "two.txt"]);
    git(&workspace, &["commit", "-m", "agent: two"]);
    run_ok(
        &repo.root,
        &[
            "merge",
            "preserve",
            "--strategy",
            "preserve",
            "--message",
            "merge preserve",
        ],
    );
    let history = git(&repo.root, &["log", "--pretty=%s", "-4"]);
    assert!(history.contains("merge preserve"));
    assert!(history.contains("agent: one"));
    assert!(history.contains("agent: two"));
}

#[test]
fn status_reports_both_sides_of_a_rename_with_spaces() {
    let repo = Repo::new();
    let workspace = repo.start("rename");
    write(&workspace.join("old name.txt"), "content\n");
    git(&workspace, &["add", "old name.txt"]);
    git(&workspace, &["commit", "-m", "add old name"]);
    git(&workspace, &["mv", "old name.txt", "new name.txt"]);
    let status: Value =
        serde_json::from_str(&run_ok(&repo.root, &["status", "rename", "--json"])).unwrap();
    let files = status["tasks"][0]["changed_files"].as_array().unwrap();
    assert!(files.iter().any(|value| value == "old name.txt"));
    assert!(files.iter().any(|value| value == "new name.txt"));
}
