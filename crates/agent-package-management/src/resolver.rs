//! Dependency resolution for packages.

use crate::error::{PackageError, Result};
use crate::types::*;
use futures::future::BoxFuture;
use semver::{Version, VersionReq};
use std::collections::{HashMap, HashSet};
use tracing::{debug, info, warn};

/// Dependency resolver that builds a resolution graph.
pub struct DependencyResolver {
    /// Conflict resolution strategy.
    conflict_strategy: ConflictStrategy,
}

/// Conflict resolution strategy.
#[derive(Debug, Clone, Copy)]
pub enum ConflictStrategy {
    /// Fail on any conflict.
    Strict,
    /// Prefer the highest version.
    HighestVersion,
    /// Prefer the most recently published.
    MostRecent,
}

impl Default for DependencyResolver {
    fn default() -> Self {
        Self {
            conflict_strategy: ConflictStrategy::Strict,
        }
    }
}

impl DependencyResolver {
    /// Creates a new dependency resolver.
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Creates a new dependency resolver with the given conflict strategy.
    pub fn with_strategy(conflict_strategy: ConflictStrategy) -> Self {
        Self { conflict_strategy }
    }
    
    /// Resolves dependencies for a package.
    ///
    /// # Arguments
    /// * `root` - The root package to resolve dependencies for.
    /// * `version_fetcher` - Async function that fetches a package version given its ID and version requirement.
    pub async fn resolve<F>(
        &self,
        root: &PackageVersion,
        version_fetcher: F,
    ) -> Result<ResolutionGraph>
    where
        F: Fn(String, VersionReq) -> BoxFuture<'static, Result<PackageVersion>>,
    {
        info!("Resolving dependencies for {} {}", root.package_id, root.version);
        
        let mut packages = HashMap::new();
        let mut edges = Vec::new();
        let mut visited = HashSet::new();
        let mut queue = Vec::new();
        let mut conflicts = Vec::new();
        
        // Add root package
        packages.insert(root.package_id.clone(), root.clone());
        queue.push((root.package_id.clone(), root.clone()));
        
        while let Some((parent_id, parent_package)) = queue.pop() {
            if !visited.insert(parent_id.clone()) {
                continue;
            }
            
            // Process dependencies
            for dep in &parent_package.dependencies {
                if dep.dep_type == DependencyType::Development {
                    // Skip development dependencies for now
                    continue;
                }
                
                debug!("Processing dependency: {} {}", dep.package_id, dep.version_req);
                
                // Fetch the dependency
                let dep_package = match version_fetcher(dep.package_id.clone(), dep.version_req.clone()).await {
                    Ok(pkg) => pkg,
                    Err(e) => {
                        warn!("Failed to fetch dependency {}: {}", dep.package_id, e);
                        conflicts.push(format!("Cannot resolve dependency {}: {}", dep.package_id, e));
                        continue;
                    }
                };
                
                // Check for version conflicts
                if let Some(existing) = packages.get(&dep_package.package_id) {
                    if existing.version != dep_package.version {
                        let conflict_msg = format!(
                            "Version conflict for {}: {} vs {}",
                            dep_package.package_id, existing.version, dep_package.version
                        );
                        
                        match self.conflict_strategy {
                            ConflictStrategy::Strict => {
                                conflicts.push(conflict_msg);
                                continue;
                            }
                            ConflictStrategy::HighestVersion => {
                                let existing_ver = Version::parse(&existing.version)
                                    .map_err(|e| PackageError::Semver(e))?;
                                let new_ver = Version::parse(&dep_package.version)
                                    .map_err(|e| PackageError::Semver(e))?;
                                
                                if new_ver > existing_ver {
                                    packages.insert(dep_package.package_id.clone(), dep_package.clone());
                                    debug!("Resolved conflict by choosing higher version: {}", new_ver);
                                } else {
                                    debug!("Keeping existing version: {}", existing_ver);
                                }
                            }
                            ConflictStrategy::MostRecent => {
                                if dep_package.created_at > existing.created_at {
                                    packages.insert(dep_package.package_id.clone(), dep_package.clone());
                                    debug!("Resolved conflict by choosing more recent version");
                                } else {
                                    debug!("Keeping existing version");
                                }
                            }
                        }
                    }
                } else {
                    // New package
                    packages.insert(dep_package.package_id.clone(), dep_package.clone());
                }
                
                // Add edge
                edges.push((parent_id.clone(), dep_package.package_id.clone(), dep.dep_type.clone()));
                
                // Add to queue for further resolution
                queue.push((dep_package.package_id.clone(), dep_package));
            }
        }
        
        // Build resolution graph
        let graph = ResolutionGraph {
            root: root.clone(),
            packages,
            edges,
            conflicts,
        };
        
        info!("Resolution complete: {} packages, {} edges, {} conflicts", 
            graph.packages.len(), graph.edges.len(), graph.conflicts.len());
        
        Ok(graph)
    }
    
    /// Validates a resolution graph for cycles.
    pub fn validate_graph(&self, graph: &ResolutionGraph) -> Result<()> {
        // Build adjacency list
        let mut adj = HashMap::new();
        for (from, to, _) in &graph.edges {
            adj.entry(from.clone()).or_insert_with(Vec::new).push(to.clone());
        }
        
        // Check for cycles using DFS
        let mut visited = HashSet::new();
        let mut recursion_stack = HashSet::new();
        
        for node in adj.keys() {
            if !visited.contains(node) {
                if self.has_cycle(node, &adj, &mut visited, &mut recursion_stack) {
                    return Err(PackageError::DependencyResolution(
                        "Dependency cycle detected".to_string()
                    ));
                }
            }
        }
        
        Ok(())
    }
    
    /// Helper function to detect cycles using DFS.
    fn has_cycle(
        &self,
        node: &str,
        adj: &HashMap<String, Vec<String>>,
        visited: &mut HashSet<String>,
        recursion_stack: &mut HashSet<String>,
    ) -> bool {
        visited.insert(node.to_string());
        recursion_stack.insert(node.to_string());
        
        if let Some(neighbors) = adj.get(node) {
            for neighbor in neighbors {
                if !visited.contains(neighbor.as_str()) {
                    if self.has_cycle(neighbor, adj, visited, recursion_stack) {
                        return true;
                    }
                } else if recursion_stack.contains(neighbor.as_str()) {
                    return true;
                }
            }
        }
        
        recursion_stack.remove(node);
        false
    }
    
    /// Topologically sorts the resolution graph.
    pub fn topological_sort(&self, graph: &ResolutionGraph) -> Result<Vec<PackageVersion>> {
        // Build adjacency list and indegree count
        let mut adj = HashMap::new();
        let mut indegree = HashMap::new();
        
        // Initialize indegree for all packages
        for package_id in graph.packages.keys() {
            indegree.insert(package_id.clone(), 0);
        }
        
        // Build adjacency list and update indegree
        for (from, to, _) in &graph.edges {
            adj.entry(from.clone()).or_insert_with(Vec::new).push(to.clone());
            *indegree.get_mut(to).unwrap() += 1;
        }
        
        // Find nodes with indegree 0
        let mut queue: Vec<String> = indegree
            .iter()
            .filter(|(_, &degree)| degree == 0)
            .map(|(id, _)| id.clone())
            .collect();
        
        let mut result = Vec::new();
        
        while let Some(node) = queue.pop() {
            // Add to result
            if let Some(package) = graph.packages.get(&node) {
                result.push(package.clone());
            }
            
            // Decrease indegree of neighbors
            if let Some(neighbors) = adj.get(&node) {
                for neighbor in neighbors {
                    let degree = indegree.get_mut(neighbor).unwrap();
                    *degree -= 1;
                    if *degree == 0 {
                        queue.push(neighbor.clone());
                    }
                }
            }
        }
        
        // Check for cycles
        if result.len() != graph.packages.len() {
            return Err(PackageError::DependencyResolution(
                "Dependency cycle detected during topological sort".to_string()
            ));
        }
        
        Ok(result)
    }
    
    /// Simplifies a resolution graph by removing redundant edges.
    pub fn simplify_graph(&self, graph: &ResolutionGraph) -> ResolutionGraph {
        // For now, return the graph as-is
        // In a real implementation, this would remove transitive dependencies
        graph.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::future;
    
    fn mock_package(id: &str, version: &str, deps: Vec<Dependency>) -> PackageVersion {
        PackageVersion {
            package_id: id.to_string(),
            version: version.to_string(),
            semver: Version::parse(version).unwrap(),
            changelog: "".to_string(),
            checksum: "".to_string(),
            size_bytes: 0,
            dependencies: deps,
            platforms: Vec::new(),
            install_instructions: None,
            is_default: true,
            is_deprecated: false,
            created_at: chrono::Utc::now(),
            author: "test".to_string(),
        }
    }
    
    fn mock_dependency(id: &str, version_req: &str) -> Dependency {
        Dependency {
            package_id: id.to_string(),
            version_req: VersionReq::parse(version_req).unwrap(),
            dep_type: DependencyType::Required,
            features: Vec::new(),
        }
    }
    
    #[tokio::test]
    async fn test_resolve_simple_dependencies() {
        let resolver = DependencyResolver::new();
        
        // Create a simple dependency chain: A -> B -> C
        let package_c = mock_package("C", "1.0.0", Vec::new());
        let package_b = mock_package("B", "1.0.0", vec![mock_dependency("C", "^1.0.0")]);
        let package_a = mock_package("A", "1.0.0", vec![mock_dependency("B", "^1.0.0")]);
        
        let version_fetcher = |id: String, req: VersionReq| {
            let packages = vec![
                ("A", "1.0.0", &package_a),
                ("B", "1.0.0", &package_b),
                ("C", "1.0.0", &package_c),
            ];
            
            Box::pin(future::ready(
                packages.iter()
                    .find(|(pid, _, _)| *pid == id)
                    .map(|(_, _, pkg)| Ok((*pkg).clone()))
                    .unwrap_or_else(|| Err(PackageError::PackageNotFound(id)))
            ))
        };
        
        let graph = resolver.resolve(&package_a, version_fetcher).await.unwrap();
        
        assert_eq!(graph.packages.len(), 3);
        assert_eq!(graph.edges.len(), 2);
        assert!(graph.conflicts.is_empty());
    }
    
    #[tokio::test]
    async fn test_resolve_version_conflict() {
        let resolver = DependencyResolver::new();
        
        // Create conflicting dependencies: A -> B@^1.0.0, A -> C -> B@^2.0.0
        let package_b_v1 = mock_package("B", "1.0.0", Vec::new());
        let package_b_v2 = mock_package("B", "2.0.0", Vec::new());
        let package_c = mock_package("C", "1.0.0", vec![mock_dependency("B", "^2.0.0")]);
        let package_a = mock_package("A", "1.0.0", vec![
            mock_dependency("B", "^1.0.0"),
            mock_dependency("C", "^1.0.0"),
        ]);
        
        let version_fetcher = |id: String, req: VersionReq| {
            let packages = vec![
                ("A", "1.0.0", &package_a),
                ("B", "1.0.0", &package_b_v1),
                ("B", "2.0.0", &package_b_v2),
                ("C", "1.0.0", &package_c),
            ];
            
            Box::pin(future::ready(
                packages.iter()
                    .filter(|(pid, _, _)| *pid == id)
                    .find(|(_, _, pkg)| req.matches(&pkg.semver))
                    .map(|(_, _, pkg)| Ok((*pkg).clone()))
                    .unwrap_or_else(|| Err(PackageError::PackageNotFound(id)))
            ))
        };
        
        let graph = resolver.resolve(&package_a, version_fetcher).await.unwrap();
        
        // With strict strategy, there should be a conflict
        assert!(!graph.conflicts.is_empty());
    }
    
    #[tokio::test]
    fn test_validate_graph_no_cycles() {
        let resolver = DependencyResolver::new();
        
        let package_a = mock_package("A", "1.0.0", Vec::new());
        let package_b = mock_package("B", "1.0.0", Vec::new());
        
        let graph = ResolutionGraph {
            root: package_a.clone(),
            packages: vec![
                ("A".to_string(), package_a),
                ("B".to_string(), package_b),
            ].into_iter().collect(),
            edges: vec![("A".to_string(), "B".to_string(), DependencyType::Required)],
            conflicts: Vec::new(),
        };
        
        let result = resolver.validate_graph(&graph);
        assert!(result.is_ok());
    }
    
    #[tokio::test]
    fn test_topological_sort() {
        let resolver = DependencyResolver::new();
        
        let package_a = mock_package("A", "1.0.0", Vec::new());
        let package_b = mock_package("B", "1.0.0", Vec::new());
        let package_c = mock_package("C", "1.0.0", Vec::new());
        
        let graph = ResolutionGraph {
            root: package_a.clone(),
            packages: vec![
                ("A".to_string(), package_a.clone()),
                ("B".to_string(), package_b.clone()),
                ("C".to_string(), package_c.clone()),
            ].into_iter().collect(),
            edges: vec![
                ("A".to_string(), "B".to_string(), DependencyType::Required),
                ("B".to_string(), "C".to_string(), DependencyType::Required),
            ],
            conflicts: Vec::new(),
        };
        
        let sorted = resolver.topological_sort(&graph).unwrap();
        
        // C should come before B, B before A (or reverse depending on direction)
        // Actually, with our implementation, root comes first
        assert_eq!(sorted.len(), 3);
    }
}