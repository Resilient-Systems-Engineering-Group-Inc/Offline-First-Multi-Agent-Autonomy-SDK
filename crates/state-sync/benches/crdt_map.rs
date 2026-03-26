use criterion::{black_box, criterion_group, criterion_main, Criterion};
use state_sync::crdt_map::CrdtMap;
use common::types::AgentId;
use serde_json::json;

fn bench_set(c: &mut Criterion) {
    c.bench_function("crdt_map_set", |b| {
        b.iter(|| {
            let mut map = CrdtMap::new();
            for i in 0..100 {
                map.set(&format!("key{}", i), json!(i), AgentId(1));
            }
            black_box(&map);
        })
    });
}

fn bench_get(c: &mut Criterion) {
    let mut map = CrdtMap::new();
    for i in 0..100 {
        map.set(&format!("key{}", i), json!(i), AgentId(1));
    }
    c.bench_function("crdt_map_get", |b| {
        b.iter(|| {
            for i in 0..100 {
                let _: Option<serde_json::Value> = map.get(&format!("key{}", i));
            }
        })
    });
}

fn bench_merge(c: &mut Criterion) {
    let mut map1 = CrdtMap::new();
    let mut map2 = CrdtMap::new();
    for i in 0..50 {
        map1.set(&format!("key{}", i), json!(i), AgentId(1));
    }
    for i in 50..100 {
        map2.set(&format!("key{}", i), json!(i), AgentId(2));
    }
    c.bench_function("crdt_map_merge", |b| {
        b.iter(|| {
            let mut map = map1.clone();
            map.merge(&map2);
            black_box(&map);
        })
    });
}

fn bench_delta_generation(c: &mut Criterion) {
    let mut map = CrdtMap::new();
    for i in 0..100 {
        map.set(&format!("key{}", i), json!(i), AgentId(1));
    }
    let vclock = map.vclock.clone();
    c.bench_function("crdt_map_delta_since", |b| {
        b.iter(|| {
            let _delta = map.delta_since(&vclock);
        })
    });
}

criterion_group!(
    benches,
    bench_set,
    bench_get,
    bench_merge,
    bench_delta_generation
);
criterion_main!(benches);