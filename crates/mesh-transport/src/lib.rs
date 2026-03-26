//! Peer‑to‑peer mesh networking for offline‑first multi‑agent systems.

pub mod discovery;
pub mod connection;
pub mod message;
pub mod transport;

pub use transport::{MeshTransport, MeshTransportConfig, TransportEvent};