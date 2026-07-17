use crate::common::{Repo, girelay, run_fail, run_ok};
use serde_json::{Value, json};
use std::fs;

#[test]
fn agent_can_submit_a_session_bound_semantic_report() {
    let repo = Repo::new();
    let binary = girelay();
    let script = format!(
        r#"report="${{TMPDIR:-/tmp}}/girelay-report-$$.json"
printf '{{"schema_version":2,"task_id":"%s","session_id":"%s","agent":"sh","start_snapshot":"%s","end_snapshot":null,"summary":"implemented change","completed":["change"],"remaining":[],"decisions":[],"failed_attempts":[],"blockers":[],"tests":["test command"],"risks":[],"next_action":"review","trust":"reported-by-agent"}}' "$GIRELAY_TASK_ID" "$GIRELAY_SESSION_ID" "$GIRELAY_START_SNAPSHOT" > "$report"
"{}" report --session "$GIRELAY_SESSION_ID" --file "$report"
rm -f "$report""#,
        binary.display()
    );
    run_ok(
        &repo.root,
        &[
            "start", "reported", "--intent", "report", "--", "sh", "-c", &script,
        ],
    );
    let task: Value = serde_json::from_str(
        &fs::read_to_string(repo.root.join(".girelay/tasks/reported.json")).unwrap(),
    )
    .unwrap();
    let session_id = task["latest_session_id"].as_str().unwrap();
    let report: Value = serde_json::from_str(
        &fs::read_to_string(
            repo.root
                .join(".girelay/reports/reported")
                .join(format!("{session_id}.json")),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(report["summary"], "implemented change");
    assert_eq!(report["trust"], "reported-by-agent");
    crate::v2_contracts::assert_schema(
        &report,
        &crate::v2_contracts::schema("report.schema.json"),
        "report",
    );
    let status: Value =
        serde_json::from_str(&run_ok(&repo.root, &["status", "reported", "--json"])).unwrap();
    assert_eq!(status["tasks"][0]["report_available"], true);
}

#[test]
fn report_is_rejected_outside_an_active_session() {
    let repo = Repo::new();
    repo.start("inactive");
    let path = repo.root.join("report.json");
    fs::write(
        &path,
        serde_json::to_vec(&json!({
            "schema_version": 2, "task_id": "inactive", "session_id": "fake", "agent": "sh",
            "start_snapshot": "fake", "end_snapshot": null, "summary": "fake", "completed": [],
            "remaining": [], "decisions": [], "failed_attempts": [], "blockers": [], "tests": [],
            "risks": [], "next_action": "none", "trust": "reported-by-agent"
        }))
        .unwrap(),
    )
    .unwrap();
    let error = run_fail(
        &repo.root,
        &[
            "report",
            "--session",
            "fake",
            "--file",
            path.to_str().unwrap(),
        ],
    );
    assert!(error.contains("not the active session"));
}

#[test]
fn next_session_receives_the_previous_report_path() {
    let repo = Repo::new();
    let binary = girelay();
    let script = format!(
        r#"report="${{TMPDIR:-/tmp}}/girelay-prior-$$.json"
printf '{{"schema_version":2,"task_id":"%s","session_id":"%s","agent":"sh","start_snapshot":"%s","end_snapshot":null,"summary":"first","completed":[],"remaining":[],"decisions":[],"failed_attempts":[],"blockers":[],"tests":[],"risks":[],"next_action":"continue","trust":"reported-by-agent"}}' "$GIRELAY_TASK_ID" "$GIRELAY_SESSION_ID" "$GIRELAY_START_SNAPSHOT" > "$report"
"{}" report --session "$GIRELAY_SESSION_ID" --file "$report"
rm -f "$report""#,
        binary.display()
    );
    run_ok(
        &repo.root,
        &[
            "start", "prior", "--intent", "prior", "--", "sh", "-c", &script,
        ],
    );
    run_ok(
        &repo.root,
        &[
            "relay",
            "prior",
            "--",
            "sh",
            "-c",
            "test -n \"$GIRELAY_PREVIOUS_REPORT\" && test -f \"$GIRELAY_PREVIOUS_REPORT\"",
        ],
    );
}

#[test]
fn duplicate_report_submission_is_rejected_as_immutable() {
    let repo = Repo::new();
    let binary = girelay();
    let script = format!(
        r#"report="${{TMPDIR:-/tmp}}/girelay-duplicate-$$.json"
printf '{{"schema_version":2,"task_id":"%s","session_id":"%s","agent":"sh","start_snapshot":"%s","end_snapshot":null,"summary":"first","completed":[],"remaining":[],"decisions":[],"failed_attempts":[],"blockers":[],"tests":[],"risks":[],"next_action":"continue","trust":"reported-by-agent"}}' "$GIRELAY_TASK_ID" "$GIRELAY_SESSION_ID" "$GIRELAY_START_SNAPSHOT" > "$report"
"{0}" report --session "$GIRELAY_SESSION_ID" --file "$report"
if "{0}" report --session "$GIRELAY_SESSION_ID" --file "$report" 2>"$report.err"; then exit 41; fi
grep -q immutable "$report.err"
rm -f "$report" "$report.err""#,
        binary.display()
    );
    run_ok(
        &repo.root,
        &[
            "start",
            "immutable",
            "--intent",
            "immutable",
            "--",
            "sh",
            "-c",
            &script,
        ],
    );
}

#[test]
fn report_missing_a_schema_required_field_is_rejected() {
    let repo = Repo::new();
    let binary = girelay();
    let script = format!(
        r#"report="${{TMPDIR:-/tmp}}/girelay-incomplete-$$.json"
printf '{{"schema_version":2,"task_id":"%s","session_id":"%s","agent":"sh","start_snapshot":"%s","end_snapshot":null,"summary":"incomplete","completed":[],"remaining":[],"decisions":[],"failed_attempts":[],"blockers":[],"risks":[],"next_action":"none","trust":"reported-by-agent"}}' "$GIRELAY_TASK_ID" "$GIRELAY_SESSION_ID" "$GIRELAY_START_SNAPSHOT" > "$report"
if "{0}" report --session "$GIRELAY_SESSION_ID" --file "$report" 2>"$report.err"; then exit 42; fi
grep -q "missing required field 'tests'" "$report.err"
rm -f "$report" "$report.err""#,
        binary.display()
    );
    run_ok(
        &repo.root,
        &[
            "start",
            "required",
            "--intent",
            "required fields",
            "--",
            "sh",
            "-c",
            &script,
        ],
    );
    let status: Value =
        serde_json::from_str(&run_ok(&repo.root, &["status", "required", "--json"])).unwrap();
    assert_eq!(status["tasks"][0]["report_available"], false);
}
