use crate::cli::{AgentTarget, SetupArgs};
use crate::{config, git, task};
use anyhow::{Result, anyhow};
use std::env;
use std::path::{Path, PathBuf};

pub fn setup(args: SetupArgs) -> Result<()> {
    let (name, home_dir) = match args.agent {
        AgentTarget::Codex => ("codex", ".codex/skills/girelay"),
        AgentTarget::Claude => ("claude", ".claude/skills/girelay"),
    };
    let destination = if args.local {
        let source = git::source_repo(Path::new("."))?;
        config::ensure_layout(&source)?;
        source.join(".girelay/skills").join(name)
    } else {
        let home = env::var_os("HOME")
            .map(PathBuf::from)
            .ok_or_else(|| anyhow!("HOME is not set; use --local"))?;
        home.join(home_dir)
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

When GIRELAY_SESSION_ID is set, work only in the current directory. Do not switch branches,
create worktrees, merge, push, or edit files under the source repository's `.girelay` directory.

Before changing code, inspect `GIRELAY_INTENT`. If `GIRELAY_PREVIOUS_REPORT` is set, read that
report first and verify its reported claims against the current files and Git state.

Implement and test the task normally. Before exiting, write a JSON report under the operating
system temporary directory with schema_version 2, task_id, session_id, agent `{agent}`,
start_snapshot, end_snapshot null, summary, completed, remaining, decisions, failed_attempts,
blockers, tests, risks, next_action, and trust `reported-by-agent`.
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

The semantic fields are your report, not facts inferred by girelay. Never claim a test passed
unless you ran it and include the exact command in `tests`.
"#
    )
}
