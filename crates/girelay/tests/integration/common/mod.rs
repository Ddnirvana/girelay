use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use tempfile::TempDir;

pub fn girelay() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_girelay"))
}

pub fn run(dir: &Path, args: &[&str]) -> Output {
    Command::new(girelay())
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap_or_else(|err| panic!("failed to run girelay {args:?}: {err}"))
}

pub fn run_ok(dir: &Path, args: &[&str]) -> String {
    let output = run(dir, args);
    if !output.status.success() {
        panic!(
            "girelay {args:?} failed\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    String::from_utf8_lossy(&output.stdout).to_string()
}

pub fn run_fail(dir: &Path, args: &[&str]) -> String {
    let output = run(dir, args);
    assert!(
        !output.status.success(),
        "girelay {args:?} unexpectedly succeeded\nstdout:\n{}",
        String::from_utf8_lossy(&output.stdout)
    );
    String::from_utf8_lossy(&output.stderr).to_string()
}

pub fn git(dir: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap_or_else(|err| panic!("failed to run git {args:?}: {err}"));
    if !output.status.success() {
        panic!(
            "git {args:?} failed\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    String::from_utf8_lossy(&output.stdout)
        .trim_end()
        .to_string()
}

pub fn write(path: &Path, body: &str) {
    fs::write(path, body).unwrap_or_else(|err| panic!("failed to write {}: {err}", path.display()));
}

pub struct Repo {
    _tmp: TempDir,
    pub root: PathBuf,
}

impl Repo {
    pub fn new() -> Self {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().join("repo");
        fs::create_dir(&root).unwrap();
        git(&root, &["init", "-b", "main"]);
        git(&root, &["config", "user.email", "agent@example.com"]);
        git(&root, &["config", "user.name", "Agent Test"]);
        write(&root.join("README.md"), "# demo\n");
        git(&root, &["add", "README.md"]);
        git(&root, &["commit", "-m", "initial commit"]);
        Self { _tmp: tmp, root }
    }

    pub fn start(&self, task: &str) -> PathBuf {
        run_ok(
            &self.root,
            &["start", task, "--intent", "Add greeting function"],
        );
        self.workspace(task)
    }

    #[allow(dead_code)]
    pub fn workspace(&self, task: &str) -> PathBuf {
        self.root.join(".girelay/workspaces").join(task)
    }
}
