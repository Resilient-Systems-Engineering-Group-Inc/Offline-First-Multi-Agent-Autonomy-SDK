use pyo3::prelude::*;
use pyo3_asyncio::tokio::future_into_py;
use common::types::AgentId;
use mesh_transport::{MeshTransport, MeshTransportConfig};
use agent_core::Agent;
use serde_json::{Value, json};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Python module for Offline‑First Multi‑Agent Autonomy SDK.
#[pymodule]
fn offline_first_autonomy(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<PyAgent>()?;
    m.add_class::<PyMeshTransport>()?;
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