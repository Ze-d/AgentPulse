//! AgentPulse hook adapter — zero-dependency binary.
//!
//! Default mode (no subcommand): reads hook JSON from stdin, enriches with
//! agent info, and POSTs to the AgentPulse event server.
//!
//! Subcommand mode: manages hook configuration in Claude Code / Codex settings.

mod agent;
mod hook;
mod installer;
mod sender;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "agentpulse-hook",
    version,
    about = "AgentPulse hook adapter",
    long_about = "Zero-dependency event forwarder and hook manager for AgentPulse."
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Print enriched JSON to stdout instead of sending to server
    #[arg(long, global = true)]
    test: bool,

    /// Target agent: claude (default) or codex
    #[arg(long, default_value = "claude", global = true)]
    agent: String,

    /// Override config/settings file path
    #[arg(long, global = true)]
    path: Option<String>,

    /// Force overwrite existing hooks
    #[arg(long, global = true)]
    force: bool,

    /// Override the AgentPulse server URL (runtime mode only)
    #[arg(long)]
    url: Option<String>,
}

#[derive(Subcommand)]
enum Commands {
    /// Install hooks to agent config file
    Install,
    /// Remove hooks from agent config file
    Remove,
    /// Show hook installation status
    Status,
    /// Preview changes without modifying
    DryRun,
}

fn main() {
    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or(
            std::env::var("AGENTPULSE_LOG_LEVEL").unwrap_or_else(|_| "info".to_string()),
        ),
    )
    .format_timestamp_millis()
    .init();

    let cli = Cli::parse();

    match cli.command {
        None => {
            // Default mode: stdin -> POST
            hook::run(cli.test, cli.url.as_deref());
        }
        Some(Commands::Install) => {
            installer::install(&cli.agent, cli.path.as_deref(), cli.force);
        }
        Some(Commands::Remove) => {
            installer::remove(&cli.agent, cli.path.as_deref());
        }
        Some(Commands::Status) => {
            installer::status(&cli.agent, cli.path.as_deref());
        }
        Some(Commands::DryRun) => {
            installer::dry_run(&cli.agent, cli.path.as_deref());
        }
    }
}
