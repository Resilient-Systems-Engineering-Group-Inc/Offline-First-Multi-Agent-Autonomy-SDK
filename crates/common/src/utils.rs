//! Utility functions.

use crate::error::Result;
use std::time::{Duration, SystemTime};

/// Convert a `SystemTime` to milliseconds since Unix epoch.
pub fn system_time_to_millis(time: SystemTime) -> u64 {
    time.duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

/// Generate a random `AgentId` (for testing).
#[cfg(feature = "testing")]
pub fn random_agent_id() -> crate::types::AgentId {
    use rand::Rng;
    crate::types::AgentId(rand::thread_rng().gen())
}

/// Serialize a value to bytes using CBOR.
pub fn to_cbor<T: serde::Serialize>(value: &T) -> Result<Vec<u8>> {
    let mut buf = Vec::new();
    ciborium::ser::into_writer(value, &mut buf).map_err(|e| {
        crate::error::SdkError::Serialization(serde_json::Error::custom(e.to_string()))
    })?;
    Ok(buf)
}

/// Deserialize bytes from CBOR.
pub fn from_cbor<T: for<'de> serde::Deserialize<'de>>(bytes: &[u8]) -> Result<T> {
    ciborium::de::from_reader(bytes).map_err(|e| {
        crate::error::SdkError::Serialization(serde_json::Error::custom(e.to_string()))
    })
}

/// Simple exponential backoff.
pub async fn exponential_backoff(
    attempt: u32,
    base_delay: Duration,
    max_delay: Duration,
) -> Duration {
    let delay = base_delay * 2u32.pow(attempt);
    std::cmp::min(delay, max_delay)
}