use criterion::{black_box, criterion_group, criterion_main, Criterion};
use distributed_planner::{
    Task, AssignmentStatus, RoundRobinPlanner, DeadlineAwarePlanner, DependencyAwarePlanner,
    PlanningAlgorithm,
};
use common::types::{AgentId, Capability};
use std::collections::HashSet;

fn create_sample_tasks(count: usize) -> Vec<Task> {
    (0..count)
        .map(|i| Task {
            id: format!("task_{}", i),
            description: format!("Sample task {}", i),
            required_resources: vec!["cpu".to_string()],
            required_capabilities: vec!["computation".to_string()],
            estimated_duration_secs: 10,
            deadline: if i % 2 == 0 { Some(1000 + i as u64) } else { None },
            priority: (i % 5) as u8,
            dependencies: if i > 0 {
                vec![format!("task_{}", i - 1)]
            } else {
                Vec::new()
            },
        })
        .collect()
}

fn bench_round_robin(c: &mut Criterion) {
    let tasks = create_sample_tasks(100);
    let agents: HashSet<AgentId> = (0..10).map(AgentId).collect();
    let planner = RoundRobinPlanner;

    c.bench_function("round_robin_plan_100_tasks_10_agents", |b| {
        b.iter(|| {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let assignments = planner.plan(
                    black_box(tasks.clone()),
                    black_box(agents.clone()),
                    black_box(Vec::new()),
                ).await.unwrap();
                black_box(assignments);
            });
        })
    });
}

fn bench_deadline_aware(c: &mut Criterion) {
    let tasks = create_sample_tasks(100);
    let agents: HashSet<AgentId> = (0..10).map(AgentId).collect();
    let planner = DeadlineAwarePlanner;

    c.bench_function("deadline_aware_plan_100_tasks_10_agents", |b| {
        b.iter(|| {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let assignments = planner.plan(
                    black_box(tasks.clone()),
                    black_box(agents.clone()),
                    black_box(Vec::new()),
                ).await.unwrap();
                black_box(assignments);
            });
        })
    });
}

fn bench_dependency_aware(c: &mut Criterion) {
    let tasks = create_sample_tasks(100);
    let agents: HashSet<AgentId> = (0..10).map(AgentId).collect();
    let planner = DependencyAwarePlanner;

    c.bench_function("dependency_aware_plan_100_tasks_10_agents", |b| {
        b.iter(|| {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let assignments = planner.plan(
                    black_box(tasks.clone()),
                    black_box(agents.clone()),
                    black_box(Vec::new()),
                ).await.unwrap();
                black_box(assignments);
            });
        })
    });
}

criterion_group!(
    benches,
    bench_round_robin,
    bench_deadline_aware,
    bench_dependency_aware,
);
criterion_main!(benches);