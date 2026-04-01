//! Geospatial data types.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Geographic coordinate (latitude, longitude) in degrees.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Coordinate {
    /// Latitude in degrees (-90 to 90).
    pub latitude: f64,
    /// Longitude in degrees (-180 to 180).
    pub longitude: f64,
    /// Altitude in meters (optional).
    pub altitude: Option<f64>,
}

impl Coordinate {
    /// Create a new coordinate.
    pub fn new(latitude: f64, longitude: f64) -> Self {
        Self {
            latitude,
            longitude,
            altitude: None,
        }
    }

    /// Create a new coordinate with altitude.
    pub fn with_altitude(latitude: f64, longitude: f64, altitude: f64) -> Self {
        Self {
            latitude,
            longitude,
            altitude: Some(altitude),
        }
    }

    /// Check if the coordinate is valid.
    pub fn is_valid(&self) -> bool {
        (-90.0..=90.0).contains(&self.latitude) && (-180.0..=180.0).contains(&self.longitude)
    }

    /// Convert to a tuple (latitude, longitude).
    pub fn to_tuple(&self) -> (f64, f64) {
        (self.latitude, self.longitude)
    }

    /// Convert to a tuple with altitude (latitude, longitude, altitude).
    pub fn to_tuple_3d(&self) -> (f64, f64, Option<f64>) {
        (self.latitude, self.longitude, self.altitude)
    }
}

impl fmt::Display for Coordinate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(alt) = self.altitude {
            write!(f, "({}, {}, {}m)", self.latitude, self.longitude, alt)
        } else {
            write!(f, "({}, {})", self.latitude, self.longitude)
        }
    }
}

/// Bounding box defined by southwest and northeast corners.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct BoundingBox {
    /// Southwest corner (min latitude, min longitude).
    pub southwest: Coordinate,
    /// Northeast corner (max latitude, max longitude).
    pub northeast: Coordinate,
}

impl BoundingBox {
    /// Create a new bounding box.
    pub fn new(sw_lat: f64, sw_lon: f64, ne_lat: f64, ne_lon: f64) -> Self {
        Self {
            southwest: Coordinate::new(sw_lat, sw_lon),
            northeast: Coordinate::new(ne_lat, ne_lon),
        }
    }

    /// Check if the bounding box is valid.
    pub fn is_valid(&self) -> bool {
        self.southwest.is_valid()
            && self.northeast.is_valid()
            && self.southwest.latitude <= self.northeast.latitude
            && self.southwest.longitude <= self.northeast.longitude
    }

    /// Check if a coordinate is within the bounding box.
    pub fn contains(&self, coord: &Coordinate) -> bool {
        coord.latitude >= self.southwest.latitude
            && coord.latitude <= self.northeast.latitude
            && coord.longitude >= self.southwest.longitude
            && coord.longitude <= self.northeast.longitude
    }

    /// Get the center of the bounding box.
    pub fn center(&self) -> Coordinate {
        let lat = (self.southwest.latitude + self.northeast.latitude) / 2.0;
        let lon = (self.southwest.longitude + self.northeast.longitude) / 2.0;
        Coordinate::new(lat, lon)
    }

    /// Get the width (longitude range) in degrees.
    pub fn width(&self) -> f64 {
        self.northeast.longitude - self.southwest.longitude
    }

    /// Get the height (latitude range) in degrees.
    pub fn height(&self) -> f64 {
        self.northeast.latitude - self.southwest.latitude
    }
}

/// Geographic area with a boundary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Area {
    /// Area ID.
    pub id: String,
    /// Area name.
    pub name: String,
    /// Boundary coordinates (polygon).
    pub boundary: Vec<Coordinate>,
    /// Bounding box for quick checks.
    pub bbox: BoundingBox,
}

impl Area {
    /// Create a new area.
    pub fn new(id: String, name: String, boundary: Vec<Coordinate>) -> Option<Self> {
        if boundary.len() < 3 {
            return None; // Need at least 3 points for a polygon
        }

        // Calculate bounding box
        let mut min_lat = f64::MAX;
        let mut max_lat = f64::MIN;
        let mut min_lon = f64::MAX;
        let mut max_lon = f64::MIN;

        for coord in &boundary {
            min_lat = min_lat.min(coord.latitude);
            max_lat = max_lat.max(coord.latitude);
            min_lon = min_lon.min(coord.longitude);
            max_lon = max_lon.max(coord.longitude);
        }

        let bbox = BoundingBox::new(min_lat, min_lon, max_lat, max_lon);

        Some(Self {
            id,
            name,
            boundary,
            bbox,
        })
    }

    /// Check if a coordinate is within the area (simple bounding box check).
    pub fn contains(&self, coord: &Coordinate) -> bool {
        self.bbox.contains(coord)
        // Note: For precise polygon containment, we'd need a more complex algorithm
    }
}

/// Agent location information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentLocation {
    /// Agent ID.
    pub agent_id: u64,
    /// Current coordinate.
    pub coordinate: Coordinate,
    /// Timestamp of the location update.
    pub timestamp: std::time::SystemTime,
    /// Accuracy in meters (if known).
    pub accuracy: Option<f64>,
    /// Speed in meters per second (if known).
    pub speed: Option<f64>,
    /// Heading in degrees (0-360, if known).
    pub heading: Option<f64>,
}

impl AgentLocation {
    /// Create a new agent location.
    pub fn new(agent_id: u64, coordinate: Coordinate) -> Self {
        Self {
            agent_id,
            coordinate,
            timestamp: std::time::SystemTime::now(),
            accuracy: None,
            speed: None,
            heading: None,
        }
    }

    /// Create a new agent location with additional data.
    pub fn with_details(
        agent_id: u64,
        coordinate: Coordinate,
        accuracy: Option<f64>,
        speed: Option<f64>,
        heading: Option<f64>,
    ) -> Self {
        Self {
            agent_id,
            coordinate,
            timestamp: std::time::SystemTime::now(),
            accuracy,
            speed,
            heading,
        }
    }
}

/// Distance between two coordinates in meters.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Distance {
    /// Distance in meters.
    pub meters: f64,
    /// Distance in kilometers.
    pub kilometers: f64,
}

impl Distance {
    /// Create a new distance from meters.
    pub fn from_meters(meters: f64) -> Self {
        Self {
            meters,
            kilometers: meters / 1000.0,
        }
    }

    /// Create a new distance from kilometers.
    pub fn from_kilometers(kilometers: f64) -> Self {
        Self {
            meters: kilometers * 1000.0,
            kilometers,
        }
    }
}

impl fmt::Display for Distance {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.kilometers >= 1.0 {
            write!(f, "{:.2} km", self.kilometers)
        } else {
            write!(f, "{:.0} m", self.meters)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_coordinate_validation() {
        let valid = Coordinate::new(45.0, 90.0);
        assert!(valid.is_valid());

        let invalid_lat = Coordinate::new(100.0, 90.0);
        assert!(!invalid_lat.is_valid());

        let invalid_lon = Coordinate::new(45.0, 200.0);
        assert!(!invalid_lon.is_valid());
    }

    #[test]
    fn test_bounding_box() {
        let bbox = BoundingBox::new(40.0, -80.0, 45.0, -75.0);
        assert!(bbox.is_valid());

        let inside = Coordinate::new(42.5, -77.5);
        assert!(bbox.contains(&inside));

        let outside = Coordinate::new(50.0, -77.5);
        assert!(!bbox.contains(&outside));
    }

    #[test]
    fn test_area_creation() {
        let boundary = vec![
            Coordinate::new(40.0, -80.0),
            Coordinate::new(40.0, -75.0),
            Coordinate::new(45.0, -75.0),
            Coordinate::new(45.0, -80.0),
        ];

        let area = Area::new("test".to_string(), "Test Area".to_string(), boundary);
        assert!(area.is_some());

        let area = area.unwrap();
        assert_eq!(area.id, "test");
        assert_eq!(area.name, "Test Area");
        assert_eq!(area.boundary.len(), 4);
    }

    #[test]
    fn test_distance_display() {
        let dist = Distance::from_meters(1500.0);
        assert_eq!(dist.kilometers, 1.5);
        assert!(dist.to_string().contains("km"));

        let dist = Distance::from_meters(500.0);
        assert!(dist.to_string().contains("m"));
    }
}