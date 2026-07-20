use crate::{git, task};
use anyhow::{Context, Result, anyhow};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;
use std::process::Command;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Warning {
    pub code: String,
    pub message: String,
    pub evidence: Vec<String>,
    pub next_action: String,
}

impl Warning {
    pub fn new(
        code: &str,
        message: impl Into<String>,
        evidence: Vec<String>,
        next_action: impl Into<String>,
    ) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            evidence,
            next_action: next_action.into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct TaskOverlap {
    pub task_id: String,
    pub paths: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Divergence {
    pub source_state: String,
    pub base_commit: String,
    pub source_tip: Option<String>,
    pub source_ahead_of_base: u64,
    pub source_behind_base: u64,
    pub task_tip: Option<String>,
    pub task_relation_to_source: String,
    pub task_ahead_of_source: u64,
    pub task_behind_source: u64,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct CommitSummary {
    pub commit: String,
    pub subject: String,
}

pub fn task_changed_paths(source: &Path, record: &task::Task) -> Result<Vec<String>> {
    let mut paths = BTreeSet::new();
    let branch_ref = format!("refs/heads/{}", record.branch);
    if git::ref_exists(source, &branch_ref) {
        let tip = git::head_commit(source, &branch_ref)?;
        paths.extend(git::changed_files(source, &record.base_commit, &tip)?);
    }
    if record.workspace_path.is_dir() {
        paths.extend(git::working_tree_files(&record.workspace_path)?);
    }
    Ok(paths.into_iter().collect())
}

pub fn overlaps(
    records: &[task::Task],
    changed: &BTreeMap<String, Vec<String>>,
) -> BTreeMap<String, Vec<TaskOverlap>> {
    let active: Vec<_> = records
        .iter()
        .filter(|record| record.lifecycle == task::TaskLifecycle::Active)
        .collect();
    let mut result: BTreeMap<String, Vec<TaskOverlap>> = BTreeMap::new();
    for (index, left) in active.iter().enumerate() {
        let left_paths: BTreeSet<_> = changed
            .get(&left.id)
            .into_iter()
            .flatten()
            .cloned()
            .collect();
        for right in active.iter().skip(index + 1) {
            let right_paths: BTreeSet<_> = changed
                .get(&right.id)
                .into_iter()
                .flatten()
                .cloned()
                .collect();
            let paths: Vec<_> = left_paths.intersection(&right_paths).cloned().collect();
            if paths.is_empty() {
                continue;
            }
            result
                .entry(left.id.clone())
                .or_default()
                .push(TaskOverlap {
                    task_id: right.id.clone(),
                    paths: paths.clone(),
                });
            result
                .entry(right.id.clone())
                .or_default()
                .push(TaskOverlap {
                    task_id: left.id.clone(),
                    paths,
                });
        }
    }
    result
}

pub fn divergence(source: &Path, record: &task::Task) -> Result<Divergence> {
    let source_ref = format!("refs/heads/{}", record.base_branch);
    let task_ref = format!("refs/heads/{}", record.branch);
    let source_tip = git::ref_exists(source, &source_ref)
        .then(|| git::head_commit(source, &source_ref))
        .transpose()?;
    let task_tip = git::ref_exists(source, &task_ref)
        .then(|| git::head_commit(source, &task_ref))
        .transpose()?;
    let (source_behind_base, source_ahead_of_base, source_state) = if let Some(tip) = &source_tip {
        let (behind, ahead) = git::rev_counts(source, &record.base_commit, tip)?;
        (behind, ahead, relation(behind, ahead))
    } else {
        (0, 0, "missing".into())
    };
    let (task_behind_source, task_ahead_of_source, task_relation_to_source) =
        match (&source_tip, &task_tip) {
            (Some(source_tip), Some(task_tip)) => {
                let (behind, ahead) = git::rev_counts(source, source_tip, task_tip)?;
                (behind, ahead, relation(behind, ahead))
            }
            _ => (0, 0, "missing".into()),
        };
    Ok(Divergence {
        source_state,
        base_commit: record.base_commit.clone(),
        source_tip,
        source_ahead_of_base,
        source_behind_base,
        task_tip,
        task_relation_to_source,
        task_ahead_of_source,
        task_behind_source,
    })
}

fn relation(left_only: u64, right_only: u64) -> String {
    match (left_only, right_only) {
        (0, 0) => "unchanged",
        (0, _) => "advanced",
        (_, 0) => "behind",
        _ => "diverged",
    }
    .into()
}

pub fn commit_summaries(source: &Path, record: &task::Task) -> Result<Vec<CommitSummary>> {
    let task_ref = format!("refs/heads/{}", record.branch);
    if !git::ref_exists(source, &task_ref) {
        return Ok(Vec::new());
    }
    let tip = git::head_commit(source, &task_ref)?;
    Ok(git::commit_summaries(source, &record.base_commit, &tip)?
        .into_iter()
        .map(|(commit, subject)| CommitSummary { commit, subject })
        .collect())
}

pub fn confirmed_conflicts(source: &Path, record: &task::Task) -> Result<Vec<String>> {
    let source_ref = format!("refs/heads/{}", record.base_branch);
    let task_ref = format!("refs/heads/{}", record.branch);
    if !git::ref_exists(source, &source_ref) || !git::ref_exists(source, &task_ref) {
        return Ok(Vec::new());
    }
    let source_tip = git::head_commit(source, &source_ref)?;
    let task_tip = git::head_commit(source, &task_ref)?;
    let base = git::merge_base(source, &source_tip, &task_tip)?;
    let index = source
        .join(".girelay/tmp")
        .join(format!("merge-preflight-{}.index", task::unique_id()));
    if let Some(parent) = index.parent() {
        fs::create_dir_all(parent)?;
    }
    let result = inspect_conflicts(source, &index, &base, &source_tip, &task_tip);
    cleanup_temporary_index(&index);
    result
}

fn cleanup_temporary_index(index: &Path) {
    let _ = fs::remove_file(index);
    let _ = fs::remove_file(index.with_extension("index.lock"));
}

fn inspect_conflicts(
    source: &Path,
    index: &Path,
    base: &str,
    source_tip: &str,
    task_tip: &str,
) -> Result<Vec<String>> {
    let read = Command::new("git")
        .args(["read-tree", "-m", base, source_tip, task_tip])
        .env("GIT_INDEX_FILE", index)
        .current_dir(source)
        .output()
        .context("failed to run Git conflict preflight")?;
    if !read.status.success() {
        return Err(anyhow!(
            "Git conflict preflight failed: {}",
            String::from_utf8_lossy(&read.stderr).trim()
        ));
    }
    let unresolved = Command::new("git")
        .args(["ls-files", "-u", "-z"])
        .env("GIT_INDEX_FILE", index)
        .current_dir(source)
        .output()
        .context("failed to inspect Git conflict preflight")?;
    if !unresolved.status.success() {
        return Err(anyhow!(
            "Git conflict inspection failed: {}",
            String::from_utf8_lossy(&unresolved.stderr).trim()
        ));
    }
    let mut paths = BTreeSet::new();
    for entry in unresolved.stdout.split(|byte| *byte == 0) {
        if let Some(index) = entry.iter().position(|byte| *byte == b'\t') {
            paths.insert(String::from_utf8_lossy(&entry[index + 1..]).to_string());
        }
    }
    Ok(paths.into_iter().collect())
}

pub fn divergence_warning(task_id: &str, value: &Divergence) -> Option<Warning> {
    (value.source_state != "unchanged").then(|| {
        Warning::new(
            "source-divergence",
            format!("source branch state is '{}'", value.source_state),
            vec![
                format!("source_ahead_of_base={}", value.source_ahead_of_base),
                format!("source_behind_base={}", value.source_behind_base),
                format!("task_relation={}", value.task_relation_to_source),
            ],
            format!("review `girelay merge {task_id} --dry-run` before integration"),
        )
    })
}

pub fn overlap_warnings(task_id: &str, values: &[TaskOverlap]) -> Vec<Warning> {
    values
        .iter()
        .map(|overlap| {
            Warning::new(
                "path-overlap",
                format!("task overlaps paths changed by '{}'", overlap.task_id),
                overlap.paths.clone(),
                format!(
                    "review both task diffs before choosing merge order for {task_id} and {}",
                    overlap.task_id
                ),
            )
        })
        .collect()
}
