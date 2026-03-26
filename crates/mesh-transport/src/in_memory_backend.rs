//! In‑memory backend for testing and simulation.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tokio::time::{sleep, Duration};
use futures::stream::{BoxStream, StreamExt};
use async_trait::async_trait;
use lazy_static::lazy_static;

use crate::message::TransportEvent;
use common::types::{AgentId, PeerInfo};
use common::error::Result;
use crate::backend::Backend;
use crate::transport::MeshTransportConfig;

lazy_static! {
    static ref GLOBAL_ROUTER: Arc<InMemoryRouter> = Arc::new(InMemoryRouter::new());
}

/// Shared router that forwards messages between backends.
struct InMemoryRouter {
    /// Map from agent ID to its event sender.
    subscribers: RwLock<HashMap<AgentId, mpsc::UnboundedSender<TransportEvent>>>,
}

impl InMemoryRouter {
    fn new() -> Self {
        Self {
            subscribers: RwLock::new(HashMap::new()),
        }
    }

    /// Register a subscriber for the given agent ID.
    async fn subscribe(&self, agent_id: AgentId, tx: mpsc::UnboundedSender<TransportEvent>) {
        self.subscribers.write().await.insert(agent_id, tx);
    }

    /// Unregister a subscriber.
    async fn unsubscribe(&self, agent_id: AgentId) {
        self.subscribers.write().await.remove(&agent_id);
    }

    /// Send an event to a target agent after a delay.
    async fn send(&self, target: AgentId, event: TransportEvent, delay: Duration) {
        let tx = self.subscribers.read().await.get(&target).cloned();
        if let Some(tx) = tx {
            tokio::spawn(async move {
                sleep(delay).await;
                let _ = tx.send(event);
            });
        }
    }

    /// Broadcast an event to all subscribers except the sender.
    async fn broadcast(&self, sender: AgentId, event: TransportEvent, delay: Duration) {
        let subscribers = self.subscribers.read().await;
        for (&agent_id, tx) in subscribers.iter() {
            if agent_id != sender {
                let event_clone = event.clone();
                let tx_clone = tx.clone();
                tokio::spawn(async move {
                    sleep(delay).await;
                    let _ = tx_clone.send(event_clone);
                });
            }
        }
    }
}

/// In‑memory backend that uses a shared router.
pub struct InMemoryBackend {
    local_agent_id: AgentId,
    event_tx: mpsc::UnboundedSender<TransportEvent>,
    event_rx: mpsc::UnboundedReceiver<TransportEvent>,
    known_peers: Vec<PeerInfo>,
}

impl InMemoryBackend {
    /// Create a new in‑memory backend.
    pub async fn new(config: MeshTransportConfig) -> Result<Self> {
        let (event_tx, event_rx) = mpsc::unbounded_channel();
        GLOBAL_ROUTER.subscribe(config.local_agent_id, event_tx.clone()).await;

        // Simulate known peers (for demo, we'll add them later via `add_peer`)
        let known_peers = vec![];

        Ok(Self {
            local_agent_id: config.local_agent_id,
            event_tx,
            event_rx,
            known_peers,
        })
    }

    /// Manually add a peer for simulation (call this before starting).
    pub async fn add_peer(&mut self, peer: PeerInfo) {
        self.known_peers.push(peer);
    }
}

#[async_trait]
impl Backend for InMemoryBackend {
    async fn start(&mut self) -> Result<()> {
        // Nothing to do; the router is already running.
        Ok(())
    }

    async fn stop(&mut self) -> Result<()> {
        GLOBAL_ROUTER.unsubscribe(self.local_agent_id).await;
        Ok(())
    }

    async fn send_to(&mut self, peer_id: AgentId, payload: Vec<u8>) -> Result<()> {
        let event = TransportEvent::MessageReceived {
            from: self.local_agent_id,
            payload,
        };
        // Simulate network latency: random delay between 10‑100 ms
        let delay = Duration::from_millis(10);
        GLOBAL_ROUTER.send(peer_id, event, delay).await;
        Ok(())
    }

    async fn broadcast(&mut self, payload: Vec<u8>) -> Result<()> {
        let event = TransportEvent::MessageReceived {
            from: self.local_agent_id,
            payload,
        };
        let delay = Duration::from_millis(10);
        GLOBAL_ROUTER.broadcast(self.local_agent_id, event, delay).await;
        Ok(())
    }

    fn peers(&self) -> Vec<PeerInfo> {
        self.known_peers.clone()
    }

    fn events(&mut self) -> BoxStream<'static, TransportEvent> {
        let rx = std::mem::replace(&mut self.event_rx, mpsc::unbounded_channel().1);
        Box::pin(tokio_stream::wrappers::UnboundedReceiverStream::new(rx))
    }

    fn local_agent_id(&self) -> AgentId {
        self.local_agent_id
    }
}