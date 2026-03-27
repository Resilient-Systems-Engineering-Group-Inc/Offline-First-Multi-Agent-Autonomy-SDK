use pyo3::prelude::*;
use pyo3_asyncio::tokio::future_into_py;
use common::types::AgentId;
use mesh_transport::{MeshTransport, MeshTransportConfig};
use agent_core::Agent;
use distributed_planner::{DistributedPlanner, DistributedPlannerConfig, Task, Assignment, AssignmentStatus};
use bounded_consensus::{BoundedConsensusConfig, TwoPhaseBoundedConsensus};
use local_planner::{LocalPlanner, LocalPlannerConfig};
use resource_monitor::{ResourceMonitor, ResourceMonitorConfig};
use state_sync::crdt_map::CrdtMap;
use serde_json::{Value, json};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Python module for Offline‑First Multi‑Agent Autonomy SDK.
#[pymodule]
fn offline_first_autonomy(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<PyAgent>()?;
    m.add_class::<PyMeshTransport>()?;
    m.add_class::<PyDistributedPlanner>()?;
    m.add_class::<PyLocalPlanner>()?;
    m.add_class::<PyResourceMonitor>()?;
    m.add_class::<PyCrdtMap>()?;
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    Ok(())
}

/// Python wrapper for Agent.
#[pyclass]
struct PyAgent {
    inner: Agent,
}

#[pymethods]
impl PyAgent {
    /// Create a new agent.
    #[new]
    fn new(agent_id: u64) -> PyResult<Self> {
        let config = MeshTransportConfig {
            local_agent_id: AgentId(agent_id),
            ..Default::default()
        };
        let agent = Agent::new(AgentId(agent_id), config)
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
        Ok(Self { inner: agent })
    }

    /// Start the agent (asynchronous).
    fn start(&mut self) -> PyResult<()> {
        self.inner.start()
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
        Ok(())
    }

    /// Stop the agent (asynchronous).
    fn stop<'py>(&mut self, py: Python<'py>) -> PyResult<&'py PyAny> {
        let mut inner = std::mem::replace(&mut self.inner, unreachable!());
        future_into_py(py, async move {
            inner.stop().await
                .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
            Ok(())
        })
    }

    /// Broadcast changes (asynchronous).
    fn broadcast_changes<'py>(&mut self, py: Python<'py>) -> PyResult<&'py PyAny> {
        let mut inner = std::mem::replace(&mut self.inner, unreachable!());
        future_into_py(py, async move {
            inner.broadcast_changes().await
                .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
            Ok(())
        })
    }

    /// Set a key‑value pair in the agent's CRDT map.
    /// `value` must be a JSON‑serializable string.
    fn set_value(&mut self, key: &str, value: &str) -> PyResult<()> {
        let json_value: Value = serde_json::from_str(value)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
        self.inner.set_value(key, json_value)
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
        Ok(())
    }

    /// Get a value from the agent's CRDT map.
    /// Returns a JSON string, or None if key does not exist.
    fn get_value(&self, key: &str) -> PyResult<Option<String>> {
        let opt: Option<Value> = self.inner.get_value(key);
        Ok(opt.map(|v| v.to_string()))
    }

    /// Get the agent's ID.
    fn agent_id(&self) -> u64 {
        self.inner.id().0
    }
}

/// Python wrapper for MeshTransport (simplified).
#[pyclass]
struct PyMeshTransport {
    inner: Arc<RwLock<MeshTransport>>,
    local_agent_id: u64,
}

#[pymethods]
impl PyMeshTransport {
    #[new]
    fn new(local_agent_id: u64) -> PyResult<Self> {
        let config = MeshTransportConfig {
            local_agent_id: AgentId(local_agent_id),
            ..Default::default()
        };
        // Note: MeshTransport::new is async, but we cannot call it synchronously.
        // For now, we'll panic if called incorrectly. In a real scenario, you'd use
        // an async factory method. This is a placeholder.
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
        let transport = rt.block_on(async {
            MeshTransport::new(config).await
        }).map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
        Ok(Self {
            inner: Arc::new(RwLock::new(transport)),
            local_agent_id,
        })
    }

    fn start<'py>(&self, py: Python<'py>) -> PyResult<&'py PyAny> {
        let inner = self.inner.clone();
        future_into_py(py, async move {
            let mut guard = inner.write().await;
            guard.start().await
                .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
            Ok(())
        })
    }

    /// Get the local agent ID.
    fn local_agent_id(&self) -> u64 {
        self.local_agent_id
    }

    /// Get a list of connected peers.
    fn peers(&self) -> PyResult<Vec<u64>> {
        use tokio::runtime::Handle;
        let handle = Handle::try_current()
            .map_err(|_| pyo3::exceptions::PyRuntimeError::new_err("No tokio runtime"))?;
        let inner = self.inner.clone();
        let peers = handle.block_on(async {
            let guard = inner.read().await;
            guard.peers()
        });
        Ok(peers.iter().map(|p| p.agent_id.0).collect())
    }

    /// Send a message to a specific peer (asynchronous).
    fn send_to<'py>(&self, py: Python<'py>, peer_id: u64, payload: Vec<u8>) -> PyResult<&'py PyAny> {
        let inner = self.inner.clone();
        future_into_py(py, async move {
            let mut guard = inner.write().await;
            guard.send_to(AgentId(peer_id), payload).await
                .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
            Ok(())
        })
    }

    /// Broadcast a message to all connected peers (asynchronous).
    fn broadcast<'py>(&self, py: Python<'py>, payload: Vec<u8>) -> PyResult<&'py PyAny> {
        let inner = self.inner.clone();
        future_into_py(py, async move {
            let mut guard = inner.write().await;
            guard.broadcast(payload).await
                .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
            Ok(())
        })
    }
}

/// Python wrapper for DistributedPlanner.
#[pyclass]
struct PyDistributedPlanner {
    inner: Arc<RwLock<DistributedPlanner<TwoPhaseBoundedConsensus<Assignment>>>>,
}

#[pymethods]
impl PyDistributedPlanner {
    #[new]
    fn new(local_agent_id: u64, participant_ids: Vec<u64>) -> PyResult<Self> {
        let config = DistributedPlannerConfig {
            local_agent_id: AgentId(local_agent_id),
            participant_agents: participant_ids.into_iter().map(AgentId).collect(),
            consensus_config: BoundedConsensusConfig {
                timeout_ms: 5000,
                max_rounds: 3,
            },
            transport_config: MeshTransportConfig::in_memory(),
        };
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
        let planner = rt.block_on(async {
            DistributedPlanner::new(config).await
        }).map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
        Ok(Self {
            inner: Arc::new(RwLock::new(planner)),
        })
    }

    /// Start the planner (asynchronous).
    fn start<'py>(&self, py: Python<'py>) -> PyResult<&'py PyAny> {
        let inner = self.inner.clone();
        future_into_py(py, async move {
            let mut guard = inner.write().await;
            guard.start().await
                .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
            Ok(())
        })
    }

    /// Add a task (asynchronous).
    fn add_task<'py>(&self, py: Python<'py>, task_id: String, description: String, required_resources: Vec<String>, estimated_duration_secs: u64) -> PyResult<&'py PyAny> {
        let inner = self.inner.clone();
        future_into_py(py, async move {
            let guard = inner.read().await;
            let task = Task {
                id: task_id,
                description,
                required_resources,
                estimated_duration_secs,
            };
            guard.add_task(task).await
                .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
            Ok(())
        })
    }

    /// Get all tasks (asynchronous).
    fn get_tasks<'py>(&self, py: Python<'py>) -> PyResult<&'py PyAny> {
        let inner = self.inner.clone();
        future_into_py(py, async move {
            let guard = inner.read().await;
            let tasks = guard.get_tasks().await;
            // Convert to Python list of dicts
            let py_tasks: Vec<PyObject> = tasks.into_iter().map(|task| {
                Python::with_gil(|py| {
                    let dict = PyDict::new(py);
                    dict.set_item("id", task.id).unwrap();
                    dict.set_item("description", task.description).unwrap();
                    dict.set_item("required_resources", task.required_resources).unwrap();
                    dict.set_item("estimated_duration_secs", task.estimated_duration_secs).unwrap();
                    dict.into()
                })
            }).collect();
            Ok(py_tasks)
        })
    }

    /// Run round‑robin planning (asynchronous).
    fn run_round_robin<'py>(&self, py: Python<'py>) -> PyResult<&'py PyAny> {
        let inner = self.inner.clone();
        future_into_py(py, async move {
            let guard = inner.read().await;
            let algorithm = distributed_planner::algorithms::RoundRobinPlanner;
            let assignments = guard.run_planning_algorithm(&algorithm).await
                .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
            // Convert assignments to Python list
            let py_assignments: Vec<PyObject> = assignments.into_iter().map(|ass| {
                Python::with_gil(|py| {
                    let dict = PyDict::new(py);
                    dict.set_item("task_id", ass.task_id).unwrap();
                    dict.set_item("agent_id", ass.agent_id.0).unwrap();
                    dict.set_item("status", format!("{:?}", ass.status)).unwrap();
                    dict.into()
                })
            }).collect();
            Ok(py_assignments)
        })
    }
}

/// Python wrapper for LocalPlanner.
#[pyclass]
struct PyLocalPlanner {
    inner: Arc<RwLock<LocalPlanner>>,
}

#[pymethods]
impl PyLocalPlanner {
    #[new]
    fn new(agent_id: u64) -> PyResult<Self> {
        let config = LocalPlannerConfig {
            agent_id: AgentId(agent_id),
            ..Default::default()
        };
        let planner = LocalPlanner::new(config);
        Ok(Self {
            inner: Arc::new(RwLock::new(planner)),
        })
    }

    /// Execute a task (asynchronous).
    fn execute_task<'py>(&self, py: Python<'py>, task_id: String) -> PyResult<&'py PyAny> {
        let inner = self.inner.clone();
        future_into_py(py, async move {
            let mut guard = inner.write().await;
            guard.execute_task(&task_id).await
                .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
            Ok(())
        })
    }
}

/// Python wrapper for ResourceMonitor.
#[pyclass]
struct PyResourceMonitor {
    inner: Arc<RwLock<ResourceMonitor>>,
}

#[pymethods]
impl PyResourceMonitor {
    #[new]
    fn new(agent_id: u64) -> PyResult<Self> {
        let config = ResourceMonitorConfig {
            agent_id: AgentId(agent_id),
            ..Default::default()
        };
        let monitor = ResourceMonitor::new(config);
        Ok(Self {
            inner: Arc::new(RwLock::new(monitor)),
        })
    }

    /// Get current CPU usage (asynchronous).
    fn cpu_usage<'py>(&self, py: Python<'py>) -> PyResult<&'py PyAny> {
        let inner = self.inner.clone();
        future_into_py(py, async move {
            let guard = inner.read().await;
            let usage = guard.cpu_usage().await;
            Ok(usage)
        })
    }

    /// Get battery level (asynchronous).
    fn battery_level<'py>(&self, py: Python<'py>) -> PyResult<&'py PyAny> {
        let inner = self.inner.clone();
        future_into_py(py, async move {
            let guard = inner.read().await;
            let level = guard.battery_level().await;
            Ok(level)
        })
    }
}

/// Python wrapper for CrdtMap.
#[pyclass]
struct PyCrdtMap {
    inner: Arc<RwLock<CrdtMap>>,
}

#[pymethods]
impl PyCrdtMap {
    #[new]
    fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(CrdtMap::new())),
        }
    }

    /// Set a key‑value pair (value must be JSON‑serializable string).
    fn set<'py>(&self, py: Python<'py>, key: String, value: String, author: u64) -> PyResult<&'py PyAny> {
        let inner = self.inner.clone();
        future_into_py(py, async move {
            let mut guard = inner.write().await;
            let json_value: Value = serde_json::from_str(&value)
                .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
            guard.set(&key, json_value, AgentId(author));
            Ok(())
        })
    }

    /// Get a value by key.
    fn get<'py>(&self, py: Python<'py>, key: String) -> PyResult<&'py PyAny> {
        let inner = self.inner.clone();
        future_into_py(py, async move {
            let guard = inner.read().await;
            let opt: Option<Value> = guard.get(&key);
            Ok(opt.map(|v| v.to_string()))
        })
    }

    /// Merge another CrdtMap (by serialized delta?).
    /// For simplicity, we skip.
}