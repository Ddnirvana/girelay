mod analysis;
mod clean;
mod cli;
mod config;
mod errors;
mod git;
mod merge;
mod output;
mod recover;
mod report;
mod session;
mod setup;
mod status;
mod task;
mod workspace;
mod workspace_lock;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Commands};

fn main() {
    if let Err(error) = run() {
        eprintln!("error: {error:#}");
        std::process::exit(errors::exit_code(&error));
    }
}

fn run() -> Result<()> {
    match Cli::parse().command {
        Commands::Setup(args) => setup::setup(args),
        Commands::Start(args) => workspace::start(args),
        Commands::Relay(args) => session::relay(args),
        Commands::Merge(args) => merge::merge(args),
        Commands::Status(args) => status::status(args),
        Commands::Clean(args) => clean::clean(args),
        Commands::Recover(args) => recover::recover(args),
        Commands::Report(args) => report::report(args),
    }
}
