//! Demo of distributed planner with multiple algorithms.

use distributed_planner::{
    Task, AssignmentStatus,
    algorithms::{
        PlanningAlgorithm,
        RoundRobinPlanner,
        AuctionPlanner,
        ResourceAwarePlanner,
        MultiObjectivePlanner,
        MultiObjectiveWeights,
        RLPlanner,
    },
};
use common::types::{AgentId, Capability};
use std::collections::{HashMap, HashSet};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    println!("=== Distributed Planner Demo ===\n");

    // Create sample tasks
    let tasks = vec![
        Task {
            id: "task-1".to_string(),
            description: "Map warehouse area A".to_string(),
            required_resources: vec!["battery".to_string()],
            required_capabilities: vec![Capability::Navigation, Capability::LiDAR],
            estimated_duration_secs: 300,
            deadline: None,
            priority: 150,
            dependencies: vec![],
        },
        Task {
            id: "task-2".to_string(),
            description: "Transport package to zone B".to_string(),
            required_resources: vec!["battery".to_string(), "cargo_capacity".to_string()],
            required_capabilities: vec![Capability::Navigation, Capability::Gripper],
            estimated_duration_secs: 180,
            deadline: Some(3600),
            priority: 200,
            dependencies: vec!["task-1".to_string()],
        },
        Task {
            id: "task-3".to_string(),
            description: "Inspect equipment".to_string(),
            required_resources: vec!["battery".to_string()],
            required_capabilities: vec![Capability::Camera, Capability::Navigation],
            estimated_duration_secs: 120,
            deadline: Some(1800),
            priority: 100,
            dependencies: vec![],
        },
        Task {
            id: "task-4".to_string(),
            description: "Emergency response".to_string(),
            required_resources: vec!["battery".to_string()],
            required_capabilities: vec![Capability::Navigation],
            estimated_duration_secs: 60,
            deadline: Some(300),
            priority: 255,
            dependencies: vec![],
        },
    ];

    // Create sample agents
    let agents: HashSet<AgentId> = vec![
        "agent-1",
        "agent-2",
        "agent-3",
    ].into_iter().map(|s| s.to_string()).collect();

    // Define agent resources and capabilities
    let mut agent_resources: HashMap<AgentId, HashMap<String, f64>> = HashMap::new();
    agent_resources.insert("agent-1".to_string(), {
        let mut res = HashMap::new();
        res.insert("battery".to_string(), 100.0);
        res.insert("cargo_capacity".to_string(), 50.0);
        res
    });
    agent_resources.insert("agent-2".to_string(), {
        let mut res = HashMap::new();
        res.insert("battery".to_string(), 80.0);
        res.insert("cargo_capacity".to_string(), 30.0);
        res
    });
    agent_resources.insert("agent-3".to_string(), {
        let mut res = HashMap::new();
        res.insert("battery".to_string(), 90.0);
        res.insert("cargo_capacity".to_string(), 40.0);
        res
    });

    let mut agent_capabilities: HashMap<AgentId, HashSet<Capability>> = HashMap::new();
    agent_capabilities.insert("agent-1".to_string(), {
        let mut caps = HashSet::new();
        caps.insert(Capability::Navigation);
        caps.insert(Capability::LiDAR);
        caps.insert(Capability::Camera);
        caps
    });
    agent_capabilities.insert("agent-2".to_string(), {
        let mut caps = HashSet::new();
        caps.insert(Capability::Navigation);
        caps.insert(Capability::Gripper);
        caps.insert(Capability::Camera);
        caps
    });
    agent_capabilities.insert("agent-3".to_string(), {
        let mut caps = HashSet::new();
        caps.insert(Capability::Navigation);
        caps.insert(Capability::LiDAR);
        caps.insert(Capability::Gripper);
        caps
    });

    println!("Tasks:");
    for task in &tasks {
        println!(
            "  {} - {} (priority: {}, deadline: {:?})",
            task.id, task.description, task.priority, task.deadline
        );
    }
    println!("\nAgents: {:?}", agents);

    // Test Round Robin
    println!("\n--- Round Robin Planning ---");
    let rr_planner = RoundRobinPlanner;
    let assignments = rr_planner.plan(tasks.clone(), agents.clone(), vec![]).await?;
    print_assignments(&assignments);

    // Test Auction-based
    println!("\n--- Auction-Based Planning ---");
    let auction_planner = AuctionPlanner::new(|task: &Task, agent: AgentId| {
        // Simple cost function based on task priority and agent ID
        let base_cost = 100 - task.priority as u64;
        let agent_factor = match agent.as_str() {
            "agent-1" => 1,
            "agent-2" => 2,
            "agent-3" => 3,
            _ => 10,
        };
        base_cost + agent_factor * 10
    });
    let assignments = auction_planner.plan(tasks.clone(), agents.clone(), vec![]).await?;
    print_assignments(&assignments);

    // Test Resource-Aware
    println!("\n--- Resource-Aware Planning ---");
    let resource_planner = ResourceAwarePlanner::new(
        agent_resources.clone(),
        agent_capabilities.clone(),
    );
    let assignments = resource_planner.plan(tasks.clone(), agents.clone(), vec![]).await?;
    print_assignments(&assignments);

    // Test Multi-Objective
    println!("\n--- Multi-Objective Planning ---");
    let weights = MultiObjectiveWeights {
        priority_weight: 0.35,
        deadline_weight: 0.30,
        efficiency_weight: 0.15,
        load_balance_weight: 0.10,
        capability_match_weight: 0.10,
    };
    let multi_obj_planner = MultiObjectivePlanner::new(
        weights,
        agent_resources.clone(),
        agent_capabilities.clone(),
    );
    let assignments = multi_obj_planner.plan(tasks.clone(), agents, vec![]).await?;
    print_assignments(&assignments);

    // Test RL-based
    println!("\n--- RL-Based Planning ---");
    let mut rl_planner = RLPlanner::new(0.1, 0.9, 0.2);
    
    // Run planning multiple times to simulate learning
    for round in 1..=3 {
        println!("\n  Round {}", round);
        let assignments = rl_planner.plan(tasks.clone(), {
            let mut set = HashSet::new();
            set.insert("agent-1".to_string());
            set.insert("agent-2".to_string());
            set.insert("agent-3".to_string());
            set
        }, vec![]).await?;
        
        for assignment in &assignments {
            // Simulate random completion success
            let success = rand::random_bool(0.8);
            rl_planner.report_completion(&assignment.task_id, assignment.agent_id, success);
            println!(
                "    {} -> {} (success: {})",
                assignment.task_id, assignment.agent_id, success
            );
        }
    }

    println!("\n=== Demo Complete ===");
    Ok(())
}

fn print_assignments(assignments: &[distributed_planner::Assignment]) {
    for assignment in assignments {
        println!(
            "  {} -> {} [status: {:?}, priority: {}]",
            assignment.task_id,
            assignment.agent_id,
            assignment.status,
            assignment.priority
        );
    }
}
