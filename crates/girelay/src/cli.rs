use clap::{Args, Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(
    name = "girelay",
    version,
    about = "Relay coding-agent work safely through Git worktrees"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Install the girelay protocol skill for an agent.
    Setup(SetupArgs),
    /// Create a task worktree and optionally start its first agent session.
    Start(StartArgs),
    /// Continue an existing task with another agent session.
    Relay(RelayArgs),
    /// Merge a task into its source branch after checks and review.
    Merge(MergeArgs),
    /// Show factual task and session state.
    Status(StatusArgs),
    /// Remove a task worktree, retaining its branch by default.
    Clean(CleanArgs),
    /// Inspect recovery state, restore it, or repair a stale operation lock.
    Recover(RecoverArgs),
    /// Submit a schema-validated semantic report for the active session.
    #[command(hide = true)]
    Report(ReportArgs),
}

#[derive(Debug, Clone, ValueEnum)]
pub enum AgentTarget {
    Codex,
    Claude,
}

#[derive(Debug, Args)]
pub struct SetupArgs {
    pub agent: AgentTarget,
    /// Install under this repository's excluded .girelay directory.
    #[arg(long)]
    pub local: bool,
}

#[derive(Debug, Args)]
#[command(trailing_var_arg = true)]
pub struct StartArgs {
    pub task_id: String,
    /// Durable task intent; defaults verbatim to TASK_ID.
    #[arg(long)]
    pub intent: Option<String>,
    /// Source branch; defaults to the configured workspace base.
    #[arg(long)]
    pub base: Option<String>,
    /// Agent command to run after creating the worktree.
    #[arg(allow_hyphen_values = true)]
    pub command: Vec<String>,
}

#[derive(Debug, Args)]
#[command(trailing_var_arg = true)]
pub struct RelayArgs {
    pub task_id: String,
    #[arg(required = true, allow_hyphen_values = true)]
    pub command: Vec<String>,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum MergeStrategy {
    Squash,
    Preserve,
}

#[derive(Debug, Args)]
pub struct MergeArgs {
    pub task_id: String,
    #[arg(long, value_enum, default_value_t = MergeStrategy::Squash)]
    pub strategy: MergeStrategy,
    /// Commit message; defaults to explicit intent or `agent: complete <task>`.
    #[arg(long)]
    pub message: Option<String>,
    /// Skip configured checks explicitly.
    #[arg(long)]
    pub no_checks: bool,
    /// Preview the merge without changing files, refs, commits, or metadata.
    #[arg(long)]
    pub dry_run: bool,
    /// Emit the preview or real merge result as JSON.
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Args)]
pub struct StatusArgs {
    pub task_id: Option<String>,
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Args)]
pub struct CleanArgs {
    pub task_id: String,
    /// Also delete the task branch when its recorded merge is still valid.
    #[arg(long)]
    pub delete_branch: bool,
    /// Discard uncommitted files. Committed-history protection still applies.
    #[arg(long)]
    pub discard_uncommitted: bool,
    /// Delete even when committed state is not preserved by a durable ref.
    #[arg(long)]
    pub discard_unreachable: bool,
    /// Create a verified bundle before removing the worktree.
    #[arg(long)]
    pub archive: bool,
    #[arg(long)]
    pub dry_run: bool,
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Args)]
pub struct RecoverArgs {
    #[command(subcommand)]
    pub command: RecoverCommand,
}

#[derive(Debug, Subcommand)]
pub enum RecoverCommand {
    /// List available recovery points.
    List {
        task_id: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// Show one recovery point and its safety conditions.
    Show {
        recovery_id: String,
        #[arg(long)]
        json: bool,
    },
    /// Restore one recovery point after explicit confirmation.
    Restore {
        recovery_id: String,
        #[arg(long)]
        confirm: bool,
    },
    /// Inspect or recover a stale task-operation lock.
    Unlock {
        task_id: String,
        /// Recover only after both recorded processes are no longer running.
        #[arg(long)]
        confirm: bool,
        /// Emit schema-v2 lock inspection JSON.
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug, Args)]
pub struct ReportArgs {
    #[arg(long)]
    pub session: String,
    #[arg(long)]
    pub file: PathBuf,
}
