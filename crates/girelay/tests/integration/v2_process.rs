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

    child.kill().unwrap();
    child.wait().unwrap();
    assert!(lock.exists());
    run_ok(
        &repo.root,
        &[
            "relay",
            "locked",
            "--recover-stale-session",
            "--",
            "git",
            "status",
            "--short",
        ],
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
    for _ in 0..100 {
        let published = fs::read_to_string(&task_path)
            .ok()
            .and_then(|body| serde_json::from_str::<Value>(&body).ok())
            .is_some_and(|value| value["active_session_id"].is_string());
        if published {
            break;
        }
        thread::sleep(Duration::from_millis(20));
    }
    child.kill().unwrap();
    child.wait().unwrap();
    run_ok(
        &repo.root,
        &["clean", "clean-stale", "--recover-stale-session"],
    );
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
