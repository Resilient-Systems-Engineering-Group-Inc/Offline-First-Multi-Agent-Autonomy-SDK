use criterion::{black_box, criterion_group, criterion_main, Criterion};
use mesh_transport::{MeshTransport, MeshTransportConfig};
use common::types::AgentId;
use tokio::runtime::Runtime;

fn bench_transport_create(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    c.bench_function("mesh_transport_create", |b| {
        b.iter(|| {
            let config = MeshTransportConfig {
                local_agent_id: AgentId(1),
                static_peers: vec![],
                use_mdns: false,
                listen_addr: "/ip4/127.0.0.1/tcp/0".to_string(),
                use_in_memory: true,
            };
            let _transport = rt.block_on(async {
                MeshTransport::new(config).await.unwrap()
            });
            black_box(());
        })
    });
}

fn bench_transport_broadcast(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let config = MeshTransportConfig {
        local_agent_id: AgentId(1),
        static_peers: vec![],
        use_mdns: false,
        listen_addr: "/ip4/127.0.0.1/tcp/0".to_string(),
        use_in_memory: true,
    };
    let mut transport = rt.block_on(async {
        MeshTransport::new(config).await.unwrap()
    });
    rt.block_on(async {
        transport.start().await.unwrap();
    });

    let payload = vec![0u8; 1024]; // 1 KB payload
    c.bench_function("mesh_transport_broadcast", |b| {
        b.iter(|| {
            rt.block_on(async {
                transport.broadcast(payload.clone()).await.unwrap();
            });
        })
    });
}

fn bench_transport_send_to(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let config = MeshTransportConfig {
        local_agent_id: AgentId(1),
        static_peers: vec![],
        use_mdns: false,
        listen_addr: "/ip4/127.0.0.1/tcp/0".to_string(),
        use_in_memory: true,
    };
    let mut transport = rt.block_on(async {
        MeshTransport::new(config).await.unwrap()
    });
    rt.block_on(async {
        transport.start().await.unwrap();
    });

    let payload = vec![0u8; 1024];
    c.bench_function("mesh_transport_send_to", |b| {
        b.iter(|| {
            // Send to a non‑existent peer (will fail, but we just measure overhead)
            rt.block_on(async {
                let _ = transport.send_to(AgentId(2), payload.clone()).await;
            });
        })
    });
}

criterion_group!(
    benches,
    bench_transport_create,
    bench_transport_broadcast,
    bench_transport_send_to,
);
criterion_main!(benches);