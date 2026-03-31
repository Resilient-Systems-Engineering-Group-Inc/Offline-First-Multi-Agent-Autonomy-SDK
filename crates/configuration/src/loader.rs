//! Configuration loading from various file formats.

use crate::error::Error;
use serde::de::DeserializeOwned;
use std::path::Path;

/// Supported configuration file formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileFormat {
    /// YAML (.yaml, .yml)
    Yaml,
    /// JSON (.json)
    Json,
    /// TOML (.toml)
    Toml,
}

impl FileFormat {
    /// Detect format from file extension.
    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext.to_lowercase().as_str() {
            "yaml" | "yml" => Some(Self::Yaml),
            "json" => Some(Self::Json),
            "toml" => Some(Self::Toml),
            _ => None,
        }
    }

    /// Detect format from file path.
    pub fn from_path<P: AsRef<Path>>(path: P) -> Option<Self> {
        path.as_ref()
            .extension()
            .and_then(|ext| ext.to_str())
            .and_then(Self::from_extension)
    }
}

/// Loads configuration from files.
#[derive(Debug, Default)]
pub struct Loader;

impl Loader {
    /// Create a new loader.
    pub fn new() -> Self {
        Self
    }

    /// Load configuration from a file, auto‑detecting format.
    pub fn load<T, P>(&self, path: P) -> Result<T, Error>
    where
        T: DeserializeOwned,
        P: AsRef<Path>,
    {
        let path = path.as_ref();
        let format = FileFormat::from_path(path)
            .ok_or_else(|| Error::Parse(format!("Unsupported file extension: {:?}", path)))?;

        let content = std::fs::read_to_string(path)
            .map_err(|e| Error::Io(e).context(format!("Failed to read {:?}", path)))?;

        self.load_from_str(&content, format)
    }

    /// Load configuration from a string with explicit format.
    pub fn load_from_str<T>(&self, content: &str, format: FileFormat) -> Result<T, Error>
    where
        T: DeserializeOwned,
    {
        match format {
            FileFormat::Yaml => serde_yaml::from_str(content)
                .map_err(|e| Error::Parse(format!("YAML parse error: {}", e))),
            FileFormat::Json => serde_json::from_str(content)
                .map_err(|e| Error::Parse(format!("JSON parse error: {}", e))),
            FileFormat::Toml => toml::from_str(content)
                .map_err(|e| Error::Parse(format!("TOML parse error: {}", e))),
        }
    }
}

// Helper trait for adding context to errors.
trait Context<T> {
    fn context(self, msg: String) -> Self;
}

impl<T> Context<T> for Result<T, Error> {
    fn context(self, msg: String) -> Self {
        self.map_err(|e| match e {
            Error::Io(io) => Error::Io(io),
            Error::Parse(s) => Error::Parse(format!("{}: {}", msg, s)),
            Error::Validation(s) => Error::Validation(format!("{}: {}", msg, s)),
            Error::NotFound(s) => Error::NotFound(format!("{}: {}", msg, s)),
            Error::Watch(s) => Error::Watch(format!("{}: {}", msg, s)),
            Error::Internal(e) => Error::Internal(e),
        })
    }
}