use crate::common::{Repo, run, run_fail, run_ok};
use serde_json::Value;

#[test]
fn public_help_is_focused_on_the_seven_user_commands() {
    let repo = Repo::new();
    let help = run_ok(&repo.root, &["--help"]);
    for command in [
        "setup", "start", "relay", "merge", "status", "clean", "recover",
    ] {
        assert!(help.contains(command), "missing {command} in help:\n{help}");
    }
    for obsolete in [
        "checkpoint",
        "handoff",
        "land",
        "pr-body",
        "doctor",
        "codex",
        "inspect",
    ] {
        assert!(
            !help.contains(obsolete),
            "obsolete command leaked: {obsolete}"
        );
    }
    assert!(
        !help.contains("report"),
        "internal report command must remain hidden"
    );
}

#[test]
fn obsolete_commands_are_rejected() {
    let repo = Repo::new();
    for command in ["run", "checkpoint", "handoff", "land", "plan", "pr-body"] {
        let output = run(&repo.root, &[command]);
        assert!(
            !output.status.success(),
            "obsolete command {command} succeeded"
        );
    }
}

#[test]
fn status_json_has_schema_v2_and_factual_state() {
    let repo = Repo::new();
    repo.start("facts");
    let value: Value =
        serde_json::from_str(&run_ok(&repo.root, &["status", "facts", "--json"])).unwrap();
    assert_eq!(value["schema_version"], 2);
    assert_eq!(value["tasks"][0]["state"], "created");
    assert_eq!(value["tasks"][0]["dirty"], false);
    assert!(value["tasks"][0]["blockers"].is_array());
}

#[test]
fn intent_is_optional_and_task_id_is_the_deterministic_default() {
    let repo = Repo::new();
    run_ok(&repo.root, &["start", "auth-fix"]);
    let task: Value = serde_json::from_str(
        &std::fs::read_to_string(repo.root.join(".girelay/tasks/auth-fix.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(task["intent"], "auth-fix");
    assert_eq!(task["intent_source"], "task-id");

    run_ok(
        &repo.root,
        &[
            "start",
            "docs-fix",
            "--intent",
            "Improve agent docs",
            "--",
            "git",
            "status",
            "--short",
        ],
    );
    let explicit: Value = serde_json::from_str(
        &std::fs::read_to_string(repo.root.join(".girelay/tasks/docs-fix.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(explicit["intent"], "Improve agent docs");
    assert_eq!(explicit["intent_source"], "explicit");
}

#[test]
fn invalid_and_duplicate_task_ids_are_rejected() {
    let repo = Repo::new();
    let error = run_fail(&repo.root, &["start", "../bad", "--intent", "bad"]);
    assert!(error.contains("invalid task id"));
    repo.start("same");
    let error = run_fail(&repo.root, &["start", "same", "--intent", "again"]);
    assert!(error.contains("already exists"));
    let error = run_fail(&repo.root, &["start", "empty", "--intent", "   "]);
    assert!(error.contains("intent cannot be empty"));
}

#[test]
fn local_setup_writes_only_excluded_skill_artifacts() {
    let repo = Repo::new();
    run_ok(&repo.root, &["setup", "codex", "--local"]);
    run_ok(&repo.root, &["setup", "claude", "--local"]);
    run_ok(&repo.root, &["setup", "pi", "--local"]);
    let codex = repo.root.join(".girelay/skills/codex/SKILL.md");
    let claude = repo.root.join(".girelay/skills/claude/SKILL.md");
    let pi = repo.root.join(".girelay/skills/pi/SKILL.md");
    assert!(codex.exists());
    assert!(claude.exists());
    assert!(pi.exists());
    for (agent, path) in [("codex", codex), ("claude", claude), ("pi", pi)] {
        let skill = std::fs::read_to_string(path).unwrap();
        for requirement in [
            "Read `GIRELAY_INTENT`",
            "git status --short --branch",
            "verify its claims against current files",
            "completed work from\nremaining work",
            "failed approaches",
            "commands actually tested",
            "including a blocker or partial result",
            "still submit the report",
            "reported-by-agent",
        ] {
            assert!(
                skill.contains(requirement),
                "{agent} skill is missing requirement: {requirement}"
            );
        }
        assert!(skill.contains(&format!("# Girelay protocol for {agent}")));
        assert!(skill.contains(&format!("\"agent\": \"{agent}\"")));
    }
    assert_eq!(crate::common::git(&repo.root, &["status", "--short"]), "");
    assert!(!repo.root.join("CLAUDE.md").exists());
    assert!(!repo.root.join("AGENTS.md").exists());
}

#[test]
fn user_setup_targets_agent_specific_skill_directories() {
    let repo = Repo::new();
    let home = tempfile::TempDir::new().unwrap();
    for agent in ["codex", "claude", "pi"] {
        let output = std::process::Command::new(crate::common::girelay())
            .args(["setup", agent])
            .env("HOME", home.path())
            .current_dir(&repo.root)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    assert!(home.path().join(".codex/skills/girelay/SKILL.md").exists());
    assert!(home.path().join(".claude/skills/girelay/SKILL.md").exists());
    assert!(
        home.path()
            .join(".pi/agent/skills/girelay/SKILL.md")
            .exists()
    );
}

#[test]
fn pi_setup_respects_its_native_agent_directory_override() {
    let repo = Repo::new();
    let home = tempfile::TempDir::new().unwrap();
    let pi_dir = home.path().join("managed-pi-profile");
    let output = std::process::Command::new(crate::common::girelay())
        .args(["setup", "pi"])
        .env("HOME", home.path())
        .env("PI_CODING_AGENT_DIR", &pi_dir)
        .current_dir(&repo.root)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(pi_dir.join("skills/girelay/SKILL.md").exists());
    assert!(!home.path().join(".pi/agent/skills/girelay").exists());
}
