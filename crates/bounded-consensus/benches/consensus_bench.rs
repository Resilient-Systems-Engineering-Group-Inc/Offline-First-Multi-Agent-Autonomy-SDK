use criterion::{black_box, criterion_group, criterion_main, Criterion};
use bounded_consensus::{TwoPhaseBoundedConsensus, BoundedConsensusConfig, Proposal};
use common::types::AgentId;
use std::collections::HashSet;
use tokio::runtime::Runtime;

fn bench_consensus_propose(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let config = BoundedConsensusConfig {
        local_agent_id: AgentId(1),
        participants: HashSet::from([AgentId(1), AgentId(2)]),
        max_rounds: 3,
        round_duration_ms: 100,
    };
    let mut consensus = TwoPhaseBoundedConsensus::<String>::new(config);

    let proposal = Proposal {
        id: 1,
        value: "test value".to_string(),
        proposer: AgentId(1),
    };

    c.bench_function("consensus_propose", |b| {
        b.iter(|| {
            let _rx = rt.block_on(async {
                consensus.propose(proposal.clone()).await.unwrap()
            });
            black_box(());
        })
    });
}

fn bench_consensus_handle_message(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let config = BoundedConsensusConfig {
        local_agent_id: AgentId(1),
        participants: HashSet::from([AgentId(1), AgentId(2)]),
        max_rounds: 3,
        round_duration_ms: 100,
    };
    let mut consensus = TwoPhaseBoundedConsensus::<String>::new(config);

    // Prepare a vote message payload
    use bounded_consensus::TwoPhaseMessage;
    use bincode;
    let vote_msg = TwoPhaseMessage::Vote {
        proposal_id: 1,
        vote: true,
    };
    let payload = bincode::serialize(&vote_msg).unwrap();

    c.bench_function("consensus_handle_message", |b| {
        b.iter(|| {
            rt.block_on(async {
                consensus.handle_message(AgentId(2), payload.clone()).await.unwrap();
            });
        })
    });
}

criterion_group!(
    benches,
    bench_consensus_propose,
    bench_consensus_handle_message,
);
criterion_main!(benches);