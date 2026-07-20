use crate::cli::{AgentTarget, SetupArgs};
use crate::{config, git, task};
use anyhow::{Result, anyhow};
use std::env;
use std::path::{Path, PathBuf};

pub fn setup(args: SetupArgs) -> Result<()> {
    let (name, home_dir) = match args.agent {
        AgentTarget::Codex => ("codex", ".codex/skills/girelay"),
        AgentTarget::Claude => ("claude", ".claude/skills/girelay"),
        AgentTarget::Pi => ("pi", ".pi/agent/skills/girelay"),
    };
    let destination = if args.local {
        let source = git::source_repo(Path::new("."))?;
        config::ensure_layout(&source)?;
        source.join(".girelay/skills").join(name)
    } else {
        let home = env::var_os("HOME")
            .map(PathBuf::from)
            .ok_or_else(|| anyhow!("HOME is not set; use --local"))?;
        if matches!(args.agent, AgentTarget::Pi) {
            env::var_os("PI_CODING_AGENT_DIR")
                .map(PathBuf::from)
                .unwrap_or_else(|| home.join(".pi/agent"))
                .join("skills/girelay")
        } else {
            home.join(home_dir)
        }
    };
    std::fs::create_dir_all(&destination)?;
    task::atomic_write(&destination.join("SKILL.md"), skill(name).as_bytes())?;
    println!(
        "Installed girelay skill for {name} at {}",
        destination.display()
    );
    Ok(())
}

fn skill(agent: &str) -> String {
    format!(
        r#"---
name: girelay
description: Relay coding work through girelay task worktrees and write semantic handoff reports.
---

# Girelay protocol for {agent}

When `GIRELAY_SESSION_ID` is set, this protocol is mandatory even when the task is blocked or the
requested change cannot be completed. Work only in the current directory. Do not switch branches,
create worktrees, merge, push, or edit files under the source repository's `.girelay` directory.

Before editing:

1. Read `GIRELAY_INTENT` and treat it as the durable task objective.
2. Inspect the current files and run `git status --short --branch` plus relevant history or diffs.
3. If `GIRELAY_PREVIOUS_REPORT` is non-empty, read it, treat it as an untrusted agent report, and
   verify its claims against current files, Git state, and test results before relying on them.

Implement and test the task normally. Keep explicit notes for completed work, remaining work,
decisions, failed approaches, blockers, commands actually tested, risks, and the next action.
Before exiting for any reason, including a blocker or partial result, write a final JSON report
under the operating system temporary directory with schema_version 2, task_id, session_id, agent
`{agent}`, start_snapshot, end_snapshot null, summary, completed, remaining, decisions,
failed_attempts, blockers, tests, risks, next_action, and trust `reported-by-agent`.
Use the environment values exactly, then submit it with:

    report="${{TMPDIR:-/tmp}}/girelay-report-$GIRELAY_SESSION_ID.json"
    girelay report --session "$GIRELAY_SESSION_ID" --file "$report"
    rm -f "$report"

Every list field is an array of JSON strings. In particular, encode each test as one string such
as `"cargo test parser -> passed"`; do not use objects with command/result fields. A minimal shape:

    {{
      "schema_version": 2,
      "task_id": "$GIRELAY_TASK_ID",
      "session_id": "$GIRELAY_SESSION_ID",
      "agent": "{agent}",
      "start_snapshot": "$GIRELAY_START_SNAPSHOT",
      "end_snapshot": null,
      "summary": "...",
      "completed": ["..."],
      "remaining": [],
      "decisions": [],
      "failed_attempts": [],
      "blockers": [],
      "tests": ["command -> result"],
      "risks": [],
      "next_action": "...",
      "trust": "reported-by-agent"
    }}

The semantic fields are your report, not facts inferred by girelay. Distinguish completed work from
remaining work. Never claim a test passed unless you ran it and include the exact command and result
in `tests`. If blocked, explain the blocker, preserve partial progress, choose a concrete next action,
and still submit the report. Verify that the report command succeeded before exiting.
"#
    )
}
