use crate::{git, task};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub workspace: WorkspaceConfig,
    pub merge: MergeConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct WorkspaceConfig {
    pub root: String,
    pub base: String,
    pub branch_prefix: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct MergeConfig {
    pub run_checks: bool,
    pub check_commands: Vec<String>,
}

impl Default for WorkspaceConfig {
    fn default() -> Self {
        Self {
            root: ".girelay/workspaces".into(),
            base: "main".into(),
            branch_prefix: "agent/".into(),
        }
    }
}

impl Default for MergeConfig {
    fn default() -> Self {
        Self {
            run_checks: true,
            check_commands: Vec::new(),
        }
    }
}

pub fn ensure_layout(repo: &Path) -> Result<()> {
    for directory in ["tasks", "sessions", "reports", "locks", "archive", "tmp"] {
        fs::create_dir_all(repo.join(".girelay").join(directory))?;
    }
    ensure_git_exclude(repo)?;
    let config_path = repo.join(".girelay/config.toml");
    if !config_path.exists() {
        task::atomic_write(
            &config_path,
            toml::to_string_pretty(&Config::default())?.as_bytes(),
        )?;
    }
    Ok(())
}

pub fn ensure_git_exclude(repo: &Path) -> Result<()> {
    let git_dir = git::git_common_dir(repo)?;
    let exclude_path = git_dir.join("info/exclude");
    if let Some(parent) = exclude_path.parent() {
        fs::create_dir_all(parent)?;
    }
    let existing = fs::read_to_string(&exclude_path).unwrap_or_default();
    if !existing.lines().any(|line| line.trim() == ".girelay/") {
        let mut next = existing;
        if !next.is_empty() && !next.ends_with('\n') {
            next.push('\n');
        }
        next.push_str(".girelay/\n");
        task::atomic_write(&exclude_path, next.as_bytes())?;
    }
    Ok(())
}

pub fn load(repo: &Path) -> Result<Config> {
    let path = repo.join(".girelay/config.toml");
    if !path.exists() {
        return Ok(Config::default());
    }
    toml::from_str(&fs::read_to_string(&path)?)
        .with_context(|| format!("failed to parse {}", path.display()))
}

pub fn workspace_root(repo: &Path, config: &Config) -> PathBuf {
    let root = PathBuf::from(&config.workspace.root);
    if root.is_absolute() {
        root
    } else {
        repo.join(root)
    }
}
