//! Distance calculations between geographic coordinates.

use crate::error::{GeospatialError, Result};
use crate::types::{Coordinate, Distance};
use haversine::haversine_distance;
use std::f64::consts::PI;

/// Calculate the great-circle distance between two coordinates using the haversine formula.
pub fn haversine_distance_meters(coord1: &Coordinate, coord2: &Coordinate) -> Result<Distance> {
    if !coord1.is_valid() {
        return Err(GeospatialError::InvalidCoordinates(format!(
            "Coordinate 1 is invalid: {}",
            coord1
        )));
    }
    if !coord2.is_valid() {
        return Err(GeospatialError::InvalidCoordinates(format!(
            "Coordinate 2 is invalid: {}",
            coord2
        )));
    }

    let distance_meters = haversine_distance(
        (coord1.latitude, coord1.longitude),
        (coord2.latitude, coord2.longitude),
    );

    Ok(Distance::from_meters(distance_meters))
}

/// Calculate the Euclidean distance between two coordinates (approximate for small distances).
pub fn euclidean_distance_meters(coord1: &Coordinate, coord2: &Coordinate) -> Result<Distance> {
    if !coord1.is_valid() {
        return Err(GeospatialError::InvalidCoordinates(format!(
            "Coordinate 1 is invalid: {}",
            coord1
        )));
    }
    if !coord2.is_valid() {
        return Err(GeospatialError::InvalidCoordinates(format!(
            "Coordinate 2 is invalid: {}",
            coord2
        )));
    }

    // Convert degrees to meters (approximate)
    let lat1_rad = coord1.latitude.to_radians();
    let lat2_rad = coord2.latitude.to_radians();
    
    // Earth radius in meters
    const R: f64 = 6_371_000.0;
    
    let dlat = (coord2.latitude - coord1.latitude).to_radians();
    let dlon = (coord2.longitude - coord1.longitude).to_radians();
    
    // Haversine formula (same as above, but we'll use our own implementation for comparison)
    let a = (dlat / 2.0).sin().powi(2)
        + lat1_rad.cos() * lat2_rad.cos() * (dlon / 2.0).sin().powi(2);
    let c = 2.0 * a.sqrt().atan2((1.0 - a).sqrt());
    let distance_meters = R * c;

    Ok(Distance::from_meters(distance_meters))
}

/// Calculate the initial bearing from coord1 to coord2.
pub fn initial_bearing(coord1: &Coordinate, coord2: &Coordinate) -> Result<f64> {
    if !coord1.is_valid() || !coord2.is_valid() {
        return Err(GeospatialError::InvalidCoordinates(
            "Invalid coordinates for bearing calculation".to_string(),
        ));
    }

    let lat1_rad = coord1.latitude.to_radians();
    let lat2_rad = coord2.latitude.to_radians();
    let dlon_rad = (coord2.longitude - coord1.longitude).to_radians();

    let y = dlon_rad.sin() * lat2_rad.cos();
    let x = lat1_rad.cos() * lat2_rad.sin() - lat1_rad.sin() * lat2_rad.cos() * dlon_rad.cos();
    
    let bearing_rad = y.atan2(x);
    let bearing_deg = (bearing_rad.to_degrees() + 360.0) % 360.0;

    Ok(bearing_deg)
}

/// Calculate the midpoint between two coordinates.
pub fn midpoint(coord1: &Coordinate, coord2: &Coordinate) -> Result<Coordinate> {
    if !coord1.is_valid() || !coord2.is_valid() {
        return Err(GeospatialError::InvalidCoordinates(
            "Invalid coordinates for midpoint calculation".to_string(),
        ));
    }

    let lat1_rad = coord1.latitude.to_radians();
    let lat2_rad = coord2.latitude.to_radians();
    let lon1_rad = coord1.longitude.to_radians();
    let lon2_rad = coord2.longitude.to_radians();

    let bx = lat2_rad.cos() * (lon2_rad - lon1_rad).cos();
    let by = lat2_rad.cos() * (lon2_rad - lon1_rad).sin();

    let mid_lat_rad = (lat1_rad.sin() + lat2_rad.sin()).atan2(
        ((lat1_rad.cos() + bx).powi(2) + by.powi(2)).sqrt(),
    );
    let mid_lon_rad = lon1_rad + by.atan2(lat1_rad.cos() + bx);

    let mid_lat = mid_lat_rad.to_degrees();
    let mid_lon = mid_lon_rad.to_degrees();

    // Calculate midpoint altitude if both have altitude
    let mid_alt = match (coord1.altitude, coord2.altitude) {
        (Some(alt1), Some(alt2)) => Some((alt1 + alt2) / 2.0),
        _ => None,
    };

    Ok(Coordinate {
        latitude: mid_lat,
        longitude: mid_lon,
        altitude: mid_alt,
    })
}

/// Calculate the destination point given a starting point, bearing, and distance.
pub fn destination(
    start: &Coordinate,
    bearing_deg: f64,
    distance_meters: f64,
) -> Result<Coordinate> {
    if !start.is_valid() {
        return Err(GeospatialError::InvalidCoordinates(
            "Invalid start coordinate".to_string(),
        ));
    }

    const R: f64 = 6_371_000.0; // Earth radius in meters
    
    let lat_rad = start.latitude.to_radians();
    let lon_rad = start.longitude.to_radians();
    let bearing_rad = bearing_deg.to_radians();
    let angular_distance = distance_meters / R;

    let dest_lat_rad = (lat_rad.sin() * angular_distance.cos()
        + lat_rad.cos() * angular_distance.sin() * bearing_rad.cos())
    .asin();
    
    let dest_lon_rad = lon_rad
        + (bearing_rad.sin() * angular_distance.sin() * lat_rad.cos())
            .atan2(angular_distance.cos() - lat_rad.sin() * dest_lat_rad.sin());

    let dest_lat = dest_lat_rad.to_degrees();
    let dest_lon = dest_lon_rad.to_degrees();

    // Keep the same altitude (or None)
    Ok(Coordinate {
        latitude: dest_lat,
        longitude: dest_lon,
        altitude: start.altitude,
    })
}

/// Check if two coordinates are within a certain distance of each other.
pub fn within_distance(
    coord1: &Coordinate,
    coord2: &Coordinate,
    max_distance_meters: f64,
) -> Result<bool> {
    let distance = haversine_distance_meters(coord1, coord2)?;
    Ok(distance.meters <= max_distance_meters)
}

/// Calculate the area of a polygon defined by coordinates (in square meters, approximate).
pub fn polygon_area(coords: &[Coordinate]) -> Result<f64> {
    if coords.len() < 3 {
        return Err(GeospatialError::InvalidCoordinates(
            "Polygon needs at least 3 coordinates".to_string(),
        ));
    }

    for coord in coords {
        if !coord.is_valid() {
            return Err(GeospatialError::InvalidCoordinates(
                "Invalid coordinate in polygon".to_string(),
            ));
        }
    }

    // Use spherical polygon area calculation (approximate)
    let mut area = 0.0;
    const R: f64 = 6_371_000.0; // Earth radius in meters
    
    for i in 0..coords.len() {
        let j = (i + 1) % coords.len();
        
        let lat1_rad = coords[i].latitude.to_radians();
        let lon1_rad = coords[i].longitude.to_radians();
        let lat2_rad = coords[j].latitude.to_radians();
        let lon2_rad = coords[j].longitude.to_radians();
        
        area += (lon2_rad - lon1_rad) * (2.0 + lat1_rad.sin() + lat2_rad.sin());
    }
    
    area = (area * R * R / 2.0).abs();
    Ok(area)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_haversine_distance() {
        // New York City
        let nyc = Coordinate::new(40.7128, -74.0060);
        // Los Angeles
        let la = Coordinate::new(34.0522, -118.2437);
        
        let distance = haversine_distance_meters(&nyc, &la).unwrap();
        
        // Actual distance is about 3940 km
        assert!(distance.kilometers > 3900.0 && distance.kilometers < 4000.0);
    }

    #[test]
    fn test_initial_bearing() {
        let coord1 = Coordinate::new(0.0, 0.0);
        let coord2 = Coordinate::new(1.0, 1.0);
        
        let bearing = initial_bearing(&coord1, &coord2).unwrap();
        assert!(bearing >= 0.0 && bearing <= 360.0);
    }

    #[test]
    fn test_midpoint() {
        let coord1 = Coordinate::new(0.0, 0.0);
        let coord2 = Coordinate::new(10.0, 10.0);
        
        let midpoint = midpoint(&coord1, &coord2).unwrap();
        assert!(midpoint.latitude > 0.0 && midpoint.latitude < 10.0);
        assert!(midpoint.longitude > 0.0 && midpoint.longitude < 10.0);
    }

    #[test]
    fn test_within_distance() {
        let coord1 = Coordinate::new(40.0, -80.0);
        let coord2 = Coordinate::new(40.001, -80.001); // About 140 meters apart
        
        // Should be within 200 meters
        assert!(within_distance(&coord1, &coord2, 200.0).unwrap());
        
        // Should not be within 100 meters
        assert!(!within_distance(&coord1, &coord2, 100.0).unwrap());
    }
}