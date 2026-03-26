//! Delta compression and serialization.

use crate::delta::Delta;
use serde_cbor;
use flate2::{Compression, write::ZlibEncoder, read::ZlibDecoder};
use std::io::{Read, Write};

/// Serialization format for deltas.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeltaFormat {
    /// JSON (human‑readable, larger).
    Json,
    /// CBOR (binary, compact).
    Cbor,
}

/// Compression algorithm.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompressionAlgo {
    /// No compression.
    None,
    /// Zlib compression.
    Zlib,
}

/// Serialize a delta to bytes using the specified format.
pub fn serialize_delta(delta: &Delta, format: DeltaFormat) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    match format {
        DeltaFormat::Json => {
            let bytes = serde_json::to_vec(delta)?;
            Ok(bytes)
        }
        DeltaFormat::Cbor => {
            let bytes = serde_cbor::to_vec(delta)?;
            Ok(bytes)
        }
    }
}

/// Deserialize bytes back to a delta.
pub fn deserialize_delta(bytes: &[u8], format: DeltaFormat) -> Result<Delta, Box<dyn std::error::Error>> {
    match format {
        DeltaFormat::Json => {
            let delta = serde_json::from_slice(bytes)?;
            Ok(delta)
        }
        DeltaFormat::Cbor => {
            let delta = serde_cbor::from_slice(bytes)?;
            Ok(delta)
        }
    }
}

/// Compress bytes using the specified algorithm.
pub fn compress(bytes: &[u8], algo: CompressionAlgo) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    match algo {
        CompressionAlgo::None => Ok(bytes.to_vec()),
        CompressionAlgo::Zlib => {
            let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
            encoder.write_all(bytes)?;
            Ok(encoder.finish()?)
        }
    }
}

/// Decompress bytes.
pub fn decompress(bytes: &[u8], algo: CompressionAlgo) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    match algo {
        CompressionAlgo::None => Ok(bytes.to_vec()),
        CompressionAlgo::Zlib => {
            let mut decoder = ZlibDecoder::new(bytes);
            let mut decompressed = Vec::new();
            decoder.read_to_end(&mut decompressed)?;
            Ok(decompressed)
        }
    }
}

/// Combined serialization and compression.
pub fn delta_to_bytes(delta: &Delta, format: DeltaFormat, compression: CompressionAlgo) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let serialized = serialize_delta(delta, format)?;
    compress(&serialized, compression)
}

/// Combined decompression and deserialization.
pub fn bytes_to_delta(bytes: &[u8], format: DeltaFormat, compression: CompressionAlgo) -> Result<Delta, Box<dyn std::error::Error>> {
    let decompressed = decompress(bytes, compression)?;
    deserialize_delta(&decompressed, format)
}

// --- Batching and deduplication ---

use std::collections::HashSet;
use serde::{Serialize, Deserialize};

/// A batch of deltas that can be sent together.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeltaBatch {
    /// List of deltas in the batch.
    pub deltas: Vec<Delta>,
    /// Timestamp when the batch was created (monotonic).
    pub timestamp: u64,
    /// Optional sequence number for ordering.
    pub seq: u64,
}

impl DeltaBatch {
    /// Create a new batch from a list of deltas.
    pub fn new(deltas: Vec<Delta>, timestamp: u64, seq: u64) -> Self {
        Self { deltas, timestamp, seq }
    }

    /// Merge another batch into this one, preserving order.
    pub fn merge(&mut self, other: DeltaBatch) {
        self.deltas.extend(other.deltas);
        // Keep the earlier timestamp
        self.timestamp = self.timestamp.min(other.timestamp);
    }

    /// Deduplicate operations within the batch (naïve).
    /// This removes duplicate ops based on a hash of the operation.
    pub fn deduplicate(&mut self) {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut seen = HashSet::new();
        let mut deduped = Vec::new();
        for delta in &self.deltas {
            let mut new_ops = Vec::new();
            for op in &delta.ops {
                let mut hasher = DefaultHasher::new();
                // Hash the operation (simplistic, for demonstration)
                // In production you'd want a more robust deduplication.
                match op {
                    crate::delta::Op::Set { key, value, author, seq } => {
                        key.hash(&mut hasher);
                        serde_json::to_string(value).unwrap().hash(&mut hasher);
                        author.hash(&mut hasher);
                        seq.hash(&mut hasher);
                    }
                    crate::delta::Op::Delete { key, author, seq } => {
                        key.hash(&mut hasher);
                        author.hash(&mut hasher);
                        seq.hash(&mut hasher);
                    }
                }
                let hash = hasher.finish();
                if seen.insert(hash) {
                    new_ops.push(op.clone());
                }
            }
            if !new_ops.is_empty() {
                let new_delta = Delta::new(delta.author, new_ops, delta.clock.clone());
                deduped.push(new_delta);
            }
        }
        self.deltas = deduped;
    }
}

/// Create a batch from a list of deltas, optionally deduplicating.
pub fn create_batch(deltas: Vec<Delta>, timestamp: u64, seq: u64, dedup: bool) -> DeltaBatch {
    let mut batch = DeltaBatch::new(deltas, timestamp, seq);
    if dedup {
        batch.deduplicate();
    }
    batch
}

/// Serialize a batch to bytes.
pub fn serialize_batch(batch: &DeltaBatch, format: DeltaFormat) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    match format {
        DeltaFormat::Json => Ok(serde_json::to_vec(batch)?),
        DeltaFormat::Cbor => Ok(serde_cbor::to_vec(batch)?),
    }
}

/// Deserialize bytes back to a batch.
pub fn deserialize_batch(bytes: &[u8], format: DeltaFormat) -> Result<DeltaBatch, Box<dyn std::error::Error>> {
    match format {
        DeltaFormat::Json => Ok(serde_json::from_slice(bytes)?),
        DeltaFormat::Cbor => Ok(serde_cbor::from_slice(bytes)?),
    }
}

/// Compress a batch (serialize + compress).
pub fn batch_to_bytes(batch: &DeltaBatch, format: DeltaFormat, compression: CompressionAlgo) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let serialized = serialize_batch(batch, format)?;
    compress(&serialized, compression)
}

/// Decompress and deserialize a batch.
pub fn bytes_to_batch(bytes: &[u8], format: DeltaFormat, compression: CompressionAlgo) -> Result<DeltaBatch, Box<dyn std::error::Error>> {
    let decompressed = decompress(bytes, compression)?;
    deserialize_batch(&decompressed, format)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::delta::{Delta, Op};
    use common::types::{AgentId, VectorClock};
    use serde_json::json;

    fn sample_delta() -> Delta {
        Delta::new(
            AgentId(42),
            vec![
                Op::Set {
                    key: "foo".to_string(),
                    value: json!("bar"),
                    author: AgentId(42),
                    seq: 1,
                },
            ],
            VectorClock::from_entries(vec![(AgentId(42), 1)]),
        )
    }

    #[test]
    fn test_serialize_json() {
        let delta = sample_delta();
        let bytes = serialize_delta(&delta, DeltaFormat::Json).unwrap();
        let delta2 = deserialize_delta(&bytes, DeltaFormat::Json).unwrap();
        assert_eq!(delta.author, delta2.author);
        assert_eq!(delta.ops.len(), delta2.ops.len());
    }

    #[test]
    fn test_serialize_cbor() {
        let delta = sample_delta();
        let bytes = serialize_delta(&delta, DeltaFormat::Cbor).unwrap();
        let delta2 = deserialize_delta(&bytes, DeltaFormat::Cbor).unwrap();
        assert_eq!(delta.author, delta2.author);
        assert_eq!(delta.ops.len(), delta2.ops.len());
    }

    #[test]
    fn test_compress_zlib() {
        let data = b"hello world".to_vec();
        let compressed = compress(&data, CompressionAlgo::Zlib).unwrap();
        let decompressed = decompress(&compressed, CompressionAlgo::Zlib).unwrap();
        assert_eq!(data, decompressed);
    }

    #[test]
    fn test_delta_to_bytes_roundtrip() {
        let delta = sample_delta();
        let bytes = delta_to_bytes(&delta, DeltaFormat::Cbor, CompressionAlgo::Zlib).unwrap();
        let delta2 = bytes_to_delta(&bytes, DeltaFormat::Cbor, CompressionAlgo::Zlib).unwrap();
        assert_eq!(delta.author, delta2.author);
    }

    // --- Batch tests ---

    #[test]
    fn test_batch_creation() {
        let delta1 = sample_delta();
        let delta2 = Delta::new(
            AgentId(43),
            vec![Op::Set {
                key: "baz".to_string(),
                value: json!("qux"),
                author: AgentId(43),
                seq: 1,
            }],
            VectorClock::from_entries(vec![(AgentId(43), 1)]),
        );
        let batch = create_batch(vec![delta1.clone(), delta2.clone()], 12345, 1, false);
        assert_eq!(batch.deltas.len(), 2);
        assert_eq!(batch.timestamp, 12345);
        assert_eq!(batch.seq, 1);
    }

    #[test]
    fn test_batch_deduplicate() {
        let delta = sample_delta();
        // Duplicate delta
        let batch = create_batch(vec![delta.clone(), delta.clone()], 12345, 1, true);
        // After deduplication, only one delta should remain
        assert_eq!(batch.deltas.len(), 1);
    }

    #[test]
    fn test_batch_serialize_roundtrip() {
        let delta1 = sample_delta();
        let delta2 = Delta::new(
            AgentId(43),
            vec![Op::Set {
                key: "baz".to_string(),
                value: json!("qux"),
                author: AgentId(43),
                seq: 1,
            }],
            VectorClock::from_entries(vec![(AgentId(43), 1)]),
        );
        let batch = create_batch(vec![delta1, delta2], 12345, 1, false);
        let bytes = serialize_batch(&batch, DeltaFormat::Cbor).unwrap();
        let batch2 = deserialize_batch(&bytes, DeltaFormat::Cbor).unwrap();
        assert_eq!(batch.deltas.len(), batch2.deltas.len());
        assert_eq!(batch.timestamp, batch2.timestamp);
        assert_eq!(batch.seq, batch2.seq);
    }

    #[test]
    fn test_batch_compress_roundtrip() {
        let delta1 = sample_delta();
        let delta2 = Delta::new(
            AgentId(43),
            vec![Op::Set {
                key: "baz".to_string(),
                value: json!("qux"),
                author: AgentId(43),
                seq: 1,
            }],
            VectorClock::from_entries(vec![(AgentId(43), 1)]),
        );
        let batch = create_batch(vec![delta1, delta2], 12345, 1, false);
        let bytes = batch_to_bytes(&batch, DeltaFormat::Cbor, CompressionAlgo::Zlib).unwrap();
        let batch2 = bytes_to_batch(&bytes, DeltaFormat::Cbor, CompressionAlgo::Zlib).unwrap();
        assert_eq!(batch.deltas.len(), batch2.deltas.len());
    }
}