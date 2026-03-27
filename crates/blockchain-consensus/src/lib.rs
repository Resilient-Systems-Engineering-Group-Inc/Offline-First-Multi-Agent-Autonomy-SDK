//! Proof‑of‑stake blockchain consensus for multi‑agent systems.
//!
//! This crate provides a Byzantine‑fault‑tolerant consensus algorithm
//! where validators are selected based on their stake (proof‑of‑stake).
//! It can be used to reach agreement on a sequence of transactions
//! (e.g., task assignments, sensor readings, configuration changes)
//! across an offline‑first mesh network.

pub mod block;
pub mod chain;
pub mod validator;
pub mod pos;
pub mod error;

pub use block::{Block, Transaction};
pub use chain::Blockchain;
pub use validator::Validator;
pub use pos::{StakeManager, ProofOfStake};
pub use error::{Error, Result};

/// Pre‑import of commonly used types.
pub mod prelude {
    pub use crate::{Blockchain, Validator, ProofOfStake, Transaction};
}