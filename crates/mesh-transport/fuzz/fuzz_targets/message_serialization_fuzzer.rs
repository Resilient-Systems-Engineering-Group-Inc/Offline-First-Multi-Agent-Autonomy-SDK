//! Fuzz test for message serialization in mesh transport.

#![no_main]

use libfuzzer_sys::fuzz_target;
use mesh_transport::message::{Message, MessageType};
use mesh_transport::connection::ConnectionId;
use serde::{Serialize, Deserialize};
use bytes::Bytes;

#[derive(Serialize, Deserialize, Clone, Debug, arbitrary::Arbitrary)]
struct FuzzMessage {
    msg_type: u8,
    payload: Vec<u8>,
    sender_id: String,
    destination_id: Option<String>,
    timestamp: u64,
    priority: u8,
}

fuzz_target!(|fuzz_msg: FuzzMessage| {
    // Convert fuzzed data to Message
    let msg_type = match fuzz_msg.msg_type % 5 {
        0 => MessageType::Data,
        1 => MessageType::Control,
        2 => MessageType::Heartbeat,
        3 => MessageType::Ack,
        _ => MessageType::Error,
    };
    
    let message = Message {
        msg_type,
        payload: Bytes::from(fuzz_msg.payload),
        sender_id: fuzz_msg.sender_id.clone(),
        destination_id: fuzz_msg.destination_id.clone(),
        message_id: format!("{}-{}", fuzz_msg.sender_id, fuzz_msg.timestamp),
        timestamp: fuzz_msg.timestamp,
        priority: fuzz_msg.priority,
        metadata: std::collections::HashMap::new(),
    };
    
    // Test serialization round-trip
    let serialized = bincode::serialize(&message).expect("Serialization failed");
    let deserialized: Message = bincode::deserialize(&serialized)
        .expect("Deserialization failed");
    
    // Verify fields match
    assert_eq!(
        message.msg_type,
        deserialized.msg_type,
        "Message type mismatch"
    );
    assert_eq!(
        message.sender_id,
        deserialized.sender_id,
        "Sender ID mismatch"
    );
    assert_eq!(
        message.timestamp,
        deserialized.timestamp,
        "Timestamp mismatch"
    );
    assert_eq!(
        message.priority,
        deserialized.priority,
        "Priority mismatch"
    );
    
    // Payload comparison
    assert_eq!(
        message.payload.len(),
        deserialized.payload.len(),
        "Payload length mismatch"
    );
    
    // Test message size constraints
    let max_size = 1024 * 1024; // 1MB
    assert!(
        serialized.len() < max_size,
        "Serialized message too large: {} bytes",
        serialized.len()
    );
    
    // Test with different compression levels (if enabled)
    #[cfg(feature = "compression")]
    {
        use flate2::Compression;
        use flate2::write::GzEncoder;
        use std::io::Write;
        
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(&serialized).unwrap();
        let compressed = encoder.finish().unwrap();
        
        // Compression should not increase size for random data significantly
        if serialized.len() > 100 {
            assert!(
                compressed.len() < serialized.len() * 2,
                "Compression overhead too high"
            );
        }
    }
});
