//! CLI commands.

pub mod task;
pub mod agent;
pub mod workflow;
pub mod system;
pub mod config;

use crate::Commands;
use anyhow::Result;

pub type CommandResult = Result<()>;

pub async fn execute(cmd: Commands, api_url: &str) -> CommandResult {
    match cmd {
        Commands::Task(task_cmd) => task::execute(task_cmd, api_url).await,
        Commands::Agent(agent_cmd) => agent::execute(agent_cmd, api_url).await,
        Commands::Workflow(workflow_cmd) => workflow::execute(workflow_cmd, api_url).await,
        Commands::System(system_cmd) => system::execute(system_cmd, api_url).await,
        Commands::Config(config_cmd) => config::execute(config_cmd).await,
    }
}
