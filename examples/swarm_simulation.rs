//! Swarm simulation with real‑time visualization.

use offline_first_autonomy::agent_core::Agent;
use offline_first_autonomy::mesh_transport::{MeshTransport, MeshTransportConfig};
use offline_first_autonomy::state_sync::CrdtMap;
use common::types::AgentId;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tokio::time;
use crossterm::{
    cursor, execute,
    style::{Color, Print, ResetColor, SetForegroundColor},
    terminal::{Clear, ClearType},
};
use std::io::{stdout, Write};

/// A simple 2D position for visualization.
#[derive(Clone, Debug)]
struct Position {
    x: f32,
    y: f32,
}

/// Simulated agent with position and state.
struct SimAgent {
    id: AgentId,
    agent: Agent,
    position: Position,
    velocity: (f32, f32),
    color: Color,
}

impl SimAgent {
    fn new(id: u64, config: MeshTransportConfig) -> Self {
        let agent = Agent::new(AgentId(id), config).expect("Failed to create agent");
        Self {
            id: AgentId(id),
            agent,
            position: Position { x: 0.0, y: 0.0 },
            velocity: (0.0, 0.0),
            color: match id % 5 {
                0 => Color::Red,
                1 => Color::Green,
                2 => Color::Yellow,
                3 => Color::Blue,
                4 => Color::Magenta,
                _ => Color::Cyan,
            },
        }
    }

    async fn update(&mut self, delta_time: f32) {
        // Simple random walk
        use rand::Rng;
        let mut rng = rand::thread_rng();
        self.velocity.0 += rng.gen_range(-1.0..1.0);
        self.velocity.1 += rng.gen_range(-1.0..1.0);
        // Damping
        self.velocity.0 *= 0.9;
        self.velocity.1 *= 0.9;
        // Update position
        self.position.x += self.velocity.0 * delta_time;
        self.position.y += self.velocity.1 * delta_time;
        // Keep within bounds
        self.position.x = self.position.x.clamp(-10.0, 10.0);
        self.position.y = self.position.y.clamp(-10.0, 10.0);

        // Update CRDT with position
        let key = format!("agent/{}/position", self.id.0);
        let value = serde_json::json!({
            "x": self.position.x,
            "y": self.position.y,
            "timestamp": std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        });
        self.agent.set_value(&key, value).expect("Set failed");
    }

    fn draw(&self, grid: &mut Vec<Vec<char>>, offset_x: usize, offset_y: usize) {
        let grid_x = ((self.position.x + 10.0) * 2.0).round() as usize;
        let grid_y = ((self.position.y + 10.0) * 2.0).round() as usize;
        if grid_x < 40 && grid_y < 40 {
            grid[grid_y + offset_y][grid_x + offset_x] = '@';
        }
    }
}

/// Render the swarm in a terminal grid.
fn render_grid(agents: &[SimAgent]) {
    let width = 80;
    let height = 24;
    let mut grid = vec![vec![' '; width]; height];

    // Draw axes
    let center_x = width / 2;
    let center_y = height / 2;
    for y in 0..height {
        grid[y][center_x] = '|';
    }
    for x in 0..width {
        grid[center_y][x] = '-';
    }
    grid[center_y][center_x] = '+';

    // Draw each agent
    for agent in agents {
        agent.draw(&mut grid, center_x, center_y);
    }

    // Output to terminal
    let mut stdout = stdout();
    execute!(stdout, Clear(ClearType::All), cursor::MoveTo(0, 0)).unwrap();
    for row in grid {
        let line: String = row.iter().collect();
        execute!(stdout, Print(line), Print("\r\n")).unwrap();
    }
    execute!(stdout, ResetColor).unwrap();
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Starting swarm simulation with 5 agents...");

    // Create agents with in‑memory transport (so they can communicate)
    let mut agents = Vec::new();
    for i in 0..5 {
        let config = MeshTransportConfig::in_memory();
        let mut sim_agent = SimAgent::new(i, config);
        sim_agent.agent.start().expect("Agent start failed");
        agents.push(sim_agent);
    }

    let start = Instant::now();
    let mut last_render = Instant::now();
    let render_interval = Duration::from_millis(100);

    loop {
        let elapsed = start.elapsed();
        let delta_time = 0.1; // fixed time step for simplicity

        // Update each agent
        for agent in &mut agents {
            agent.update(delta_time).await;
        }

        // Broadcast changes (simulate sync)
        for agent in &mut agents {
            let _ = agent.agent.broadcast_changes().await;
        }

        // Render if enough time has passed
        if last_render.elapsed() >= render_interval {
            render_grid(&agents);
            last_render = Instant::now();
        }

        // Break after some time
        if elapsed >= Duration::from_secs(30) {
            break;
        }

        time::sleep(Duration::from_millis(50)).await;
    }

    println!("Simulation finished.");
    Ok(())
}