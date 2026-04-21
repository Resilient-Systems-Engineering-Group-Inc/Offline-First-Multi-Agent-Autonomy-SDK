//! Command-line interface for the Multi-Agent SDK.

mod commands;
mod config;

use anyhow::Result;
use clap::{Parser, Subcommand};
use tracing::info;

#[derive(Parser)]
#[command(name = "sdk")]
#[command(author = "Resilient Systems Engineering")]
#[command(version = "0.1.0")]
#[command(about = "Multi-Agent Autonomy SDK CLI", long_about = None)]
struct Cli {
    /// API endpoint
    #[arg(short, long, default_value = "http://localhost:3000")]
    api: String,

    /// Output format
    #[arg(short, long, default_value = "table")]
    format: String,

    /// Verbose output
    #[arg(short, long)]
    verbose: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Task management
    #[command(subcommand)]
    Task(TaskCommands),

    /// Agent management
    #[command(subcommand)]
    Agent(AgentCommands),

    /// Workflow management
    #[command(subcommand)]
    Workflow(WorkflowCommands),

    /// System operations
    #[command(subcommand)]
    System(SystemCommands),

    /// Configuration
    #[command(subcommand)]
    Config(ConfigCommands),
}

#[derive(Subcommand)]
enum TaskCommands {
    /// List tasks
    List {
        /// Filter by status
        #[arg(short, long)]
        status: Option<String>,

        /// Limit results
        #[arg(short, long, default_value = "10")]
        limit: i32,
    },

    /// Create a new task
    Create {
        /// Task description
        #[arg(short, long)]
        description: String,

        /// Task priority (1-1000)
        #[arg(short, long, default_value = "100")]
        priority: i32,

        /// Required capabilities
        #[arg(short, long)]
        capabilities: Option<Vec<String>>,
    },

    /// Get task details
    Get {
        /// Task ID
        #[arg(short, long)]
        id: String,
    },

    /// Update task
    Update {
        /// Task ID
        #[arg(short, long)]
        id: String,

        /// New status
        #[arg(short, long)]
        status: Option<String>,

        /// Assign to agent
        #[arg(short, long)]
        agent: Option<String>,
    },

    /// Delete task
    Delete {
        /// Task ID
        #[arg(short, long)]
        id: String,
    },
}

#[derive(Subcommand)]
enum AgentCommands {
    /// List agents
    List {
        /// Filter by status
        #[arg(short, long)]
        status: Option<String>,
    },

    /// Get agent details
    Get {
        /// Agent ID
        #[arg(short, long)]
        id: String,
    },

    /// Register new agent
    Register {
        /// Agent name
        #[arg(short, long)]
        name: String,

        /// Agent capabilities
        #[arg(short, long)]
        capabilities: Vec<String>,
    },

    /// Unregister agent
    Unregister {
        /// Agent ID
        #[arg(short, long)]
        id: String,
    },
}

#[derive(Subcommand)]
enum WorkflowCommands {
    /// List workflows
    List,

    /// Create workflow
    Create {
        /// Workflow name
        #[arg(short, long)]
        name: String,

        /// Workflow YAML file
        #[arg(short, long)]
        file: Option<String>,
    },

    /// Start workflow
    Start {
        /// Workflow ID
        #[arg(short, long)]
        id: String,
    },

    /// Get workflow status
    Status {
        /// Workflow instance ID
        #[arg(short, long)]
        id: String,
    },
}

#[derive(Subcommand)]
enum SystemCommands {
    /// Health check
    Health,

    /// Get system metrics
    Metrics,

    /// Get system statistics
    Stats,

    /// Start interactive mode
    Interactive,
}

#[derive(Subcommand)]
enum ConfigCommands {
    /// Show current configuration
    Show,

    /// Set configuration value
    Set {
        /// Configuration key
        key: String,

        /// Configuration value
        value: String,
    },

    /// Reset configuration
    Reset,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    let env = if std::env::var("RUST_LOG").is_ok() {
        std::env::var("RUST_LOG").unwrap()
    } else {
        "info".to_string()
    };

    tracing_subscriber::fmt::init()
        .with_max_level(if env.contains("debug") {
            tracing::Level::DEBUG
        } else {
            tracing::Level::INFO
        });

    let cli = Cli::parse();

    info!("SDK CLI v{}", env!("CARGO_PKG_VERSION"));
    info!("API endpoint: {}", cli.api);

    // Execute command
    match cli.command {
        Commands::Task(task_cmd) => commands::task::execute(task_cmd, &cli.api).await,
        Commands::Agent(agent_cmd) => commands::agent::execute(agent_cmd, &cli.api).await,
        Commands::Workflow(workflow_cmd) => commands::workflow::execute(workflow_cmd, &cli.api).await,
        Commands::System(system_cmd) => commands::system::execute(system_cmd, &cli.api).await,
        Commands::Config(config_cmd) => commands::config::execute(config_cmd).await,
    }
}
