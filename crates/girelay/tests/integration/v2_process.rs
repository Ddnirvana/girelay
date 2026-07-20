use crate::common::{Repo, run, run_ok};
use serde_json::Value;
use std::fs;
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

#[test]
fn child_failure_code_and_redacted_command_are_recorded() {
    let repo = Repo::new();
    let output = run(
        &repo.root,
        &[
            "start",
            "failure",
            "--intent",
            "fail safely",
            "--",
            "sh",
            "-c",
            "exit 23",
            "--token",
            "super-secret",
        ],
    );
    assert_eq!(output.status.code(), Some(23));
    let task: Value = serde_json::from_str(
        &fs::read_to_string(repo.root.join(".girelay/tasks/failure.json")).unwrap(),
    )
    .unwrap();
    let session_id = task["latest_session_id"].as_str().unwrap();
    let body = fs::read_to_string(
        repo.root
            .join(".girelay/sessions/failure")
            .join(format!("{session_id}.json")),
    )
    .unwrap();
    assert!(!body.contains("super-secret"));
    assert!(body.contains("[REDACTED]"));
    let session: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(session["state"], "failed");
    assert_eq!(session["exit_code"], 23);
    assert!(session["end_snapshot"].is_string());
}

#[test]
fn missing_agent_is_recorded_and_returns_127() {
    let repo = Repo::new();
    let output = run(
        &repo.root,
        &[
            "start",
            "missing-agent",
            "--intent",
            "missing",
            "--",
            "definitely-no-agent-binary",
        ],
    );
    assert_eq!(output.status.code(), Some(127));
    let task: Value = serde_json::from_str(
        &fs::read_to_string(repo.root.join(".girelay/tasks/missing-agent.json")).unwrap(),
    )
    .unwrap();
    let session_id = task["latest_session_id"].as_str().unwrap();
    let session: Value = serde_json::from_str(
        &fs::read_to_string(
            repo.root
                .join(".girelay/sessions/missing-agent")
                .join(format!("{session_id}.json")),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(session["state"], "failed-to-start");
    assert_eq!(session["exit_code"], 127);
}

#[cfg(unix)]
#[test]
fn concurrent_session_is_refused_and_killed_parent_is_recoverable() {
    let repo = Repo::new();
    repo.start("locked");
    let mut child = Command::new(crate::common::girelay())
        .args(["relay", "locked", "--", "sleep", "10"])
        .current_dir(&repo.root)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let lock = repo.root.join(".girelay/locks/locked.lock");
    for _ in 0..100 {
        if lock.exists() {
            let task = fs::read_to_string(repo.root.join(".girelay/tasks/locked.json"))
                .ok()
                .and_then(|body| serde_json::from_str::<Value>(&body).ok());
            if task
                .as_ref()
                .is_some_and(|value| value["active_session_id"].is_string())
            {
                break;
            }
        }
        thread::sleep(Duration::from_millis(20));
    }
    assert!(lock.exists());
    let active: Value = serde_json::from_str(
        &fs::read_to_string(repo.root.join(".girelay/tasks/locked.json")).unwrap(),
    )
    .unwrap();
    assert!(active["active_session_id"].is_string());
    let second = run(&repo.root, &["relay", "locked", "--", "git", "status"]);
    assert_eq!(second.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&second.stderr).contains("active or stale operation"));

    let inspection: Value = serde_json::from_str(&run_ok(
        &repo.root,
        &["recover", "unlock", "locked", "--json"],
    ))
    .unwrap();
    assert_eq!(inspection["operation"], "agent-session");
    assert_eq!(inspection["recoverable"], false);
    let human = run_ok(&repo.root, &["recover", "unlock", "locked"]);
    assert!(human.contains("No unlock is available"));
    assert!(!human.contains("--confirm"));
    let refusal =
        crate::common::run_fail(&repo.root, &["recover", "unlock", "locked", "--confirm"]);
    assert!(refusal.contains("still alive"));

    let child_pid = inspection["child_pid"].as_u64().unwrap().to_string();
    child.kill().unwrap();
    child.wait().unwrap();
    Command::new("kill").arg(child_pid).status().unwrap();
    thread::sleep(Duration::from_millis(100));
    assert!(lock.exists());
    run_ok(&repo.root, &["recover", "unlock", "locked", "--confirm"]);
    run_ok(
        &repo.root,
        &["relay", "locked", "--", "git", "status", "--short"],
    );
    let sessions = repo.root.join(".girelay/sessions/locked");
    let states: Vec<String> = fs::read_dir(sessions)
        .unwrap()
        .map(|entry| {
            let value: Value =
                serde_json::from_str(&fs::read_to_string(entry.unwrap().path()).unwrap()).unwrap();
            value["state"].as_str().unwrap().to_string()
        })
        .collect();
    assert!(states.contains(&"interrupted".to_string()));
    assert!(states.contains(&"completed".to_string()));
    assert!(!lock.exists());
}

#[cfg(unix)]
#[test]
fn cleanup_stale_recovery_closes_interrupted_session_metadata() {
    let repo = Repo::new();
    let workspace = repo.start("clean-stale");
    let mut child = Command::new(crate::common::girelay())
        .args(["relay", "clean-stale", "--", "sleep", "10"])
        .current_dir(&repo.root)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let task_path = repo.root.join(".girelay/tasks/clean-stale.json");
    let lock_path = repo.root.join(".girelay/locks/clean-stale.lock");
    for _ in 0..100 {
        let published = fs::read_to_string(&task_path)
            .ok()
            .and_then(|body| serde_json::from_str::<Value>(&body).ok())
            .is_some_and(|value| value["active_session_id"].is_string());
        let child_recorded = fs::read_to_string(&lock_path)
            .ok()
            .and_then(|body| serde_json::from_str::<Value>(&body).ok())
            .is_some_and(|value| value["child_pid"].is_u64());
        if published && child_recorded {
            break;
        }
        thread::sleep(Duration::from_millis(20));
    }
    child.kill().unwrap();
    child.wait().unwrap();
    let lock: Value = serde_json::from_str(&fs::read_to_string(lock_path).unwrap()).unwrap();
    Command::new("kill")
        .arg(lock["child_pid"].as_u64().unwrap().to_string())
        .status()
        .unwrap();
    thread::sleep(Duration::from_millis(100));
    run_ok(
        &repo.root,
        &["recover", "unlock", "clean-stale", "--confirm"],
    );
    run_ok(&repo.root, &["clean", "clean-stale"]);
    assert!(!workspace.exists());
    let task: Value = serde_json::from_str(&fs::read_to_string(task_path).unwrap()).unwrap();
    assert!(task["active_session_id"].is_null());
    let session_id = task["latest_session_id"].as_str().unwrap();
    let session: Value = serde_json::from_str(
        &fs::read_to_string(
            repo.root
                .join(".girelay/sessions/clean-stale")
                .join(format!("{session_id}.json")),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(session["state"], "interrupted");
}

#[test]
fn stale_non_session_locks_are_recovered_through_one_operation() {
    let repo = Repo::new();
    for (task_id, operation) in [
        ("stale-merge", "merge"),
        ("stale-cleanup", "cleanup"),
        ("stale-recovery", "source-recovery"),
    ] {
        repo.start(task_id);
        let lock = repo.root.join(format!(".girelay/locks/{task_id}.lock"));
        let body = format!(
            r#"{{
  "schema_version": 1,
  "operation": "{operation}",
  "parent_pid": 4294967295,
  "child_pid": null,
  "created_at": "1"
}}"#
        );
        fs::write(&lock, body).unwrap();
        let inspected: Value = serde_json::from_str(&run_ok(
            &repo.root,
            &["recover", "unlock", task_id, "--json"],
        ))
        .unwrap();
        assert_eq!(inspected["operation"], operation);
        assert_eq!(inspected["recoverable"], true);
        assert!(lock.exists());
        let output = run_ok(&repo.root, &["recover", "unlock", task_id, "--confirm"]);
        assert!(output.contains(&format!("Unlocked stale {operation} lock")));
        assert!(!output.contains("interrupted session state was preserved"));
        assert!(!lock.exists());
    }
}

#[test]
fn failed_stale_session_repair_restores_the_original_lock() {
    let repo = Repo::new();
    repo.start("repair-failure");
    let task_path = repo.root.join(".girelay/tasks/repair-failure.json");
    let mut task: Value = serde_json::from_str(&fs::read_to_string(&task_path).unwrap()).unwrap();
    task["active_session_id"] = Value::String("missing-session".into());
    fs::write(&task_path, serde_json::to_vec_pretty(&task).unwrap()).unwrap();

    let lock = repo.root.join(".girelay/locks/repair-failure.lock");
    let original = r#"{
  "schema_version": 1,
  "operation": "agent-session",
  "parent_pid": 4294967295,
  "child_pid": null,
  "created_at": "1"
}"#;
    fs::write(&lock, original).unwrap();
    let error = crate::common::run_fail(
        &repo.root,
        &["recover", "unlock", "repair-failure", "--confirm"],
    );
    assert!(error.contains("missing-session"));
    assert_eq!(fs::read_to_string(&lock).unwrap(), original);
    assert_eq!(
        fs::read_dir(repo.root.join(".girelay/locks"))
            .unwrap()
            .count(),
        1
    );
}
