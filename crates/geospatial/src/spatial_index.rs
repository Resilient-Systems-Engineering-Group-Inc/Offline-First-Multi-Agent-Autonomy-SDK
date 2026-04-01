//! Spatial indexing for efficient location-based queries.

use crate::error::{GeospatialError, Result};
use crate::types::{AgentLocation, BoundingBox, Coordinate};
use kdtree::distance::squared_euclidean;
use kdtree::KdTree;
use rstar::{RTree, RTreeObject, AABB};
use std::collections::HashMap;
use std::sync::Arc;

/// Wrapper for AgentLocation that implements RTreeObject.
#[derive(Debug, Clone)]
pub struct SpatialAgent {
    pub location: Arc<AgentLocation>,
}

impl RTreeObject for SpatialAgent {
    type Envelope = AABB<[f64; 2]>;

    fn envelope(&self) -> Self::Envelope {
        let point = [self.location.coordinate.longitude, self.location.coordinate.latitude];
        AABB::from_point(point)
    }
}

/// Spatial index for efficient agent location queries.
pub struct SpatialIndex {
    /// R-tree for 2D spatial queries.
    rtree: RTree<SpatialAgent>,
    /// K-d tree for nearest neighbor queries.
    kdtree: KdTree<f64, u64, [f64; 2]>,
    /// Map from agent ID to location.
    locations: HashMap<u64, Arc<AgentLocation>>,
}

impl SpatialIndex {
    /// Create a new empty spatial index.
    pub fn new() -> Self {
        Self {
            rtree: RTree::new(),
            kdtree: KdTree::new(2), // 2 dimensions (longitude, latitude)
            locations: HashMap::new(),
        }
    }

    /// Insert or update an agent's location.
    pub fn update_location(&mut self, location: AgentLocation) -> Result<()> {
        if !location.coordinate.is_valid() {
            return Err(GeospatialError::InvalidCoordinates(format!(
                "Invalid coordinate for agent {}: {}",
                location.agent_id, location.coordinate
            )));
        }

        let agent_id = location.agent_id;
        let location_arc = Arc::new(location);

        // Remove old location if it exists
        self.remove_location(agent_id);

        // Add to R-tree
        let spatial_agent = SpatialAgent {
            location: location_arc.clone(),
        };
        self.rtree.insert(spatial_agent);

        // Add to K-d tree
        let point = [
            location_arc.coordinate.longitude,
            location_arc.coordinate.latitude,
        ];
        self.kdtree.add(point, agent_id).map_err(|e| {
            GeospatialError::SpatialIndexError(format!("Failed to add to K-d tree: {}", e))
        })?;

        // Store in map
        self.locations.insert(agent_id, location_arc);

        Ok(())
    }

    /// Remove an agent's location from the index.
    pub fn remove_location(&mut self, agent_id: u64) {
        if let Some(location) = self.locations.remove(&agent_id) {
            // Remove from R-tree (inefficient but works for our scale)
            // In a production system, we'd need to track the R-tree objects
            self.rtree = self
                .rtree
                .iter()
                .filter(|agent| agent.location.agent_id != agent_id)
                .cloned()
                .collect();

            // Remove from K-d tree (rebuild for simplicity)
            self.rebuild_kdtree();
        }
    }

    /// Rebuild the K-d tree from current locations.
    fn rebuild_kdtree(&mut self) {
        self.kdtree = KdTree::new(2);
        for (agent_id, location) in &self.locations {
            let point = [location.coordinate.longitude, location.coordinate.latitude];
            let _ = self.kdtree.add(point, *agent_id);
        }
    }

    /// Get an agent's location.
    pub fn get_location(&self, agent_id: u64) -> Option<Arc<AgentLocation>> {
        self.locations.get(&agent_id).cloned()
    }

    /// Find agents within a bounding box.
    pub fn within_bbox(&self, bbox: &BoundingBox) -> Vec<Arc<AgentLocation>> {
        if !bbox.is_valid() {
            return Vec::new();
        }

        let envelope = AABB::from_corners(
            [bbox.southwest.longitude, bbox.southwest.latitude],
            [bbox.northeast.longitude, bbox.northeast.latitude],
        );

        self.rtree
            .locate_in_envelope(&envelope)
            .map(|agent| agent.location.clone())
            .collect()
    }

    /// Find agents within a certain distance of a point.
    pub fn within_distance(
        &self,
        center: &Coordinate,
        max_distance_meters: f64,
    ) -> Result<Vec<Arc<AgentLocation>>> {
        if !center.is_valid() {
            return Err(GeospatialError::InvalidCoordinates(
                "Invalid center coordinate".to_string(),
            ));
        }

        // Convert max distance to degrees (approximate)
        // 1 degree of latitude ≈ 111 km, 1 degree of longitude varies with latitude
        const METERS_PER_DEGREE_LAT: f64 = 111_000.0;
        let meters_per_degree_lon = METERS_PER_DEGREE_LAT * center.latitude.to_radians().cos();

        let max_deg_lat = max_distance_meters / METERS_PER_DEGREE_LAT;
        let max_deg_lon = max_distance_meters / meters_per_degree_lon.abs().max(1.0);

        // Create bounding box for initial filtering
        let bbox = BoundingBox::new(
            center.latitude - max_deg_lat,
            center.longitude - max_deg_lon,
            center.latitude + max_deg_lat,
            center.longitude + max_deg_lon,
        );

        let candidates = self.within_bbox(&bbox);

        // Filter by actual distance
        use crate::distance::haversine_distance_meters;
        
        let mut results = Vec::new();
        for location in candidates {
            match haversine_distance_meters(center, &location.coordinate) {
                Ok(distance) => {
                    if distance.meters <= max_distance_meters {
                        results.push(location);
                    }
                }
                Err(_) => continue,
            }
        }

        Ok(results)
    }

    /// Find the nearest agents to a point.
    pub fn nearest_neighbors(
        &self,
        center: &Coordinate,
        count: usize,
    ) -> Result<Vec<(Arc<AgentLocation>, f64)>> {
        if !center.is_valid() {
            return Err(GeospatialError::InvalidCoordinates(
                "Invalid center coordinate".to_string(),
            ));
        }

        let point = [center.longitude, center.latitude];
        
        // Use K-d tree for nearest neighbor search
        let nearest = self.kdtree.nearest(&point, count, &squared_euclidean).map_err(|e| {
            GeospatialError::SpatialIndexError(format!("K-d tree search failed: {}", e))
        })?;

        let mut results = Vec::new();
        for (dist_squared, &agent_id) in nearest {
            if let Some(location) = self.get_location(agent_id) {
                // Convert squared Euclidean distance (in degree units) to approximate meters
                let dist_deg = dist_squared.sqrt();
                const METERS_PER_DEGREE: f64 = 111_000.0; // Approximate
                let dist_meters = dist_deg * METERS_PER_DEGREE;
                
                results.push((location, dist_meters));
            }
        }

        Ok(results)
    }

    /// Get all agent locations.
    pub fn all_locations(&self) -> Vec<Arc<AgentLocation>> {
        self.locations.values().cloned().collect()
    }

    /// Get the number of agents in the index.
    pub fn len(&self) -> usize {
        self.locations.len()
    }

    /// Check if the index is empty.
    pub fn is_empty(&self) -> bool {
        self.locations.is_empty()
    }

    /// Clear all locations from the index.
    pub fn clear(&mut self) {
        self.rtree = RTree::new();
        self.kdtree = KdTree::new(2);
        self.locations.clear();
    }
}

impl Default for SpatialIndex {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::SystemTime;

    #[test]
    fn test_spatial_index_basic() {
        let mut index = SpatialIndex::new();
        assert!(index.is_empty());

        let location = AgentLocation::new(1, Coordinate::new(40.0, -80.0));
        index.update_location(location).unwrap();
        
        assert_eq!(index.len(), 1);
        assert!(!index.is_empty());

        let retrieved = index.get_location(1);
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().agent_id, 1);
    }

    #[test]
    fn test_within_bbox() {
        let mut index = SpatialIndex::new();

        // Add some locations
        index
            .update_location(AgentLocation::new(1, Coordinate::new(40.0, -80.0)))
            .unwrap();
        index
            .update_location(AgentLocation::new(2, Coordinate::new(41.0, -79.0)))
            .unwrap();
        index
            .update_location(AgentLocation::new(3, Coordinate::new(42.0, -78.0)))
            .unwrap();

        // Bounding box that should contain only the first two
        let bbox = BoundingBox::new(39.5, -80.5, 41.5, -78.5);
        let within = index.within_bbox(&bbox);

        assert_eq!(within.len(), 2);
        let agent_ids: Vec<u64> = within.iter().map(|loc| loc.agent_id).collect();
        assert!(agent_ids.contains(&1));
        assert!(agent_ids.contains(&2));
        assert!(!agent_ids.contains(&3));
    }

    #[test]
    fn test_remove_location() {
        let mut index = SpatialIndex::new();

        index
            .update_location(AgentLocation::new(1, Coordinate::new(40.0, -80.0)))
            .unwrap();
        assert_eq!(index.len(), 1);

        index.remove_location(1);
        assert_eq!(index.len(), 0);
        assert!(index.get_location(1).is_none());
    }
}