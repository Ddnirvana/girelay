use anyhow::{Context, Result, anyhow};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

#[derive(Debug)]
pub struct GitOutput {
    pub stdout: String,
}

pub fn run(repo: &Path, args: &[&str]) -> Result<GitOutput> {
    let output = Command::new("git")
        .args(args)
        .current_dir(repo)
        .output()
        .with_context(|| format!("failed to run git {}", args.join(" ")))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let detail = if stderr.is_empty() { stdout } else { stderr };
        return Err(anyhow!("git {} failed: {}", args.join(" "), detail));
    }
    Ok(GitOutput {
        stdout: String::from_utf8_lossy(&output.stdout)
            .trim_end()
            .to_string(),
    })
}

pub fn run_quiet(repo: &Path, args: &[&str]) -> Result<()> {
    run(repo, args).map(|_| ())
}

pub fn succeeds(repo: &Path, args: &[&str]) -> Result<bool> {
    let status = Command::new("git")
        .args(args)
        .current_dir(repo)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .with_context(|| format!("failed to run git {}", args.join(" ")))?;
    match status.code() {
        Some(0) => Ok(true),
        Some(_) => Ok(false),
        None => Err(anyhow!("git {} terminated by signal", args.join(" "))),
    }
}

pub fn repo_root(start: &Path) -> Result<PathBuf> {
    let output = run(start, &["rev-parse", "--show-toplevel"])?;
    Ok(PathBuf::from(output.stdout))
}

pub fn git_common_dir(repo: &Path) -> Result<PathBuf> {
    let path = run(
        repo,
        &["rev-parse", "--path-format=absolute", "--git-common-dir"],
    )?;
    Ok(PathBuf::from(path.stdout))
}

pub fn source_repo(repo: &Path) -> Result<PathBuf> {
    let common = git_common_dir(repo)?;
    let source = common
        .parent()
        .ok_or_else(|| anyhow!("Git common directory has no parent"))?;
    fs_canonical(source)
}

fn fs_canonical(path: &Path) -> Result<PathBuf> {
    std::fs::canonicalize(path).with_context(|| format!("failed to resolve {}", path.display()))
}

pub fn current_branch(repo: &Path) -> Result<String> {
    Ok(run(repo, &["branch", "--show-current"])?.stdout)
}

pub fn head_commit(repo: &Path, rev: &str) -> Result<String> {
    Ok(run(repo, &["rev-parse", rev])?.stdout)
}

pub fn is_dirty(repo: &Path) -> Result<bool> {
    Ok(!run(repo, &["status", "--porcelain"])?.stdout.is_empty())
}

pub fn working_tree_files(repo: &Path) -> Result<Vec<String>> {
    let output = working_tree_state(repo)?;
    let fields: Vec<&[u8]> = output.split(|byte| *byte == 0).collect();
    let mut files = Vec::new();
    let mut index = 0;
    while index < fields.len() {
        let field = fields[index];
        if field.is_empty() {
            break;
        }
        if field.len() < 4 || field[2] != b' ' {
            return Err(anyhow!("unexpected Git porcelain status record"));
        }
        let renamed = matches!(field[0], b'R' | b'C') || matches!(field[1], b'R' | b'C');
        files.push(String::from_utf8_lossy(&field[3..]).to_string());
        index += 1;
        if renamed {
            let original = fields
                .get(index)
                .filter(|value| !value.is_empty())
                .ok_or_else(|| anyhow!("Git rename status is missing its original path"))?;
            files.push(String::from_utf8_lossy(original).to_string());
            index += 1;
        }
    }
    files.sort();
    files.dedup();
    Ok(files)
}

pub fn working_tree_state(repo: &Path) -> Result<Vec<u8>> {
    let output = Command::new("git")
        .args(["status", "--porcelain=v1", "-z"])
        .current_dir(repo)
        .output()
        .context("failed to inspect working tree files")?;
    if !output.status.success() {
        return Err(anyhow!(
            "git status --porcelain=v1 -z failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(output.stdout)
}

pub fn ensure_commit_identity(repo: &Path) -> Result<()> {
    run(repo, &["var", "GIT_AUTHOR_IDENT"])
        .map(|_| ())
        .context("Git commit identity is not configured; set user.name and user.email")
}

pub fn changed_files(repo: &Path, base: &str, head: &str) -> Result<Vec<String>> {
    let range = format!("{base}..{head}");
    let out = run(repo, &["diff", "--name-only", &range])?.stdout;
    Ok(out
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(ToString::to_string)
        .collect())
}

pub fn object_size(repo: &Path, object: &str) -> Result<u64> {
    Ok(run(repo, &["cat-file", "-s", object])?.stdout.parse()?)
}

pub fn add_worktree(source: &Path, destination: &Path, branch: &str, base: &str) -> Result<()> {
    run_quiet(
        source,
        &[
            "worktree",
            "add",
            path_str(destination)?,
            "-b",
            branch,
            base,
        ],
    )
}

pub fn add_existing_worktree(source: &Path, destination: &Path, branch: &str) -> Result<()> {
    run_quiet(source, &["worktree", "add", path_str(destination)?, branch])
}

pub fn remove_worktree(source: &Path, destination: &Path, force: bool) -> Result<()> {
    let mut args = vec!["worktree", "remove"];
    if force {
        args.push("--force");
    }
    args.push(path_str(destination)?);
    run_quiet(source, &args)
}

pub fn delete_branch(source: &Path, branch: &str) -> Result<()> {
    run_quiet(source, &["branch", "-D", branch])
}

pub fn update_ref(repo: &Path, reference: &str, object: &str) -> Result<()> {
    run_quiet(repo, &["update-ref", reference, object])
}

pub fn ref_exists(repo: &Path, reference: &str) -> bool {
    succeeds(repo, &["show-ref", "--verify", "--quiet", reference]).unwrap_or(false)
}

pub fn path_str(path: &Path) -> Result<&str> {
    path.to_str()
        .ok_or_else(|| anyhow!("path is not valid UTF-8: {}", path.display()))
}
