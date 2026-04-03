//! Distributed workflow orchestration for offline‑first multi‑agent systems.
//!
//! Provides definition, scheduling, execution, and monitoring of workflows
//! across a network of agents.

pub mod error;
pub mod model;
pub mod scheduler;
pub mod executor;
pub mod coordinator;
pub mod monitor;

pub use error::WorkflowError;
pub use model::{Workflow, Task, TaskStatus, WorkflowStatus};
pub use scheduler::WorkflowScheduler;
pub use executor::TaskExecutor;
pub use coordinator::DistributedCoordinator;
pub use monitor::WorkflowMonitor;

/// Re‑export of common types.
pub mod prelude {
    pub use super::{
        WorkflowError,
        Workflow,
        Task,
        TaskStatus,
        WorkflowStatus,
        WorkflowScheduler,
        TaskExecutor,
        DistributedCoordinator,
        WorkflowMonitor,
    };
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}