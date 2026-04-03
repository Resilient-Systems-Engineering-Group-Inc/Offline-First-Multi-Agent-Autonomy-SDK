//! Core types for agent package management.

use chrono::{DateTime, Utc};
use semver::{Version, VersionReq};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

/// Unique identifier for a package.
pub type PackageId = String;

/// Unique identifier for a package version.
pub type VersionId = String;

/// Package metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageMetadata {
    /// Unique package identifier.
    pub id: PackageId,
    /// Human-readable package name.
    pub name: String,
    /// Package description.
    pub description: String,
    /// Package type (e.g., "agent", "capability", "library", "plugin").
    pub package_type: PackageType,
    /// Author/owner.
    pub author: String,
    /// License.
    pub license: String,
    /// Repository URL.
    pub repository: Option<String>,
    /// Tags for categorization.
    pub tags: Vec<String>,
    /// Custom metadata.
    pub custom_metadata: HashMap<String, serde_json::Value>,
    /// Creation timestamp.
    pub created_at: DateTime<Utc>,
    /// Last update timestamp.
    pub updated_at: DateTime<Utc>,
}

/// Package type.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PackageType {
    /// Agent package (full agent implementation).
    Agent,
    /// Capability package (adds specific capability to agent).
    Capability,
    /// Library package (shared code).
    Library,
    /// Plugin package (extends existing functionality).
    Plugin,
    /// Tool package (development or management tools).
    Tool,
}

/// Package version metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageVersion {
    /// Package identifier.
    pub package_id: PackageId,
    /// Version identifier.
    pub version: VersionId,
    /// Semantic version.
    pub semver: Version,
    /// Description of changes.
    pub changelog: String,
    /// SHA-256 checksum of the package archive.
    pub checksum: String,
    /// Size in bytes.
    pub size_bytes: u64,
    /// Dependencies.
    pub dependencies: Vec<Dependency>,
    /// Supported platforms.
    pub platforms: Vec<Platform>,
    /// Installation instructions.
    pub install_instructions: Option<String>,
    /// Whether this version is the default/latest.
    pub is_default: bool,
    /// Whether this version is deprecated.
    pub is_deprecated: bool,
    /// Creation timestamp.
    pub created_at: DateTime<Utc>,
    /// Author/uploader.
    pub author: String,
}

/// Dependency specification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dependency {
    /// Package identifier.
    pub package_id: PackageId,
    /// Version requirement (semver range).
    pub version_req: VersionReq,
    /// Dependency type.
    pub dep_type: DependencyType,
    /// Optional features to enable.
    pub features: Vec<String>,
}

/// Dependency type.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DependencyType {
    /// Required dependency.
    Required,
    /// Optional dependency.
    Optional,
    /// Development dependency.
    Development,
    /// Peer dependency.
    Peer,
}

/// Platform specification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Platform {
    /// Operating system.
    pub os: String,
    /// Architecture.
    pub arch: String,
    /// Additional constraints.
    pub constraints: HashMap<String, String>,
}

/// Package archive format.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ArchiveFormat {
    /// Tar.gz format.
    TarGz,
    /// Zip format.
    Zip,
    /// Raw directory.
    Directory,
}

/// Package installation request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallRequest {
    /// Package identifier.
    pub package_id: PackageId,
    /// Version requirement.
    pub version_req: VersionReq,
    /// Target installation path.
    pub target_path: String,
    /// Whether to install dependencies.
    pub install_deps: bool,
    /// Features to enable.
    pub features: Vec<String>,
    /// Platform constraints.
    pub platform_constraints: Vec<Platform>,
}

/// Package installation result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallResult {
    /// Installed package.
    pub package: PackageVersion,
    /// Installation path.
    pub install_path: String,
    /// Installed dependencies.
    pub installed_deps: Vec<PackageVersion>,
    /// Total size installed.
    pub total_size_bytes: u64,
    /// Installation timestamp.
    pub installed_at: DateTime<Utc>,
}

/// Package query filters.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageQuery {
    /// Filter by package type.
    pub package_type: Option<PackageType>,
    /// Filter by tags.
    pub tags: Vec<String>,
    /// Filter by author.
    pub author: Option<String>,
    /// Search term.
    pub search: Option<String>,
    /// Limit results.
    pub limit: Option<usize>,
    /// Offset for pagination.
    pub offset: Option<usize>,
}

/// Package registry configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryConfig {
    /// Registry URL.
    pub url: String,
    /// Authentication token.
    pub auth_token: Option<String>,
    /// Timeout in seconds.
    pub timeout_secs: u64,
    /// Whether to verify SSL.
    pub verify_ssl: bool,
}

/// Local package cache entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry {
    /// Package version.
    pub package_version: PackageVersion,
    /// Cache path.
    pub cache_path: String,
    /// Cached at timestamp.
    pub cached_at: DateTime<Utc>,
    /// Last accessed timestamp.
    pub last_accessed: DateTime<Utc>,
    /// Access count.
    pub access_count: u64,
}

/// Package resolution graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolutionGraph {
    /// Root package.
    pub root: PackageVersion,
    /// All resolved packages.
    pub packages: HashMap<PackageId, PackageVersion>,
    /// Dependency edges.
    pub edges: Vec<(PackageId, PackageId, DependencyType)>,
    /// Conflicts (if any).
    pub conflicts: Vec<String>,
}

/// Package manager statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageStats {
    /// Total packages in registry.
    pub total_packages: usize,
    /// Total versions.
    pub total_versions: usize,
    /// Total cache size in bytes.
    pub cache_size_bytes: u64,
    /// Packages by type.
    pub packages_by_type: HashMap<String, usize>,
    /// Most popular packages.
    pub popular_packages: Vec<(PackageId, u64)>,
    /// Last update timestamp.
    pub last_updated: DateTime<Utc>,
}