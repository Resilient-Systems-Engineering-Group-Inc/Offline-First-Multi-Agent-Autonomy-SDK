//! Peer‑to‑peer mesh networking for offline‑first multi‑agent systems.

pub mod backend;
pub mod discovery;
pub mod connection;
pub mod message;
pub mod transport;
pub mod libp2p_backend;
pub mod security;
pub mod in_memory_backend;
pub mod webrtc_backend;
pub mod lora_backend;
pub mod qos;
#[cfg(feature = "post-quantum")]
pub mod post_quantum;

pub use transport::{MeshTransport, MeshTransportConfig, TransportEvent};

#[cfg(feature = "post-quantum")]
pub use post_quantum::{
    KyberKeyPair, DilithiumKeyPair, FalconKeyPair, PostQuantumError,
    DilithiumSignedMessage, PostQuantumSecurityManager,
    hybrid,
};