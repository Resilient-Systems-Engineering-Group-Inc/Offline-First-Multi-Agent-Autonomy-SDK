//! Error types for the Kubernetes operator.

use thiserror::Error;

/// Top‑level error for the operator.
#[derive(Error, Debug)]
pub enum Error {
    /// Kubernetes API error.
    #[error("Kubernetes error: {0}")]
    KubeError(#[from] kube::Error),

    /// Invalid custom resource specification.
    #[error("Invalid CRD spec: {0}")]
    InvalidSpec(String),

    /// Reconciliation failed.
    #[error("Reconciliation failed: {0}")]
    ReconciliationFailed(String),

    /// Internal error.
    #[error("Internal error: {0}")]
    Internal(#[from] anyhow::Error),
}