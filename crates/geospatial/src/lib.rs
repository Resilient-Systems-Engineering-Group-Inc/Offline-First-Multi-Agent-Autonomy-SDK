//! Geolocation and spatial planning for offline-first multi-agent systems.
//!
//! This crate provides comprehensive geospatial capabilities for agents, including:
//! - Geographic coordinate representation and validation
//! - Distance calculations (haversine, Euclidean)
//! - Spatial indexing for efficient queries
//! - Path planning algorithms
//! - Area and boundary management
//!
//! # Example
//! ```
//! use geospatial::{Coordinate, distance::haversine_distance_meters};
//!
//! let nyc = Coordinate::new(40.7128, -74.0060);
//! let la = Coordinate::new(34.0522, -118.2437);
//!
//! let distance = haversine_distance_meters(&nyc, &la).unwrap();
//! println!("Distance from NYC to LA: {}", distance);
//! ```

pub mod distance;
pub mod error;
pub mod path_planning;
pub mod spatial_index;
pub mod types;

// Re-export commonly used types
pub use error::{GeospatialError, Result};
pub use types::{AgentLocation, Area, BoundingBox, Coordinate, Distance};

/// Current version of the geospatial crate.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Initialize the geospatial system.
pub fn init() {
    // Any initialization logic would go here
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        assert!(!VERSION.is_empty());
    }

    #[test]
    fn test_coordinate_creation() {
        let coord = Coordinate::new(40.0, -80.0);
        assert!(coord.is_valid());
        assert_eq!(coord.latitude, 40.0);
        assert_eq!(coord.longitude, -80.0);
        assert!(coord.altitude.is_none());
    }

    #[test]
    fn test_distance_calculation() {
        use distance::haversine_distance_meters;
        
        let coord1 = Coordinate::new(0.0, 0.0);
        let coord2 = Coordinate::new(1.0, 1.0);
        
        let distance = haversine_distance_meters(&coord1, &coord2).unwrap();
        assert!(distance.meters > 0.0);
    }
}