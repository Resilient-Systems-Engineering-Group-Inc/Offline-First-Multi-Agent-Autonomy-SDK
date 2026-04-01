//! WebSocket integration for real‑time communication with external services.
//!
//! This module provides WebSocket client and server implementations for
//! bidirectional real‑time communication between agents and external systems.

use std::sync::Arc;
use std::time::Duration;

use futures::{SinkExt, StreamExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, RwLock};
use tokio_tungstenite::{
    accept_async,
    tungstenite::{Error as WsError, Message},
    WebSocketStream,
};

use crate::error::{ApiIntegrationError, Result};

/// WebSocket server for exposing real‑time agent events.
pub struct WebSocketServer {
    addr: std::net::SocketAddr,
    event_tx: mpsc::UnboundedSender<ServerEvent>,
}

impl WebSocketServer {
    /// Create a new WebSocket server bound to the given address.
    pub fn new(addr: std::net::SocketAddr) -> Self {
        let (event_tx, _) = mpsc::unbounded_channel();
        Self { addr, event_tx }
    }

    /// Start the WebSocket server.
    ///
    /// This runs indefinitely, accepting connections and handling messages.
    pub async fn serve(self) -> Result<()> {
        let listener = TcpListener::bind(self.addr)
            .await
            .map_err(|e| ApiIntegrationError::ConnectionError(e.to_string()))?;

        tracing::info!("WebSocket server listening on {}", self.addr);

        while let Ok((stream, addr)) = listener.accept().await {
            tokio::spawn(handle_connection(stream, addr));
        }

        Ok(())
    }

    /// Get a sender for broadcasting events to connected clients.
    pub fn event_sender(&self) -> mpsc::UnboundedSender<ServerEvent> {
        self.event_tx.clone()
    }
}

/// Events that can be sent to WebSocket clients.
#[derive(Debug, Clone)]
pub enum ServerEvent {
    /// Broadcast a message to all connected clients.
    Broadcast(String),
    /// Send a message to a specific client.
    SendTo { client_id: u64, message: String },
    /// Notify that a client has connected.
    ClientConnected { client_id: u64, addr: std::net::SocketAddr },
    /// Notify that a client has disconnected.
    ClientDisconnected { client_id: u64 },
}

/// Handle a single WebSocket connection.
async fn handle_connection(stream: TcpStream, addr: std::net::SocketAddr) {
    let ws_stream = match accept_async(stream).await {
        Ok(ws) => ws,
        Err(e) => {
            tracing::error!("Failed to accept WebSocket connection from {}: {}", addr, e);
            return;
        }
    };

    tracing::info!("New WebSocket connection from {}", addr);
    let (mut write, mut read) = ws_stream.split();

    // Generate a simple client ID (in production, use proper authentication)
    let client_id = addr.port() as u64;

    // Handle incoming messages
    while let Some(msg) = read.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                tracing::debug!("Received from {}: {}", addr, text);
                // Echo back for now
                let _ = write.send(Message::Text(format!("Echo: {}", text))).await;
            }
            Ok(Message::Close(_)) => {
                tracing::info!("WebSocket connection closed by {}", addr);
                break;
            }
            Ok(_) => {
                // Ignore binary/ping/pong messages
            }
            Err(e) => {
                tracing::warn!("WebSocket error from {}: {}", addr, e);
                break;
            }
        }
    }

    tracing::info!("WebSocket connection closed for {}", addr);
}

/// WebSocket client for connecting to external WebSocket servers.
pub struct WebSocketClient {
    url: String,
    reconnect_interval: Duration,
}

impl WebSocketClient {
    /// Create a new WebSocket client.
    pub fn new(url: &str) -> Self {
        Self {
            url: url.to_string(),
            reconnect_interval: Duration::from_secs(5),
        }
    }

    /// Set the reconnection interval.
    pub fn with_reconnect_interval(mut self, interval: Duration) -> Self {
        self.reconnect_interval = interval;
        self
    }

    /// Connect to the WebSocket server and start listening for messages.
    ///
    /// Returns a channel for sending messages and a stream of incoming messages.
    pub async fn connect(
        &self,
    ) -> Result<(
        mpsc::UnboundedSender<ClientMessage>,
        mpsc::UnboundedReceiver<ServerMessage>,
    )> {
        let (ws_stream, _) = tokio_tungstenite::connect_async(&self.url)
            .await
            .map_err(|e| ApiIntegrationError::ConnectionError(e.to_string()))?;

        let (write, read) = ws_stream.split();

        let (tx_outgoing, rx_outgoing) = mpsc::unbounded_channel();
        let (tx_incoming, rx_incoming) = mpsc::unbounded_channel();

        // Spawn task to handle outgoing messages
        tokio::spawn(handle_outgoing(write, rx_outgoing));
        // Spawn task to handle incoming messages
        tokio::spawn(handle_incoming(read, tx_incoming));

        Ok((tx_outgoing, rx_incoming))
    }

    /// Connect with automatic reconnection.
    ///
    /// This will continuously try to reconnect if the connection is lost.
    pub async fn connect_with_reconnect(
        &self,
    ) -> Result<(
        mpsc::UnboundedSender<ClientMessage>,
        mpsc::UnboundedReceiver<ServerMessage>,
    )> {
        loop {
            match self.connect().await {
                Ok(channels) => return Ok(channels),
                Err(e) => {
                    tracing::warn!("Failed to connect to {}: {}. Retrying in {:?}", self.url, e, self.reconnect_interval);
                    tokio::time::sleep(self.reconnect_interval).await;
                }
            }
        }
    }
}

/// Messages that can be sent from the client to the server.
#[derive(Debug)]
pub enum ClientMessage {
    /// Send a text message.
    Text(String),
    /// Send binary data.
    Binary(Vec<u8>),
    /// Ping the server.
    Ping(Vec<u8>),
    /// Close the connection.
    Close,
}

/// Messages received from the server.
#[derive(Debug)]
pub enum ServerMessage {
    /// Text message from server.
    Text(String),
    /// Binary message from server.
    Binary(Vec<u8>),
    /// Ping from server.
    Ping(Vec<u8>),
    /// Pong from server.
    Pong(Vec<u8>),
    /// Connection closed.
    Closed,
}

/// Handle outgoing messages to the WebSocket server.
async fn handle_outgoing(
    mut write: impl SinkExt<Message> + Unpin,
    mut rx: mpsc::UnboundedReceiver<ClientMessage>,
) {
    while let Some(msg) = rx.recv().await {
        let ws_msg = match msg {
            ClientMessage::Text(text) => Message::Text(text),
            ClientMessage::Binary(data) => Message::Binary(data),
            ClientMessage::Ping(data) => Message::Ping(data),
            ClientMessage::Close => Message::Close(None),
        };

        if let Err(e) = write.send(ws_msg).await {
            tracing::error!("Failed to send WebSocket message: {}", e);
            break;
        }
    }
}

/// Handle incoming messages from the WebSocket server.
async fn handle_incoming(
    mut read: impl StreamExt<Item = Result<Message, WsError>> + Unpin,
    tx: mpsc::UnboundedSender<ServerMessage>,
) {
    while let Some(msg) = read.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                let _ = tx.send(ServerMessage::Text(text));
            }
            Ok(Message::Binary(data)) => {
                let _ = tx.send(ServerMessage::Binary(data));
            }
            Ok(Message::Ping(data)) => {
                let _ = tx.send(ServerMessage::Ping(data));
            }
            Ok(Message::Pong(data)) => {
                let _ = tx.send(ServerMessage::Pong(data));
            }
            Ok(Message::Close(_)) => {
                let _ = tx.send(ServerMessage::Closed);
                break;
            }
            Ok(Message::Frame(_)) => {
                // Ignore raw frames
            }
            Err(e) => {
                tracing::warn!("WebSocket read error: {}", e);
                break;
            }
        }
    }
}

/// Integration between WebSocket and mesh transport.
pub struct WebSocketMeshBridge {
    /// WebSocket client for external communication
    ws_client: Option<WebSocketClient>,
    /// Mesh transport for internal agent communication
    mesh_transport: Arc<dyn crate::mesh_transport::Transport>,
    /// Channel for sending messages to WebSocket
    ws_tx: Option<mpsc::UnboundedSender<ClientMessage>>,
}

impl WebSocketMeshBridge {
    /// Create a new bridge between WebSocket and mesh transport.
    pub fn new(mesh_transport: Arc<dyn crate::mesh_transport::Transport>) -> Self {
        Self {
            ws_client: None,
            mesh_transport,
            ws_tx: None,
        }
    }

    /// Connect to an external WebSocket server.
    pub async fn connect(&mut self, url: &str) -> Result<()> {
        let client = WebSocketClient::new(url);
        let (tx_outgoing, mut rx_incoming) = client.connect().await?;
        
        self.ws_client = Some(client);
        self.ws_tx = Some(tx_outgoing);

        // Spawn task to forward incoming WebSocket messages to mesh
        let mesh_transport = self.mesh_transport.clone();
        tokio::spawn(async move {
            while let Some(msg) = rx_incoming.recv().await {
                match msg {
                    ServerMessage::Text(text) => {
                        tracing::debug!("Forwarding WebSocket message to mesh: {}", text);
                        // In a real implementation, you would parse and forward to mesh
                    }
                    _ => {}
                }
            }
        });

        Ok(())
    }

    /// Send a mesh message to the WebSocket server.
    pub async fn send_to_websocket(&self, message: String) -> Result<()> {
        match &self.ws_tx {
            Some(tx) => {
                tx.send(ClientMessage::Text(message))
                    .map_err(|e| ApiIntegrationError::ConnectionError(e.to_string()))?;
                Ok(())
            }
            None => Err(ApiIntegrationError::NotConnected(
                "WebSocket client not connected".to_string(),
            )),
        }
    }

    /// Broadcast a message from mesh to all WebSocket clients (if server mode).
    pub async fn broadcast_to_websocket(&self, _message: String) -> Result<()> {
        // In server mode, you would maintain a list of connected clients
        // and broadcast to all of them
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_websocket_client_creation() {
        let client = WebSocketClient::new("ws://localhost:8080/ws");
        assert_eq!(client.url, "ws://localhost:8080/ws");
        assert_eq!(client.reconnect_interval, Duration::from_secs(5));
    }

    #[test]
    fn test_server_event_enum() {
        let broadcast = ServerEvent::Broadcast("hello".to_string());
        match broadcast {
            ServerEvent::Broadcast(msg) => assert_eq!(msg, "hello"),
            _ => panic!("Wrong variant"),
        }

        let send_to = ServerEvent::SendTo {
            client_id: 42,
            message: "test".to_string(),
        };
        match send_to {
            ServerEvent::SendTo { client_id, message } => {
                assert_eq!(client_id, 42);
                assert_eq!(message, "test");
            }
            _ => panic!("Wrong variant"),
        }
    }
}