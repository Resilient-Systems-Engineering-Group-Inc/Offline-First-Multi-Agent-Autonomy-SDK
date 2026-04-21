//! WebAssembly bindings for the Multi-Agent SDK.
//!
//! This crate provides WebAssembly bindings that allow the SDK to run
//! in the browser or other WASM runtimes.

use wasm_bindgen::prelude::*;
use serde::{Serialize, Deserialize};

// Enable panic hook for better error messages
#[cfg(target_arch = "wasm32")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

#[macro_export]
macro_rules! console_log {
    ($($t:tt)*) => (log(&format_args!($($t)*).to_string()))
}

/// Task representation for WASM.
#[derive(Serialize, Deserialize, Clone)]
#[wasm_bindgen]
pub struct WasmTask {
    id: String,
    description: String,
    status: String,
    priority: i32,
}

#[wasm_bindgen]
impl WasmTask {
    #[wasm_bindgen(constructor)]
    pub fn new(id: &str, description: &str, priority: i32) -> Self {
        Self {
            id: id.to_string(),
            description: description.to_string(),
            status: "pending".to_string(),
            priority,
        }
    }

    #[wasm_bindgen(getter)]
    pub fn id(&self) -> String {
        self.id.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn description(&self) -> String {
        self.description.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn status(&self) -> String {
        self.status.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn priority(&self) -> i32 {
        self.priority
    }

    #[wasm_bindgen]
    pub fn set_status(&mut self, status: &str) {
        self.status = status.to_string();
    }
}

/// Task planner for WASM.
#[wasm_bindgen]
pub struct WasmTaskPlanner {
    tasks: Vec<WasmTask>,
}

#[wasm_bindgen]
impl WasmTaskPlanner {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            tasks: Vec::new(),
        }
    }

    /// Add a new task.
    #[wasm_bindgen]
    pub fn add_task(&mut self, id: &str, description: &str, priority: i32) {
        let task = WasmTask::new(id, description, priority);
        self.tasks.push(task);
        console_log!("Task added: {}", id);
    }

    /// Get all tasks.
    #[wasm_bindgen]
    pub fn get_tasks(&self) -> JsValue {
        serde_wasm_bindgen::to_value(&self.tasks).unwrap()
    }

    /// Plan tasks using default algorithm.
    #[wasm_bindgen]
    pub async fn plan_tasks(&mut self) -> JsValue {
        // Sort by priority
        self.tasks.sort_by(|a, b| b.priority.cmp(&a.priority));
        
        // Mark all as planned
        for task in &mut self.tasks {
            task.set_status("planned");
        }

        serde_wasm_bindgen::to_value(&self.tasks).unwrap()
    }

    /// Get task count.
    #[wasm_bindgen]
    pub fn task_count(&self) -> usize {
        self.tasks.len()
    }

    /// Clear all tasks.
    #[wasm_bindgen]
    pub fn clear(&mut self) {
        self.tasks.clear();
    }
}

/// Network simulator for browser.
#[wasm_bindgen]
pub struct WasmNetworkSimulator {
    latency_ms: u32,
    bandwidth_mbps: f64,
    packet_loss_rate: f64,
}

#[wasm_bindgen]
impl WasmNetworkSimulator {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            latency_ms: 50,
            bandwidth_mbps: 100.0,
            packet_loss_rate: 0.01,
        }
    }

    /// Set network conditions.
    #[wasm_bindgen]
    pub fn set_conditions(&mut self, latency_ms: u32, bandwidth_mbps: f64, packet_loss_rate: f64) {
        self.latency_ms = latency_ms;
        self.bandwidth_mbps = bandwidth_mbps;
        self.packet_loss_rate = packet_loss_rate;
    }

    /// Simulate network delay.
    #[wasm_bindgen]
    pub async fn simulate_delay(&self) {
        use wasm_bindgen_futures::JsFuture;
        let promise = js_sys::Promise::new(&mut |resolve, _| {
            web_sys::window()
                .unwrap()
                .set_timeout_with_callback_and_timeout_and_arguments_0(
                    &resolve,
                    self.latency_ms as i32
                )
                .unwrap();
        });
        JsFuture::from(promise).await.unwrap();
    }

    /// Check if packet is lost.
    #[wasm_bindgen]
    pub fn simulate_packet_loss(&self) -> bool {
        let mut rng = rand::thread_rng();
        rng.gen::<f64>() < self.packet_loss_rate
    }

    /// Get current latency.
    #[wasm_bindgen]
    pub fn latency_ms(&self) -> u32 {
        self.latency_ms
    }

    /// Get current bandwidth.
    #[wasm_bindgen]
    pub fn bandwidth_mbps(&self) -> f64 {
        self.bandwidth_mbps
    }
}

/// Initialize WASM module.
#[wasm_bindgen(start)]
pub fn init() {
    console_error_panic_hook::set_once();
    console_log!("SDK WASM module initialized");
}

/// Get version information.
#[wasm_bindgen]
pub fn get_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_planner() {
        let mut planner = WasmTaskPlanner::new();
        
        planner.add_task("task-1", "Test task", 100);
        planner.add_task("task-2", "Another task", 150);
        
        assert_eq!(planner.task_count(), 2);
    }
}
