//! Centralized configuration management for the Offline‑First Multi‑Agent Autonomy SDK.
//!
//! This crate provides a unified way to load, validate, watch, and hot‑reload
//! configuration files across all components of the system.

#![deny(missing_docs, unsafe_code)]

pub mod error;
pub mod loader;
pub mod manager;
pub mod schema;
pub mod validator;
pub mod watch;
pub mod versioning;

pub use error::Error;
pub use loader::{FileFormat, Loader};
pub use manager::ConfigurationManager;
pub use schema::Configuration;
pub use validator::Validator;
pub use versioning::{
    ConfigurationDiff, ConfigurationVersion, ConfigurationVersionManager,
    DiffSummary, FileVersionStorage, InMemoryVersionStorage, VersionStorage,
};