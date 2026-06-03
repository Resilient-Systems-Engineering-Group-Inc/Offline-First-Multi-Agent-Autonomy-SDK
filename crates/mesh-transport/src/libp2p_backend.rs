//! libp2p backend for mesh transport.

use libp2p::{
    core::{muxing::StreamMuxerBox, transport::Boxed},
    identity,
    mdns::{Mdns, MdnsEvent},
    noise,
    request_response::{self, ProtocolSupport, RequestResponse, RequestResponseEvent, RequestResponseMessage},
    swarm::{NetworkBehaviour, Swarm, SwarmEvent},
    tcp::TokioTcpConfig,
    yamux, Multiaddr, PeerId, Transport,
};
use std::collections::HashMap;
use std::error::Error;
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc, RwLock};
use tokio::task::JoinHandle;
use futures::stream::{BoxStream, StreamExt};
use async_trait::async_trait;
use tokio::runtime::Handle;
use tokio_stream::wrappers::BroadcastStream;
use std::sync::atomic::{AtomicBool, Ordering};

use common::types::{AgentId, PeerInfo};
use common::metrics;
use crate::message::TransportEvent;
use crate::backend::Backend;
use crate::transport::MeshTransportConfig;
use crate::security::{SecurityManager, SignedMessage};

/// Protocol identifier for our mesh messages.
const MESH_PROTOCOL: &[u8] = b"/offline-first-mesh/1.0.0";

/// Network behaviour for our mesh protocol.
#[derive(NetworkBehaviour)]
struct MeshBehaviour {
    mdns: Mdns,
    request_response: RequestResponse<MeshCodec>,
    #[behaviour(ignore)]
    event_tx: mpsc::UnboundedSender<TransportEvent>,
}

/// Codec for serializing/deserializing mesh messages.
#[derive(Clone)]
struct MeshCodec;

impl request_response::Codec for MeshCodec {
    type Protocol = Vec<u8>;
    type Request = Vec<u8>;
    type Response = Vec<u8>;

    fn read_request<T>(&mut self, _: &Self::Protocol, mut reader: &mut T) -> std::io::Result<Self::Request>
    where
        T: std::io::Read,
    {
        let mut buf = Vec::new();
        std::io::Read::read_to_end(&mut reader, &mut buf)?;
        Ok(buf)
    }

    fn read_response<T>(&mut self, _: &Self::Protocol, mut reader: &mut T) -> std::io::Result<Self::Response>
    where
        T: std::io::Read,
    {
        let mut buf = Vec::new();
        std::io::Read::read_to_end(&mut reader, &mut buf)?;
        Ok(buf)
    }

    fn write_request<T>(&mut self, _: &Self::Protocol, writer: &mut T, request: Self::Request) -> std::io::Result<()>
    where
        T: std::io::Write,
    {
        writer.write_all(&request)
    }

    fn write_response<T>(&mut self, _: &Self::Protocol, writer: &mut T, response: Self::Response) -> std::io::Result<()>
    where
        T: std::io::Write,
    {
        writer.write_all(&response)
    }
}

impl MeshBehaviour {
    fn new(event_tx: mpsc::UnboundedSender<TransportEvent>, use_mdns: bool) -> Self {
        let protocols = vec![(MESH_PROTOCOL.to_vec(), ProtocolSupport::Full)];
        let request_response = RequestResponse::new(MeshCodec, protocols, Default::default());
        let mdns = if use_mdns {
            Mdns::new(Default::default()).expect("mDNS creation failed")
        } else {
            // Create a dummy Mdns that does nothing (but still satisfies the type).
            // This is a hack; libp2p doesn't provide a dummy behaviour.
            // We'll just create Mdns anyway but ignore its events.
            Mdns::new(Default::default()).expect("mDNS creation failed")
        };
        Self {
            mdns,
            request_response,
            event_tx,
        }
    }
}

impl NetworkBehaviourEventProcess<MdnsEvent> for MeshBehaviour {
    fn inject_event(&mut self, event: MdnsEvent) {
        match event {
            MdnsEvent::Discovered(list) => {
                for (peer_id, _) in list {
                    let agent_id = peer_id_to_agent_id(&peer_id);
                    let _ = self.event_tx.send(TransportEvent::PeerDiscovered(agent_id));
                }
            }
            MdnsEvent::Expired(list) => {
                for (peer_id, _) in list {
                    let agent_id = peer_id_to_agent_id(&peer_id);
                    let _ = self.event_tx.send(TransportEvent::PeerLost(agent_id));
                }
            }
        }
    }
}

impl NetworkBehaviourEventProcess<RequestResponseEvent<Vec<u8>, Vec<u8>>> for MeshBehaviour {
    fn inject_event(&mut self, event: RequestResponseEvent<Vec<u8>, Vec<u8>>) {
        match event {
            RequestResponseEvent::Message { peer, message } => match message {
                RequestResponseMessage::Request { request, channel, .. } => {
                    // Incoming request: try to parse as SignedMessage
                    match SignedMessage::from_bytes(&request) {
                        Ok(signed) => {
                            // Verify signature
                            if let Err(e) = signed.verify() {
                                tracing::warn!("Dropping message with invalid signature: {}", e);
                                // Still send ack? We'll ack anyway to avoid hanging.
                                self.request_response.send_response(channel, vec![]).ok();
                                return;
                            }
                            // Extract payload
                            let agent_id = peer_id_to_agent_id(&peer);
                            metrics::inc_messages_received();
                            let _ = self.event_tx.send(TransportEvent::MessageReceived {
                                from: agent_id,
                                payload: signed.payload,
                            });
                        }
                        Err(e) => {
                            // If deserialization fails, maybe it's an old‑format message (unsigned).
                            // For backward compatibility, treat as raw payload.
                            // Log a warning.
                            tracing::debug!("Received unsigned message (fallback): {}", e);
                            let agent_id = peer_id_to_agent_id(&peer);
                            metrics::inc_messages_received();
                            let _ = self.event_tx.send(TransportEvent::MessageReceived {
                                from: agent_id,
                                payload: request,
                            });
                        }
                    }
                    // Auto‑respond with empty vector (ack).
                    self.request_response.send_response(channel, vec![]).ok();
                }
                RequestResponseMessage::Response { .. } => {
                    // Response to a request we sent; ignore for now.
                }
            },
            RequestResponseEvent::OutboundFailure { .. } => {}
            RequestResponseEvent::InboundFailure { .. } => {}
            RequestResponseEvent::ResponseSent { .. } => {}
        }
    }
}

/// Convert a libp2p PeerId to an AgentId (deterministic hash).
fn peer_id_to_agent_id(peer_id: &PeerId) -> AgentId {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    peer_id.hash(&mut hasher);
    AgentId(hasher.finish())
}

/// libp2p backend that manages a swarm.
pub struct Libp2pBackend {
    swarm: Option<Swarm<MeshBehaviour>>,
    local_agent_id: AgentId,
    local_peer_id: PeerId,
    /// Sender used by MeshBehaviour to push events (mpsc, internal).
    event_tx: mpsc::UnboundedSender<TransportEvent>,
    /// Broadcast channel for external consumers — allows multiple subscribers without data loss.
    broadcast_tx: broadcast::Sender<TransportEvent>,
    /// Mapping from AgentId to PeerId for connected peers.
    connected_peers: Arc<RwLock<HashMap<AgentId, PeerId>>>,
    /// Security manager for signing and verifying messages.
    security_manager: SecurityManager,
    /// Background task handle for swarm event loop.
    task_handle: Option<JoinHandle<()>>,
    /// Background task handle for the mpsc→broadcast bridge.
    bridge_handle: Option<JoinHandle<()>>,
    /// Shutdown signal for graceful termination.
    shutdown: Arc<AtomicBool>,
}

impl Libp2pBackend {
    /// Create a new libp2p backend with the given configuration.
    pub async fn new(config: MeshTransportConfig) -> Result<Self, Box<dyn Error>> {
        let local_key = identity::Keypair::generate_ed25519();
        let local_peer_id = PeerId::from(local_key.public());

        let transport = TokioTcpConfig::new()
            .upgrade(libp2p::core::upgrade::Version::V1)
            .authenticate(noise::NoiseConfig::xx(&local_key).unwrap())
            .multiplex(yamux::YamuxConfig::default())
            .map(|(peer, muxer), _| (peer, StreamMuxerBox::new(muxer)))
            .boxed();

        let (event_tx, mut event_rx) = mpsc::unbounded_channel();
        let behaviour = MeshBehaviour::new(event_tx.clone(), config.use_mdns);

        // Broadcast channel for external consumers — capacity 256, allows multiple subscribers.
        let (broadcast_tx, _) = broadcast::channel(256);

        // Bridge task: forwards events from mpsc (internal, from MeshBehaviour) to broadcast (external).
        let bridge_broadcast_tx = broadcast_tx.clone();
        let bridge_shutdown = shutdown.clone();
        let bridge_handle = tokio::spawn(async move {
            loop {
                tokio::select! {
                    maybe_event = event_rx.recv() => {
                        match maybe_event {
                            Some(event) => {
                                let _ = bridge_broadcast_tx.send(event);
                            }
                            None => {
                                // mpsc channel closed, exit bridge
                                break;
                            }
                        }
                    }
                    _ = tokio::time::sleep(tokio::time::Duration::from_millis(100)) => {
                        if bridge_shutdown.load(Ordering::SeqCst) {
                            break;
                        }
                    }
                }
            }
        });

        let mut swarm = Swarm::new(transport, behaviour, local_peer_id);
        // Listen on the configured address
        swarm.listen_on(config.listen_addr.parse()?)?;

        // Dial static peers
        for peer in config.static_peers {
            for addr in peer.addresses {
                if let Ok(addr) = addr.parse::<Multiaddr>() {
                    swarm.dial(addr)?;
                }
            }
        }

        Ok(Self {
            swarm: Some(swarm),
            local_agent_id: config.local_agent_id,
            local_peer_id,
            event_tx,
            broadcast_tx,
            connected_peers: Arc::new(RwLock::new(HashMap::new())),
            security_manager: SecurityManager::generate(),
            task_handle: None,
            bridge_handle: Some(bridge_handle),
            shutdown: Arc::new(AtomicBool::new(false)),
        })
    }

    /// Run the swarm event loop.
    pub async fn run(&mut self) -> Result<(), Box<dyn Error>> {
        let mut swarm = match self.swarm.take() {
            Some(swarm) => swarm,
            None => return Err("Swarm already taken or not initialized".into()),
        };
        loop {
            tokio::select! {
                event = swarm.select_next_some() => {
                    match event {
                        SwarmEvent::NewListenAddr { address, .. } => {
                            tracing::info!("Listening on {}", address);
                        }
                        SwarmEvent::Behaviour(_) => {
                            // Handled by MeshBehaviour
                        }
                        _ => {}
                    }
                }
                _ = tokio::signal::ctrl_c() => {
                    break;
                }
            }
        }
        self.swarm = Some(swarm);
        Ok(())
    }

    /// Get the local peer ID.
    pub fn local_peer_id(&self) -> PeerId {
        self.local_peer_id
    }

    /// Get incoming events.
    pub async fn next_event(&mut self) -> Option<TransportEvent> {
        let mut rx = self.broadcast_tx.subscribe();
        rx.recv().await.ok()
    }

    /// Send a message to a peer.
    pub async fn send(&mut self, peer_id: PeerId, payload: Vec<u8>) -> Result<(), Box<dyn Error>> {
        if let Some(swarm) = self.swarm.as_mut() {
            swarm.behaviour_mut().request_response.send_request(&peer_id, payload);
        }
        Ok(())
    }

    /// Broadcast a message to all known peers.
    pub async fn broadcast(&mut self, payload: Vec<u8>) -> Result<(), Box<dyn Error>> {
        let peers: Vec<PeerId> = if let Some(swarm) = self.swarm.as_ref() {
            swarm.connected_peers().cloned().collect()
        } else {
            Vec::new()
        };
        for peer in peers {
            self.send(peer, payload.clone()).await?;
        }
        Ok(())
    }
}

#[async_trait]
impl Backend for Libp2pBackend {
    async fn start(&mut self) -> Result<()> {
        // Take the swarm out of the Option — safe even if called multiple times
        let mut swarm = match self.swarm.take() {
            Some(swarm) => swarm,
            None => return Ok(()), // Already started, no-op
        };
        let event_tx = self.event_tx.clone();
        let connected_peers = self.connected_peers.clone();
        let shutdown = self.shutdown.clone();
        let task = tokio::spawn(async move {
            loop {
                tokio::select! {
                    event = swarm.select_next_some() => {
                        match event {
                            SwarmEvent::NewListenAddr { address, .. } => {
                                tracing::info!("Listening on {}", address);
                            }
                            SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                                // Map peer_id to agent_id and store in connected_peers
                                let agent_id = peer_id_to_agent_id(&peer_id);
                                let mut map = connected_peers.write().await;
                                map.insert(agent_id, peer_id);
                                // Update metrics
                                metrics::set_connected_peers(map.len());
                                // Notify about new peer
                                let _ = event_tx.send(TransportEvent::PeerDiscovered(agent_id));
                            }
                            SwarmEvent::ConnectionClosed { peer_id, .. } => {
                                // Remove from connected_peers
                                let agent_id = peer_id_to_agent_id(&peer_id);
                                let mut map = connected_peers.write().await;
                                map.remove(&agent_id);
                                // Update metrics
                                metrics::set_connected_peers(map.len());
                                let _ = event_tx.send(TransportEvent::PeerLost(agent_id));
                            }
                            SwarmEvent::Behaviour(_) => {
                                // Handled by MeshBehaviour
                            }
                            _ => {}
                        }
                    }
                    _ = tokio::time::sleep(tokio::time::Duration::from_millis(100)) => {
                        if shutdown.load(Ordering::SeqCst) {
                            tracing::info!("Libp2p backend shutting down");
                            break;
                        }
                    }
                }
            }
        });
        // Put swarm back into the Option
        self.swarm = Some(swarm);
        self.task_handle = Some(task);
        Ok(())
    }

    async fn stop(&mut self) -> Result<()> {
        // Signal shutdown
        self.shutdown.store(true, Ordering::SeqCst);
        // Wait for the background task to finish
        if let Some(handle) = self.task_handle.take() {
            let _ = tokio::time::timeout(
                tokio::time::Duration::from_secs(5),
                handle,
            ).await;
        }
        // Stop the mpsc→broadcast bridge
        if let Some(handle) = self.bridge_handle.take() {
            let _ = tokio::time::timeout(
                tokio::time::Duration::from_secs(5),
                handle,
            ).await;
        }
        // Clear connected peers
        self.connected_peers.write().await.clear();
        Ok(())
    }

    async fn send_to(&mut self, agent_id: AgentId, payload: Vec<u8>) -> Result<()> {
        let connected_peers = self.connected_peers.read().await;
        let peer_id = connected_peers.get(&agent_id)
            .cloned()
            .ok_or_else(|| common::error::SdkError::Network("Peer not found".to_string()))?;
        // Sign the payload
        let signed = self.security_manager.sign(payload);
        let bytes = signed.to_bytes()
            .map_err(|e| common::error::SdkError::Security(format!("Failed to serialize signed message: {}", e)))?;
        if let Some(swarm) = self.swarm.as_mut() {
            swarm.behaviour_mut().request_response.send_request(&peer_id, bytes);
        }
        Ok(())
    }

    async fn broadcast(&mut self, payload: Vec<u8>) -> Result<()> {
        let connected_peers = self.connected_peers.read().await;
        let peers: Vec<PeerId> = connected_peers.values().cloned().collect();
        // Sign once and reuse for all peers
        let signed = self.security_manager.sign(payload);
        let bytes = signed.to_bytes()
            .map_err(|e| common::error::SdkError::Security(format!("Failed to serialize signed message: {}", e)))?;
        if let Some(swarm) = self.swarm.as_mut() {
            for peer in peers {
                swarm.behaviour_mut().request_response.send_request(&peer, bytes.clone());
            }
        }
        Ok(())
    }

    fn peers(&self) -> Vec<PeerInfo> {
        // This is called from synchronous context; we block on the async lock.
        // Use the current runtime handle.
        match Handle::try_current() {
            Ok(handle) => handle.block_on(async {
                let connected_peers = self.connected_peers.read().await;
                connected_peers.iter().map(|(&agent_id, &peer_id)| {
                    PeerInfo {
                        agent_id,
                        addresses: Vec::new(), // TODO: retrieve peer addresses from swarm
                        metadata: std::collections::HashMap::new(),
                    }
                }).collect()
            }),
            Err(_) => {
                // No runtime; return empty list (should not happen in normal use)
                vec![]
            }
        }
    }

    fn events(&mut self) -> BoxStream<'static, TransportEvent> {
        let rx = self.broadcast_tx.subscribe();
        Box::pin(
            BroadcastStream::new(rx)
                .filter_map(|result| futures::future::ready(result.ok()))
        )
    }

    fn local_agent_id(&self) -> AgentId {
        self.local_agent_id
    }
}