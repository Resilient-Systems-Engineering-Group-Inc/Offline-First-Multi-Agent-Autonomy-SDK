//! Quality of Service (QoS) definitions for streaming.

use serde::{Deserialize, Serialize};

/// Quality of Service levels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum QoS {
    /// At most once delivery (best effort).
    AtMostOnce = 0,
    /// At least once delivery (acknowledged).
    AtLeastOnce = 1,
    /// Exactly once delivery (guaranteed, ordered).
    ExactlyOnce = 2,
}

impl Default for QoS {
    fn default() -> Self {
        Self::AtMostOnce
    }
}

impl TryFrom<u8> for QoS {
    type Error = crate::error::Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::AtMostOnce),
            1 => Ok(Self::AtLeastOnce),
            2 => Ok(Self::ExactlyOnce),
            _ => Err(crate::error::Error::InvalidQoS(format!(
                "Invalid QoS value: {}",
                value
            ))),
        }
    }
}

/// Quality of Service configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityOfService {
    /// QoS level.
    pub level: QoS,
    /// Maximum retries for AtLeastOnce and ExactlyOnce.
    pub max_retries: u32,
    /// Timeout in milliseconds for acknowledgements.
    pub ack_timeout_ms: u64,
    /// Enable ordering (for ExactlyOnce).
    pub ordered: bool,
}

impl Default for QualityOfService {
    fn default() -> Self {
        Self {
            level: QoS::default(),
            max_retries: 3,
            ack_timeout_ms: 5000,
            ordered: true,
        }
    }
}