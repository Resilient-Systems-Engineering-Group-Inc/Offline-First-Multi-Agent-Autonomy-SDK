//! Integration tests for distributed planner.

use distributed_planner::{
    Task, AssignmentStatus, TaskState, TaskLifecycleManager,
    algorithms::{
        PlanningAlgorithm,
        RoundRobinPlanner,
        ResourceAwarePlanner,
        MultiObjectivePlanner,
        MultiObjectiveWeights,
    },
};
use common::types::{AgentId, Capability};
use std::collections::{HashMap, HashSet};

#[tokio::test]
async fn test_round_robin_planning() {
    let tasks = vec![
        create_task("task-1", 100, vec![]),
        create_task("task-2", 150, vec![]),
        create_task("task-3", 200, vec![]),
    ];

    let agents: HashSet<AgentId> = vec!["agent-1", "agent-2", "agent-3"]
        .into_iter()
        .map(|s| s.to_string())
        .collect();

    let planner = RoundRobinPlanner;
    let assignments = planner.plan(tasks, agents, vec![]).await.unwrap();

    assert_eq!(assignments.len(), 3);
    
    // Each agent should get exactly one task
    let mut agent_counts = HashMap::new();
    for assignment in &assignments {
        *agent_counts.entry(assignment.agent_id.clone()).or_insert(0) += 1;
    }

    for count in agent_counts.values() {
        assert_eq!(*count, 1);
    }
}

#[tokio::test]
async fn test_resource_aware_planning() {
    let tasks = vec![
        Task {
            id: "task-1".to_string(),
            description: "Task requiring camera".to_string(),
            required_resources: vec!["battery".to_string()],
            required_capabilities: vec![Capability::Camera],
            estimated_duration_secs: 60,
            deadline: None,
            priority: 100,
            dependencies: vec![],
        },
    ];

    let mut agent_capabilities: HashMap<AgentId, HashSet<Capability>> = HashMap::new();
    agent_capabilities.insert("agent-1".to_string(), {
        let mut caps = HashSet::new();
        caps.insert(Capability::Camera);
        caps.insert(Capability::Navigation);
        caps
    });
    agent_capabilities.insert("agent-2".to_string(), {
        let mut caps = HashSet::new();
        caps.insert(Capability::Navigation);
        caps // No camera
    });

    let agents: HashSet<AgentId> = vec!["agent-1", "agent-2"]
        .into_iter()
        .map(|s| s.to_string())
        .collect();

    let planner = ResourceAwarePlanner::new(HashMap::new(), agent_capabilities);
    let assignments = planner.plan(tasks, agents, vec![]).await.unwrap();

    // Only agent-1 has camera capability
    assert_eq!(assignments.len(), 1);
    assert_eq!(assignments[0].agent_id, "agent-1");
}

#[tokio::test]
async fn test_multi_objective_planning() {
    let tasks = vec![
        create_task_with_deadline("urgent-task", 255, 100), // High priority, urgent
        create_task_with_deadline("normal-task", 100, 3600), // Normal priority
    ];

    let agents: HashSet<AgentId> = vec!["agent-1", "agent-2"]
        .into_iter()
        .map(|s| s.to_string())
        .collect();

    let weights = MultiObjectiveWeights {
        priority_weight: 0.5,
        deadline_weight: 0.3,
        efficiency_weight: 0.1,
        load_balance_weight: 0.05,
        capability_match_weight: 0.05,
    };

    let planner = MultiObjectivePlanner::new(
        weights,
        HashMap::new(),
        HashMap::new(),
    );

    let assignments = planner.plan(tasks, agents, vec![]).await.unwrap();
    assert_eq!(assignments.len(), 2);
}

#[tokio::test]
async fn test_task_lifecycle() {
    let manager = TaskLifecycleManager::new(3);
    let task_id = "test-task";
    let agent_id = "test-agent";

    // Register task
    manager.register_task(task_id).await;
    assert_eq!(manager.get_task_state(task_id).await, Some(TaskState::Pending));

    // Assign task
    manager.assign_task(task_id, agent_id).await.unwrap();
    assert_eq!(manager.get_task_state(task_id).await, Some(TaskState::Assigned));

    // Start task
    manager.start_task(task_id, agent_id).await.unwrap();
    assert_eq!(manager.get_task_state(task_id).await, Some(TaskState::InProgress));

    // Complete task
    manager.complete_task(task_id, agent_id, 30).await.unwrap();
    assert_eq!(manager.get_task_state(task_id).await, Some(TaskState::Completed));
}

#[tokio::test]
async fn test_task_retry_logic() {
    let manager = TaskLifecycleManager::new(2);
    let task_id = "retry-task";
    let agent_id = "test-agent";

    manager.register_task(task_id).await;
    manager.assign_task(task_id, agent_id).await.unwrap();
    manager.start_task(task_id, agent_id).await.unwrap();

    // First failure
    let decision = manager.fail_task(task_id, agent_id, "error 1".to_string()).await.unwrap();
    assert!(matches!(distributed_planner::RetryDecision::Retry(1), decision));

    // Second failure
    let decision = manager.fail_task(task_id, agent_id, "error 2".to_string()).await.unwrap();
    assert!(matches!(distributed_planner::RetryDecision::Retry(2), decision));

    // Third failure - max retries exceeded
    let decision = manager.fail_task(task_id, agent_id, "error 3".to_string()).await.unwrap();
    assert!(matches!(
        distributed_planner::RetryDecision::MaxRetriesExceeded,
        decision
    ));

    // Task should be in Failed state
    assert_eq!(manager.get_task_state(task_id).await, Some(TaskState::Failed));
}

#[tokio::test]
async fn test_dependency_aware_planning() {
    use distributed_planner::algorithms::DependencyAwarePlanner;

    let tasks = vec![
        Task {
            id: "task-1".to_string(),
            description: "First task".to_string(),
            required_resources: vec![],
            required_capabilities: vec![],
            estimated_duration_secs: 30,
            deadline: None,
            priority: 100,
            dependencies: vec![], // No dependencies
        },
        Task {
            id: "task-2".to_string(),
            description: "Second task".to_string(),
            required_resources: vec![],
            required_capabilities: vec![],
            estimated_duration_secs: 30,
            deadline: None,
            priority: 100,
            dependencies: vec!["task-1".to_string()], // Depends on task-1
        },
    ];

    let agents: HashSet<AgentId> = vec!["agent-1"].into_iter().map(|s| s.to_string()).collect();

    // No completed tasks yet
    let planner = DependencyAwarePlanner;
    let assignments = planner.plan(tasks.clone(), agents.clone(), vec![]).await.unwrap();

    // Only task-1 should be assigned (task-2 has unmet dependency)
    assert_eq!(assignments.len(), 1);
    assert_eq!(assignments[0].task_id, "task-1");

    // Simulate task-1 completion
    let completed_assignments = vec![
        distributed_planner::Assignment {
            task_id: "task-1".to_string(),
            agent_id: "agent-1".to_string(),
            start_time: None,
            status: AssignmentStatus::Completed,
            deadline: None,
            priority: 100,
            dependencies_satisfied: true,
            estimated_finish_time: None,
        }
    ];

    let assignments = planner.plan(tasks, agents, completed_assignments).await.unwrap();
    
    // Now task-2 should be assignable
    assert!(assignments.len() >= 1);
}

// Helper functions
fn create_task(id: &str, priority: u8, dependencies: Vec<&str>) -> Task {
    Task {
        id: id.to_string(),
        description: format!("Task {}", id),
        required_resources: vec![],
        required_capabilities: vec![],
        estimated_duration_secs: 60,
        deadline: None,
        priority,
        dependencies: dependencies.iter().map(|s| s.to_string()).collect(),
    }
}

fn create_task_with_deadline(id: &str, priority: u8, deadline_offset_secs: u64) -> Task {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    
    Task {
        id: id.to_string(),
        description: format!("Task {}", id),
        required_resources: vec![],
        required_capabilities: vec![],
        estimated_duration_secs: 60,
        deadline: Some(now + deadline_offset_secs),
        priority,
        dependencies: vec![],
    }
}
