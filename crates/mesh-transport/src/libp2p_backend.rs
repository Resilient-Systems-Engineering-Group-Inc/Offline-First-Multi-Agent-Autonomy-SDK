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
use tokio::sync::mpsc;
use futures::stream::{BoxStream, StreamExt};
use async_trait::async_trait;

use common::types::{AgentId, PeerInfo};
use crate::message::TransportEvent;
use crate::backend::Backend;

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
    fn new(event_tx: mpsc::UnboundedSender<TransportEvent>) -> Self {
        let protocols = vec![(MESH_PROTOCOL.to_vec(), ProtocolSupport::Full)];
        let request_response = RequestResponse::new(MeshCodec, protocols, Default::default());
        Self {
            mdns: Mdns::new(Default::default()).expect("mDNS creation failed"),
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
                    // Incoming request: treat as a message from peer.
                    let agent_id = peer_id_to_agent_id(&peer);
                    let _ = self.event_tx.send(TransportEvent::MessageReceived {
                        from: agent_id,
                        payload: request,
                    });
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
    swarm: Swarm<MeshBehaviour>,
    local_agent_id: AgentId,
    local_peer_id: PeerId,
    event_rx: mpsc::UnboundedReceiver<TransportEvent>,
    /// Mapping from AgentId to PeerId (for sending).
    peer_map: HashMap<AgentId, PeerId>,
}

impl Libp2pBackend {
    /// Create a new libp2p backend.
    pub async fn new(local_agent_id: AgentId) -> Result<Self, Box<dyn Error>> {
        let local_key = identity::Keypair::generate_ed25519();
        let local_peer_id = PeerId::from(local_key.public());

        let transport = TokioTcpConfig::new()
            .upgrade(libp2p::core::upgrade::Version::V1)
            .authenticate(noise::NoiseConfig::xx(&local_key).unwrap())
            .multiplex(yamux::YamuxConfig::default())
            .map(|(peer, muxer), _| (peer, StreamMuxerBox::new(muxer)))
            .boxed();

        let (event_tx, event_rx) = mpsc::unbounded_channel();
        let behaviour = MeshBehaviour::new(event_tx);

        let mut swarm = Swarm::new(transport, behaviour, local_peer_id);
        swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse()?)?;

        Ok(Self {
            swarm,
            local_agent_id,
            local_peer_id,
            event_rx,
            peer_map: HashMap::new(),
        })
    }

    /// Run the swarm event loop.
    pub async fn run(&mut self) -> Result<(), Box<dyn Error>> {
        loop {
            tokio::select! {
                event = self.swarm.select_next_some() => {
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
        Ok(())
    }

    /// Get the local peer ID.
    pub fn local_peer_id(&self) -> PeerId {
        self.local_peer_id
    }

    /// Get incoming events.
    pub async fn next_event(&mut self) -> Option<TransportEvent> {
        self.event_rx.recv().await
    }

    /// Send a message to a peer.
    pub async fn send(&mut self, peer_id: PeerId, payload: Vec<u8>) -> Result<(), Box<dyn Error>> {
        self.swarm.behaviour_mut().request_response.send_request(&peer_id, payload);
        Ok(())
    }

    /// Broadcast a message to all known peers.
    pub async fn broadcast(&mut self, payload: Vec<u8>) -> Result<(), Box<dyn Error>> {
        let peers: Vec<PeerId> = self.swarm.connected_peers().cloned().collect();
        for peer in peers {
            self.send(peer, payload.clone()).await?;
        }
        Ok(())
    }
}

#[async_trait]
impl Backend for Libp2pBackend {
    async fn start(&mut self) -> Result<()> {
        // The swarm is already listening; we just need to start processing events.
        // In a real implementation we would spawn a task for `run`.
        Ok(())
    }

    async fn stop(&mut self) -> Result<()> {
        // No explicit stop mechanism yet.
        Ok(())
    }

    async fn send_to(&mut self, agent_id: AgentId, payload: Vec<u8>) -> Result<()> {
        // Look up PeerId from mapping, or derive from agent_id (if possible).
        // For now, we assume agent_id is derived from PeerId via hash, which is not invertible.
        // We'll need to iterate over known peers to find matching agent_id.
        let peer_id = self.peer_map.get(&agent_id)
            .cloned()
            .ok_or_else(|| common::error::SdkError::Network("Peer not found".to_string()))?;
        self.swarm.behaviour_mut().request_response.send_request(&peer_id, payload);
        Ok(())
    }

    async fn broadcast(&mut self, payload: Vec<u8>) -> Result<()> {
        self.broadcast(payload).await
            .map_err(|e| common::error::SdkError::Network(e.to_string()))
    }

    fn peers(&self) -> Vec<PeerInfo> {
        self.swarm.connected_peers()
            .map(|peer_id| {
                let agent_id = peer_id_to_agent_id(peer_id);
                PeerInfo {
                    agent_id,
                    addresses: vec![], // TODO: get addresses
                    metadata: Default::default(),
                }
            })
            .collect()
    }

    fn events(&mut self) -> BoxStream<'static, TransportEvent> {
        let rx = std::mem::replace(&mut self.event_rx, mpsc::unbounded_channel().1);
        Box::pin(tokio_stream::wrappers::UnboundedReceiverStream::new(rx))
    }

    fn local_agent_id(&self) -> AgentId {
        self.local_agent_id
    }
}