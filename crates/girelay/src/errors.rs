use anyhow::Error;
use std::fmt;

#[derive(Debug)]
pub struct ChildExit {
    pub code: i32,
    pub message: String,
}

impl fmt::Display for ChildExit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for ChildExit {}

pub fn exit_code(error: &Error) -> i32 {
    if let Some(exit) = error.downcast_ref::<ChildExit>() {
        return exit.code;
    }
    let text = format!("{error:#}");
    if contains_any(
        &text,
        &[
            "dirty",
            "stale",
            "invalid task id",
            "cleanup refused",
            "not reachable",
            "active or stale operation",
        ],
    ) {
        2
    } else if contains_any(&text, &["not found", "missing", "unknown girelay task"]) {
        3
    } else {
        1
    }
}

fn contains_any(text: &str, needles: &[&str]) -> bool {
    let lower = text.to_ascii_lowercase();
    needles
        .iter()
        .any(|needle| lower.contains(&needle.to_ascii_lowercase()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::anyhow;

    #[test]
    fn unsafe_refusals_use_exit_code_two() {
        assert_eq!(exit_code(&anyhow!("working tree is dirty")), 2);
        assert_eq!(exit_code(&anyhow!("source rollback is stale")), 2);
        assert_eq!(
            exit_code(&anyhow!("cleanup refused: HEAD not reachable")),
            2
        );
        assert_eq!(
            exit_code(&anyhow!("task has an active or stale operation")),
            2
        );
    }
}
