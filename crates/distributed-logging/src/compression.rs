//! Compression utilities for log records and batches.

use crate::error::{Result, LogError};

/// Supported compression algorithms.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompressionAlgo {
    /// LZ4 fast compression.
    Lz4,
    /// Snappy compression.
    Snappy,
    /// Zlib (gzip‑compatible) compression.
    Zlib,
    /// No compression (identity).
    None,
}

impl CompressionAlgo {
    /// Compresses data using the selected algorithm.
    pub fn compress(&self, data: &[u8]) -> Result<Vec<u8>> {
        match self {
            CompressionAlgo::Lz4 => {
                #[cfg(feature = "lz4")]
                {
                    use lz4::EncoderBuilder;
                    let mut encoder = EncoderBuilder::new()
                        .level(4)
                        .build(Vec::new())
                        .map_err(|e| LogError::Compression(e.to_string()))?;
                    encoder.write_all(data).map_err(|e| LogError::Io(e))?;
                    let (compressed, result) = encoder.finish();
                    result.map_err(|e| LogError::Compression(e.to_string()))?;
                    Ok(compressed)
                }
                #[cfg(not(feature = "lz4"))]
                Err(LogError::Compression("LZ4 support not compiled".to_string()))
            }
            CompressionAlgo::Snappy => {
                #[cfg(feature = "snap")]
                {
                    use snap::raw::Encoder;
                    let mut encoder = Encoder::new();
                    encoder
                        .compress_vec(data)
                        .map_err(|e| LogError::Compression(e.to_string()))
                }
                #[cfg(not(feature = "snap"))]
                Err(LogError::Compression("Snappy support not compiled".to_string()))
            }
            CompressionAlgo::Zlib => {
                #[cfg(feature = "flate2")]
                {
                    use flate2::write::ZlibEncoder;
                    use flate2::Compression;
                    use std::io::Write;
                    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
                    encoder.write_all(data).map_err(|e| LogError::Io(e))?;
                    encoder.finish().map_err(|e| LogError::Io(e))
                }
                #[cfg(not(feature = "flate2"))]
                Err(LogError::Compression("Zlib support not compiled".to_string()))
            }
            CompressionAlgo::None => Ok(data.to_vec()),
        }
    }

    /// Decompresses data using the selected algorithm.
    pub fn decompress(&self, data: &[u8]) -> Result<Vec<u8>> {
        match self {
            CompressionAlgo::Lz4 => {
                #[cfg(feature = "lz4")]
                {
                    use lz4::Decoder;
                    let mut decoder = Decoder::new(data)
                        .map_err(|e| LogError::Compression(e.to_string()))?;
                    let mut decompressed = Vec::new();
                    decoder
                        .read_to_end(&mut decompressed)
                        .map_err(|e| LogError::Io(e))?;
                    Ok(decompressed)
                }
                #[cfg(not(feature = "lz4"))]
                Err(LogError::Compression("LZ4 support not compiled".to_string()))
            }
            CompressionAlgo::Snappy => {
                #[cfg(feature = "snap")]
                {
                    use snap::raw::Decoder;
                    let mut decoder = Decoder::new();
                    decoder
                        .decompress_vec(data)
                        .map_err(|e| LogError::Compression(e.to_string()))
                }
                #[cfg(not(feature = "snap"))]
                Err(LogError::Compression("Snappy support not compiled".to_string()))
            }
            CompressionAlgo::Zlib => {
                #[cfg(feature = "flate2")]
                {
                    use flate2::read::ZlibDecoder;
                    use std::io::Read;
                    let mut decoder = ZlibDecoder::new(data);
                    let mut decompressed = Vec::new();
                    decoder
                        .read_to_end(&mut decompressed)
                        .map_err(|e| LogError::Io(e))?;
                    Ok(decompressed)
                }
                #[cfg(not(feature = "flate2"))]
                Err(LogError::Compression("Zlib support not compiled".to_string()))
            }
            CompressionAlgo::None => Ok(data.to_vec()),
        }
    }
}

/// Compresses a log record (as JSON) with the given algorithm.
pub fn compress_record(
    record: &crate::log_record::LogRecord,
    algo: CompressionAlgo,
) -> Result<Vec<u8>> {
    let json = record.to_json()?;
    algo.compress(json.as_bytes())
}

/// Decompresses a log record.
pub fn decompress_record(
    compressed: &[u8],
    algo: CompressionAlgo,
) -> Result<crate::log_record::LogRecord> {
    let decompressed = algo.decompress(compressed)?;
    let json = String::from_utf8(decompressed)
        .map_err(|e| LogError::Serialization(e.to_string()))?;
    crate::log_record::LogRecord::from_json(&json)
}

/// Compresses a batch of records (as JSON array).
pub fn compress_batch(
    records: &[crate::log_record::LogRecord],
    algo: CompressionAlgo,
) -> Result<Vec<u8>> {
    let json = serde_json::to_string(records)
        .map_err(|e| LogError::Serialization(e.to_string()))?;
    algo.compress(json.as_bytes())
}

/// Decompresses a batch of records.
pub fn decompress_batch(
    compressed: &[u8],
    algo: CompressionAlgo,
) -> Result<Vec<crate::log_record::LogRecord>> {
    let decompressed = algo.decompress(compressed)?;
    let json = String::from_utf8(decompressed)
        .map_err(|e| LogError::Serialization(e.to_string()))?;
    serde_json::from_str(&json).map_err(|e| LogError::Serialization(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::log_record::LogRecord;

    #[test]
    fn test_compression_none() {
        let algo = CompressionAlgo::None;
        let data = b"hello world";
        let compressed = algo.compress(data).unwrap();
        assert_eq!(compressed, data);
        let decompressed = algo.decompress(&compressed).unwrap();
        assert_eq!(decompressed, data);
    }

    #[cfg(feature = "lz4")]
    #[test]
    fn test_compression_lz4() {
        let algo = CompressionAlgo::Lz4;
        let data = b"hello world".repeat(100);
        let compressed = algo.compress(&data).unwrap();
        assert!(compressed.len() < data.len());
        let decompressed = algo.decompress(&compressed).unwrap();
        assert_eq!(decompressed, data);
    }

    #[test]
    fn test_compress_record() {
        let record = LogRecord::new(
            crate::log_record::LogLevel::Info,
            "agent",
            "test",
            "message",
        );
        let compressed = compress_record(&record, CompressionAlgo::None).unwrap();
        let decompressed = decompress_record(&compressed, CompressionAlgo::None).unwrap();
        assert_eq!(decompressed.message, record.message);
    }
}