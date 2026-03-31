//! Federated learning server.

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tracing::{info, warn, error};

use crate::error::Error;
use crate::aggregation::{Aggregator, AggregationConfig, ClientUpdate};
use crate::privacy::PrivacyManager;

/// Server configuration.
#[derive(Debug, Clone)]
pub struct FederatedServerConfig {
    /// Aggregation configuration.
    pub aggregation_config: AggregationConfig,
    /// Maximum number of rounds.
    pub max_rounds: usize,
    /// Minimum clients per round.
    pub min_clients_per_round: usize,
    /// Model dimension.
    pub model_dim: usize,
    /// Enable privacy.
    pub enable_privacy: bool,
}

/// Server state.
#[derive(Debug, Clone)]
pub struct ServerState {
    /// Global model parameters.
    pub global_model: Vec<f64>,
    /// Round number.
    pub round: usize,
    /// Clients that participated in the last round.
    pub last_round_clients: Vec<String>,
    /// Model accuracy history.
    pub accuracy_history: Vec<f64>,
}

/// Federated learning server.
pub struct FederatedServer {
    config: FederatedServerConfig,
    state: Arc<RwLock<ServerState>>,
    aggregator: Aggregator,
    privacy_manager: Option<PrivacyManager>,
    clients: Arc<RwLock<HashMap<String, ClientUpdate>>>,
    event_tx: mpsc::UnboundedSender<ServerEvent>,
}

/// Events emitted by the server.
#[derive(Debug, Clone)]
pub enum ServerEvent {
    /// Round started.
    RoundStarted(usize),
    /// Round completed.
    RoundCompleted(usize, Vec<f64>),
    /// Client joined.
    ClientJoined(String),
    /// Client left.
    ClientLeft(String),
    /// Aggregation failed.
    AggregationFailed(String),
}

impl FederatedServer {
    /// Create a new federated learning server.
    pub fn new(
        config: FederatedServerConfig,
    ) -> (Self, mpsc::UnboundedReceiver<ServerEvent>) {
        let (event_tx, event_rx) = mpsc::unbounded_channel();

        let global_model = vec![0.0; config.model_dim];
        let state = Arc::new(RwLock::new(ServerState {
            global_model,
            round: 0,
            last_round_clients: Vec::new(),
            accuracy_history: Vec::new(),
        }));

        let aggregator = Aggregator::new(config.aggregation_config.clone());

        let privacy_manager = if config.enable_privacy {
            Some(PrivacyManager::new(None, None, None))
        } else {
            None
        };

        let server = Self {
            config,
            state,
            aggregator,
            privacy_manager,
            clients: Arc::new(RwLock::new(HashMap::new())),
            event_tx,
        };

        (server, event_rx)
    }

    /// Start the server (main loop).
    pub async fn start(&self) -> Result<(), Error> {
        info!("Starting federated learning server");

        for round in 1..=self.config.max_rounds {
            self.event_tx
                .send(ServerEvent::RoundStarted(round))
                .map_err(|e| Error::Network(e.to_string()))?;

            info!("Round {} started", round);

            // Wait for enough clients.
            let updates = self.wait_for_updates().await?;
            if updates.len() < self.config.min_clients_per_round {
                warn!("Not enough clients for round {}", round);
                continue;
            }

            // Aggregate updates.
            let aggregated = match self.aggregator.aggregate(&updates) {
                Some(model) => model,
                None => {
                    let err = "Aggregation failed".to_string();
                    self.event_tx
                        .send(ServerEvent::AggregationFailed(err.clone()))
                        .unwrap();
                    return Err(Error::Aggregation(err));
                }
            };

            // Apply privacy if enabled.
            let global_model = if let Some(pm) = &self.privacy_manager {
                pm.protect_update(&aggregated)
            } else {
                aggregated
            };

            // Update global model.
            {
                let mut state = self.state.write().await;
                state.global_model = global_model.clone();
                state.round = round;
                state.last_round_clients = updates.iter().map(|u| u.client_id.clone()).collect();
                // Simulate accuracy (placeholder).
                state.accuracy_history.push(0.8);
            }

            self.event_tx
                .send(ServerEvent::RoundCompleted(round, global_model))
                .map_err(|e| Error::Network(e.to_string()))?;

            info!("Round {} completed", round);
        }

        info!("Server finished after {} rounds", self.config.max_rounds);
        Ok(())
    }

    /// Wait for client updates (simulated).
    async fn wait_for_updates(&self) -> Result<Vec<ClientUpdate>, Error> {
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        let clients = self.clients.read().await;
        Ok(clients.values().cloned().collect())
    }

    /// Register a client update.
    pub async fn register_update(&self, update: ClientUpdate) -> Result<(), Error> {
        let mut clients = self.clients.write().await;
        clients.insert(update.client_id.clone(), update);
        Ok(())
    }

    /// Get current global model.
    pub async fn global_model(&self) -> Vec<f64> {
        self.state.read().await.global_model.clone()
    }

    /// Get server state.
    pub async fn state(&self) -> ServerState {
        self.state.read().await.clone()
    }

    /// Add a client (simulate joining).
    pub async fn add_client(&self, client_id: String) {
        let mut clients = self.clients.write().await;
        clients.insert(
            client_id.clone(),
            ClientUpdate {
                client_id: client_id.clone(),
                parameters: vec![0.0; self.config.model_dim],
                sample_count: 100,
                metadata: HashMap::new(),
            },
        );
        self.event_tx
            .send(ServerEvent::ClientJoined(client_id))
            .unwrap();
    }

    /// Remove a client.
    pub async fn remove_client(&self, client_id: &str) {
        let mut clients = self.clients.write().await;
        clients.remove(client_id);
        self.event_tx
            .send(ServerEvent::ClientLeft(client_id.to_string()))
            .unwrap();
    }
}

/// Server manager for multiple federated learning tasks.
pub struct ServerManager {
    servers: HashMap<String, FederatedServer>,
}

impl ServerManager {
    /// Create a new server manager.
    pub fn new() -> Self {
        Self {
            servers: HashMap::new(),
        }
    }

    /// Add a server.
    pub fn add_server(&mut self, id: String, server: FederatedServer) {
        self.servers.insert(id, server);
    }

    /// Get a server.
    pub fn get_server(&self, id: &str) -> Option<&FederatedServer> {
        self.servers.get(id)
    }
}