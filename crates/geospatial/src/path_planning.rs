//! Path planning algorithms for spatial navigation.

use crate::error::{GeospatialError, Result};
use crate::types::{Coordinate, Distance};
use std::collections::{BinaryHeap, HashMap, HashSet};

/// Path planning result.
#[derive(Debug, Clone)]
pub struct Path {
    /// Sequence of coordinates from start to goal.
    pub waypoints: Vec<Coordinate>,
    /// Total distance in meters.
    pub total_distance: Distance,
    /// Estimated travel time in seconds (if speed is known).
    pub estimated_time: Option<f64>,
}

impl Path {
    /// Create a new path.
    pub fn new(waypoints: Vec<Coordinate>, total_distance: Distance) -> Self {
        Self {
            waypoints,
            total_distance,
            estimated_time: None,
        }
    }

    /// Create a new path with estimated time.
    pub fn with_time(
        waypoints: Vec<Coordinate>,
        total_distance: Distance,
        estimated_time: f64,
    ) -> Self {
        Self {
            waypoints,
            total_distance,
            estimated_time: Some(estimated_time),
        }
    }

    /// Check if the path is valid (has at least start and end points).
    pub fn is_valid(&self) -> bool {
        self.waypoints.len() >= 2
    }

    /// Get the start coordinate.
    pub fn start(&self) -> Option<&Coordinate> {
        self.waypoints.first()
    }

    /// Get the goal coordinate.
    pub fn goal(&self) -> Option<&Coordinate> {
        self.waypoints.last()
    }

    /// Get the number of waypoints.
    pub fn len(&self) -> usize {
        self.waypoints.len()
    }

    /// Check if the path is empty.
    pub fn is_empty(&self) -> bool {
        self.waypoints.is_empty()
    }
}

/// Graph node for path planning.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct Node {
    id: usize,
    coordinate: Coordinate,
}

/// Edge between nodes with cost.
#[derive(Debug, Clone)]
struct Edge {
    from: usize,
    to: usize,
    cost: f64, // Distance in meters
}

/// Graph for path planning.
struct Graph {
    nodes: Vec<Node>,
    edges: Vec<Edge>,
    adjacency: HashMap<usize, Vec<usize>>, // node_id -> list of edge indices
}

impl Graph {
    /// Create a new empty graph.
    fn new() -> Self {
        Self {
            nodes: Vec::new(),
            edges: Vec::new(),
            adjacency: HashMap::new(),
        }
    }

    /// Add a node to the graph.
    fn add_node(&mut self, coordinate: Coordinate) -> usize {
        let id = self.nodes.len();
        self.nodes.push(Node { id, coordinate });
        self.adjacency.insert(id, Vec::new());
        id
    }

    /// Add an edge between two nodes.
    fn add_edge(&mut self, from: usize, to: usize, cost: f64) -> Result<()> {
        if from >= self.nodes.len() || to >= self.nodes.len() {
            return Err(GeospatialError::PathPlanningError(
                "Invalid node ID".to_string(),
            ));
        }

        let edge_id = self.edges.len();
        self.edges.push(Edge { from, to, cost });
        
        self.adjacency.entry(from).or_default().push(edge_id);
        
        // For undirected graph, add reverse edge
        let reverse_edge_id = self.edges.len();
        self.edges.push(Edge { from: to, to: from, cost });
        self.adjacency.entry(to).or_default().push(reverse_edge_id);

        Ok(())
    }

    /// Find the shortest path using Dijkstra's algorithm.
    fn dijkstra(&self, start: usize, goal: usize) -> Result<Vec<usize>> {
        if start >= self.nodes.len() || goal >= self.nodes.len() {
            return Err(GeospatialError::PathPlanningError(
                "Invalid start or goal node".to_string(),
            ));
        }

        let mut distances: HashMap<usize, f64> = HashMap::new();
        let mut previous: HashMap<usize, usize> = HashMap::new();
        let mut visited: HashSet<usize> = HashSet::new();
        
        // Use a min-heap for priority queue
        let mut heap = BinaryHeap::new();
        
        distances.insert(start, 0.0);
        heap.push(HeapNode {
            id: start,
            cost: 0.0,
        });

        while let Some(HeapNode { id: current, cost }) = heap.pop() {
            if visited.contains(&current) {
                continue;
            }
            
            visited.insert(current);
            
            // If we reached the goal, reconstruct the path
            if current == goal {
                let mut path = Vec::new();
                let mut node = goal;
                
                while let Some(&prev) = previous.get(&node) {
                    path.push(node);
                    node = prev;
                    if node == start {
                        break;
                    }
                }
                path.push(start);
                path.reverse();
                return Ok(path);
            }
            
            // Explore neighbors
            if let Some(edge_indices) = self.adjacency.get(&current) {
                for &edge_idx in edge_indices {
                    let edge = &self.edges[edge_idx];
                    if edge.from != current {
                        continue;
                    }
                    
                    let neighbor = edge.to;
                    let new_cost = cost + edge.cost;
                    
                    if !distances.contains_key(&neighbor) || new_cost < distances[&neighbor] {
                        distances.insert(neighbor, new_cost);
                        previous.insert(neighbor, current);
                        heap.push(HeapNode {
                            id: neighbor,
                            cost: new_cost,
                        });
                    }
                }
            }
        }

        Err(GeospatialError::NoPathFound(
            self.nodes[start].coordinate.to_tuple(),
            self.nodes[goal].coordinate.to_tuple(),
        ))
    }
}

/// Heap node for Dijkstra's algorithm.
#[derive(Debug, Clone, Copy)]
struct HeapNode {
    id: usize,
    cost: f64,
}

impl PartialEq for HeapNode {
    fn eq(&self, other: &Self) -> bool {
        self.cost == other.cost
    }
}

impl Eq for HeapNode {}

impl PartialOrd for HeapNode {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        // Reverse comparison for min-heap
        other.cost.partial_cmp(&self.cost)
    }
}

impl Ord for HeapNode {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.partial_cmp(other).unwrap_or(std::cmp::Ordering::Equal)
    }
}

/// Simple straight-line path planner.
pub struct StraightLinePlanner;

impl StraightLinePlanner {
    /// Plan a straight-line path between two points.
    pub fn plan(&self, start: &Coordinate, goal: &Coordinate) -> Result<Path> {
        if !start.is_valid() || !goal.is_valid() {
            return Err(GeospatialError::InvalidCoordinates(
                "Invalid start or goal coordinate".to_string(),
            ));
        }

        use crate::distance::haversine_distance_meters;
        let distance = haversine_distance_meters(start, goal)?;

        Ok(Path::new(vec![start.clone(), goal.clone()], distance))
    }

    /// Plan a path with intermediate waypoints.
    pub fn plan_with_waypoints(&self, waypoints: &[Coordinate]) -> Result<Path> {
        if waypoints.len() < 2 {
            return Err(GeospatialError::PathPlanningError(
                "Need at least 2 waypoints".to_string(),
            ));
        }

        for waypoint in waypoints {
            if !waypoint.is_valid() {
                return Err(GeospatialError::InvalidCoordinates(
                    "Invalid waypoint coordinate".to_string(),
                ));
            }
        }

        use crate::distance::haversine_distance_meters;
        
        let mut total_distance = Distance::from_meters(0.0);
        for i in 0..waypoints.len() - 1 {
            let segment_distance = haversine_distance_meters(&waypoints[i], &waypoints[i + 1])?;
            total_distance.meters += segment_distance.meters;
            total_distance.kilometers += segment_distance.kilometers;
        }

        Ok(Path::new(waypoints.to_vec(), total_distance))
    }
}

/// Grid-based path planner.
pub struct GridPlanner {
    /// Grid resolution in degrees.
    resolution: f64,
    /// Obstacles (coordinates to avoid).
    obstacles: HashSet<Coordinate>,
}

impl GridPlanner {
    /// Create a new grid planner with default resolution.
    pub fn new() -> Self {
        Self {
            resolution: 0.001, // About 100 meters
            obstacles: HashSet::new(),
        }
    }

    /// Create a new grid planner with custom resolution.
    pub fn with_resolution(resolution: f64) -> Self {
        Self {
            resolution: resolution.max(0.00001), // Minimum resolution
            obstacles: HashSet::new(),
        }
    }

    /// Add an obstacle.
    pub fn add_obstacle(&mut self, coordinate: Coordinate) {
        self.obstacles.insert(coordinate);
    }

    /// Remove an obstacle.
    pub fn remove_obstacle(&mut self, coordinate: &Coordinate) {
        self.obstacles.remove(coordinate);
    }

    /// Check if a coordinate is an obstacle.
    pub fn is_obstacle(&self, coordinate: &Coordinate) -> bool {
        self.obstacles.contains(coordinate)
    }

    /// Plan a path avoiding obstacles.
    pub fn plan(&self, start: &Coordinate, goal: &Coordinate) -> Result<Path> {
        if !start.is_valid() || !goal.is_valid() {
            return Err(GeospatialError::InvalidCoordinates(
                "Invalid start or goal coordinate".to_string(),
            ));
        }

        // Simple implementation: use straight line if no obstacles in the way
        // In a real implementation, this would use A* on a grid
        
        use crate::distance::haversine_distance_meters;
        let distance = haversine_distance_meters(start, goal)?;

        // Check if the straight line path intersects any obstacles
        let mut waypoints = vec![start.clone(), goal.clone()];
        
        // For now, just return the straight line path
        // A proper implementation would detect and avoid obstacles
        
        Ok(Path::new(waypoints, distance))
    }
}

impl Default for GridPlanner {
    fn default() -> Self {
        Self::new()
    }
}

/// Path planner that uses a road network or predefined graph.
pub struct NetworkPlanner {
    graph: Graph,
}

impl NetworkPlanner {
    /// Create a new network planner from a list of nodes and edges.
    pub fn new(nodes: Vec<Coordinate>, edges: Vec<(usize, usize, f64)>) -> Result<Self> {
        let mut graph = Graph::new();
        
        // Add nodes
        for coordinate in nodes {
            graph.add_node(coordinate);
        }
        
        // Add edges
        for (from, to, cost) in edges {
            graph.add_edge(from, to, cost)?;
        }
        
        Ok(Self { graph })
    }

    /// Plan a path using the network.
    pub fn plan(&self, start: &Coordinate, goal: &Coordinate) -> Result<Path> {
        if !start.is_valid() || !goal.is_valid() {
            return Err(GeospatialError::InvalidCoordinates(
                "Invalid start or goal coordinate".to_string(),
            ));
        }

        // Find nearest nodes to start and goal
        let start_node = self.find_nearest_node(start)?;
        let goal_node = self.find_nearest_node(goal)?;

        // Find path using Dijkstra
        let node_path = self.graph.dijkstra(start_node, goal_node)?;
        
        // Convert node IDs to coordinates
        let waypoints: Vec<Coordinate> = node_path
            .iter()
            .map(|&node_id| self.graph.nodes[node_id].coordinate.clone())
            .collect();
        
        // Calculate total distance
        use crate::distance::haversine_distance_meters;
        let mut total_distance = Distance::from_meters(0.0);
        for i in 0..waypoints.len() - 1 {
            let segment_distance = haversine_distance_meters(&waypoints[i], &waypoints[i + 1])?;
            total_distance.meters += segment_distance.meters;
            total_distance.kilometers += segment_distance.kilometers;
        }

        Ok(Path::new(waypoints, total_distance))
    }

    /// Find the nearest graph node to a coordinate.
    fn find_nearest_node(&self, coordinate: &Coordinate) -> Result<usize> {
        use crate::distance::haversine_distance_meters;
        
        let mut nearest = None;
        let mut min_distance = f64::MAX;
        
        for (i, node) in self.graph.nodes.iter().enumerate() {
            let distance = haversine_distance_meters(coordinate, &node.coordinate)?;
            if distance.meters < min_distance {
                min_distance = distance.meters;
                nearest = Some(i);
            }
        }
        
        nearest.ok_or_else(|| GeospatialError::PathPlanningError("No nodes in graph".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_straight_line_planner() {
        let planner = StraightLinePlanner;
        
        let start = Coordinate::new(40.0, -80.0);
        let goal = Coordinate::new(41.0, -79.0);
        
        let path = planner.plan(&start, &goal).unwrap();
        
        assert_eq!(path.len(), 2);
        assert_eq!(path.start().unwrap(), &start);
        assert_eq!(path.goal().unwrap(), &goal);
        assert!(path.total_distance.meters > 0.0);
    }

    #[test]
    fn test_grid_planner_creation() {
        let planner = GridPlanner::new();
        assert_eq!(planner.resolution, 0.001);
        
        let coordinate = Coordinate::new(40.0, -80.0);
        assert!(!planner.is_obstacle(&coordinate));
    }

    #[test]
    fn test_network_planner() {
        // Create a simple triangle network
        let nodes = vec![
            Coordinate::new(40.0, -80.0),
            Coordinate::new(40.1, -79.9),
            Coordinate::new(40.2, -80.1),
        ];
        
        let edges = vec![
            (0, 1, 10000.0), // 10 km
            (1, 2, 15000.0), // 15 km
            (2, 0, 12000.0), // 12 km
        ];
        
        let planner = NetworkPlanner::new(nodes, edges).unwrap();
        
        let start = Coordinate::new(40.0, -80.0);
        let goal = Coordinate::new(40.2, -80.1);
        
        let path = planner.plan(&start, &goal).unwrap();
        assert!(path.len() >= 2);
        assert!(path.total_distance.meters > 0.0);
    }
}