use crate::common::{Repo, run_ok, write};
use serde_json::Value;
use std::fs;
use std::path::Path;

pub(crate) fn schema(name: &str) -> Value {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    serde_json::from_str(&fs::read_to_string(root.join("schemas").join(name)).unwrap()).unwrap()
}

pub(crate) fn assert_schema(value: &Value, schema: &Value, path: &str) {
    if let Some(constant) = schema.get("const") {
        assert_eq!(value, constant, "{path} violates const");
    }
    if let Some(values) = schema.get("enum").and_then(Value::as_array) {
        assert!(values.contains(value), "{path} violates enum: {value}");
    }
    if let Some(types) = schema.get("type") {
        let accepted: Vec<&str> = match types {
            Value::String(value) => vec![value],
            Value::Array(values) => values.iter().map(|value| value.as_str().unwrap()).collect(),
            _ => panic!("invalid type declaration at {path}"),
        };
        assert!(
            accepted.iter().any(|kind| matches_type(value, kind)),
            "{path} expected {accepted:?}, got {value}"
        );
    }
    if let Some(required) = schema.get("required").and_then(Value::as_array) {
        let object = value
            .as_object()
            .unwrap_or_else(|| panic!("{path} must be object"));
        for field in required {
            let field = field.as_str().unwrap();
            assert!(
                object.contains_key(field),
                "{path} missing {field}: {value}"
            );
        }
    }
    if let (Some(properties), Some(object)) = (
        schema.get("properties").and_then(Value::as_object),
        value.as_object(),
    ) {
        for (name, child_schema) in properties {
            if let Some(child) = object.get(name) {
                assert_schema(child, child_schema, &format!("{path}.{name}"));
            }
        }
    }
    if let (Some(items), Some(array)) = (schema.get("items"), value.as_array()) {
        for (index, child) in array.iter().enumerate() {
            assert_schema(child, items, &format!("{path}[{index}]"));
        }
    }
}

fn matches_type(value: &Value, kind: &str) -> bool {
    match kind {
        "array" => value.is_array(),
        "boolean" => value.is_boolean(),
        "integer" => value.as_i64().is_some(),
        "null" => value.is_null(),
        "object" => value.is_object(),
        "string" => value.is_string(),
        other => panic!("unsupported schema type {other}"),
    }
}

#[test]
fn runtime_json_and_metadata_match_all_v2_schemas() {
    let repo = Repo::new();
    let workspace = repo.start("contracts");
    run_ok(
        &repo.root,
        &[
            "relay",
            "contracts",
            "--",
            "sh",
            "-c",
            "printf contract > contract.txt",
        ],
    );

    let task: Value = serde_json::from_str(
        &fs::read_to_string(repo.root.join(".girelay/tasks/contracts.json")).unwrap(),
    )
    .unwrap();
    assert_schema(&task, &schema("task.schema.json"), "task");
    let session_id = task["latest_session_id"].as_str().unwrap();
    let session: Value = serde_json::from_str(
        &fs::read_to_string(
            repo.root
                .join(".girelay/sessions/contracts")
                .join(format!("{session_id}.json")),
        )
        .unwrap(),
    )
    .unwrap();
    assert_schema(&session, &schema("session.schema.json"), "session");

    let status: Value =
        serde_json::from_str(&run_ok(&repo.root, &["status", "contracts", "--json"])).unwrap();
    assert_schema(&status, &schema("status.schema.json"), "status");
    let clean_plan: Value = serde_json::from_str(&run_ok(
        &repo.root,
        &["clean", "contracts", "--dry-run", "--json"],
    ))
    .unwrap();
    assert_schema(&clean_plan, &schema("clean-plan.schema.json"), "clean-plan");

    let merged: Value =
        serde_json::from_str(&run_ok(&repo.root, &["merge", "contracts", "--json"])).unwrap();
    assert_schema(&merged, &schema("merge.schema.json"), "merge");
    let recoveries: Value = serde_json::from_str(&run_ok(
        &repo.root,
        &["recover", "list", "contracts", "--json"],
    ))
    .unwrap();
    assert_schema(&recoveries, &schema("recovery.schema.json"), "recovery");

    write(&workspace.join("archive-note.txt"), "archive\n");
    let output = run_ok(&repo.root, &["clean", "contracts", "--archive"]);
    let archive_id = output
        .lines()
        .find_map(|line| line.strip_prefix("Archive: "))
        .unwrap();
    let manifest: Value = serde_json::from_str(
        &fs::read_to_string(
            repo.root
                .join(".girelay/archive")
                .join(archive_id)
                .join("manifest.json"),
        )
        .unwrap(),
    )
    .unwrap();
    assert_schema(
        &manifest,
        &schema("archive-manifest.schema.json"),
        "archive",
    );
}

#[test]
fn every_published_schema_is_a_valid_draft_2020_document() {
    for name in [
        "archive-manifest.schema.json",
        "clean-plan.schema.json",
        "merge.schema.json",
        "recovery.schema.json",
        "report.schema.json",
        "session.schema.json",
        "status.schema.json",
        "task.schema.json",
    ] {
        let value = schema(name);
        assert_eq!(
            value["$schema"],
            "https://json-schema.org/draft/2020-12/schema"
        );
        assert!(
            value["required"].is_array(),
            "{name} has no required fields"
        );
    }
}

#[test]
fn task_metadata_missing_an_explicit_null_field_is_rejected() {
    let repo = Repo::new();
    repo.start("strict-task");
    let path = repo.root.join(".girelay/tasks/strict-task.json");
    let mut value: Value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    value.as_object_mut().unwrap().remove("active_session_id");
    fs::write(&path, serde_json::to_vec_pretty(&value).unwrap()).unwrap();
    let error = crate::common::run_fail(&repo.root, &["status", "strict-task"]);
    assert!(error.contains("missing required field 'active_session_id'"));
}
