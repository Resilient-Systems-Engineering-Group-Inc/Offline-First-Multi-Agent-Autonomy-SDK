//! Command handlers for CQRS.

use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Command trait.
pub trait Command: Send + Sync {
    fn command_type(&self) -> &str;
    fn aggregate_id(&self) -> &str;
}

/// Command handler trait.
#[async_trait::async_trait]
pub trait CommandHandler<C: Command>: Send + Sync {
    async fn handle(&self, command: C) -> Result<CommandResult>;
}

/// Command result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandResult {
    pub success: bool,
    pub aggregate_id: String,
    pub events_generated: Vec<String>,
    pub error: Option<String>,
}

impl CommandResult {
    pub fn success(aggregate_id: &str, events: Vec<String>) -> Self {
        Self {
            success: true,
            aggregate_id: aggregate_id.to_string(),
            events_generated: events,
            error: None,
        }
    }

    pub fn failure(aggregate_id: &str, error: &str) -> Self {
        Self {
            success: false,
            aggregate_id: aggregate_id.to_string(),
            events_generated: vec![],
            error: Some(error.to_string()),
        }
    }
}

/// Create task command.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTaskCommand {
    pub task_id: String,
    pub description: String,
    pub priority: i32,
    pub assigned_agent: Option<String>,
}

impl Command for CreateTaskCommand {
    fn command_type(&self) -> &str {
        "CreateTask"
    }

    fn aggregate_id(&self) -> &str {
        &self.task_id
    }
}

/// Assign task command.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssignTaskCommand {
    pub task_id: String,
    pub agent_id: String,
}

impl Command for AssignTaskCommand {
    fn command_type(&self) -> &str {
        "AssignTask"
    }

    fn aggregate_id(&self) -> &str {
        &self.task_id
    }
}

/// Complete task command.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompleteTaskCommand {
    pub task_id: String,
    pub result: serde_json::Value,
}

impl Command for CompleteTaskCommand {
    fn command_type(&self) -> &str {
        "CompleteTask"
    }

    fn aggregate_id(&self) -> &str {
        &self.task_id
    }
}

/// Register agent command.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterAgentCommand {
    pub agent_id: String,
    pub name: String,
    pub capabilities: Vec<String>,
}

impl Command for RegisterAgentCommand {
    fn command_type(&self) -> &str {
        "RegisterAgent"
    }

    fn aggregate_id(&self) -> &str {
        &self.agent_id
    }
}

/// Command bus for dispatching commands.
pub struct CommandBus {
    handlers: std::collections::HashMap<String, Box<dyn CommandHandlerBase>>,
}

trait CommandHandlerBase: Send + Sync {
    fn handle(&self, command: serde_json::Value) -> Result<CommandResult>;
}

impl CommandBus {
    pub fn new() -> Self {
        Self {
            handlers: std::collections::HashMap::new(),
        }
    }

    pub fn register<H, C>(&mut self, command_type: &str, handler: H)
    where
        H: CommandHandler<C> + 'static,
        C: Command + for<'de> Deserialize<'de>,
    {
        struct Wrapper<H, C> {
            handler: H,
            _phantom: std::marker::PhantomData<C>,
        }

        impl<H, C> CommandHandlerBase for Wrapper<H, C>
        where
            H: CommandHandler<C>,
            C: Command + for<'de> Deserialize<'de>,
        {
            fn handle(&self, command: serde_json::Value) -> Result<CommandResult> {
                let command: C = serde_json::from_value(command)?;
                futures::executor::block_on(self.handler.handle(command))
            }
        }

        self.handlers.insert(
            command_type.to_string(),
            Box::new(Wrapper {
                handler,
                _phantom: std::marker::PhantomData,
            }),
        );
    }

    pub async fn dispatch(&self, command_type: &str, command: serde_json::Value) -> Result<CommandResult> {
        let handler = self.handlers.get(command_type)
            .ok_or_else(|| anyhow::anyhow!("No handler for command type: {}", command_type))?;

        handler.handle(command)
    }
}

impl Default for CommandBus {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_commands() {
        let create_task = CreateTaskCommand {
            task_id: "task-1".to_string(),
            description: "Test".to_string(),
            priority: 100,
            assigned_agent: None,
        };

        assert_eq!(create_task.command_type(), "CreateTask");
        assert_eq!(create_task.aggregate_id(), "task-1");
    }
}
