//! Codec for streaming messages.

use crate::error::Error;
use bytes::{Bytes, BytesMut};
use serde_cbor;
use tokio_util::codec::{Decoder, Encoder};

/// CBOR codec for StreamMessage.
pub struct CborCodec;

impl Encoder<crate::channel::StreamMessage> for CborCodec {
    type Error = Error;

    fn encode(
        &mut self,
        item: crate::channel::StreamMessage,
        dst: &mut BytesMut,
    ) -> Result<(), Self::Error> {
        let bytes = serde_cbor::to_vec(&item).map_err(|e| Error::Codec(e.to_string()))?;
        dst.extend_from_slice(&bytes);
        Ok(())
    }
}

impl Decoder for CborCodec {
    type Item = crate::channel::StreamMessage;
    type Error = Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        match serde_cbor::from_slice::<Self::Item>(src) {
            Ok(msg) => {
                src.clear();
                Ok(Some(msg))
            }
            Err(serde_cbor::Error::Eof) => Ok(None),
            Err(e) => Err(Error::Codec(e.to_string())),
        }
    }
}

/// Compression trait.
#[cfg(feature = "compression")]
pub trait Compressor {
    fn compress(&self, data: Bytes) -> Result<Bytes, Error>;
    fn decompress(&self, data: Bytes) -> Result<Bytes, Error>;
}

/// LZ4 compressor.
#[cfg(feature = "compression")]
pub struct Lz4Compressor;

#[cfg(feature = "compression")]
impl Compressor for Lz4Compressor {
    fn compress(&self, data: Bytes) -> Result<Bytes, Error> {
        use lz4::EncoderBuilder;
        use std::io::Write;

        let mut encoder = EncoderBuilder::new()
            .level(4)
            .build(Vec::new())
            .map_err(|e| Error::Codec(format!("LZ4 encoder error: {}", e)))?;
        encoder
            .write_all(&data)
            .map_err(|e| Error::Codec(format!("LZ4 write error: {}", e)))?;
        let (compressed, result) = encoder.finish();
        result.map_err(|e| Error::Codec(format!("LZ4 finish error: {}", e)))?;
        Ok(Bytes::from(compressed))
    }

    fn decompress(&self, data: Bytes) -> Result<Bytes, Error> {
        use lz4::Decoder;
        use std::io::Read;

        let mut decoder = Decoder::new(&data[..])
            .map_err(|e| Error::Codec(format!("LZ4 decoder error: {}", e)))?;
        let mut decompressed = Vec::new();
        decoder
            .read_to_end(&mut decompressed)
            .map_err(|e| Error::Codec(format!("LZ4 read error: {}", e)))?;
        Ok(Bytes::from(decompressed))
    }
}