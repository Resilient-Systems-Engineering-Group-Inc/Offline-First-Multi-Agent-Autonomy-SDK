//! Demonstration of data versioning with lineage tracking.
//!
//! This example shows:
//! 1. Creating data versions with snapshots
//! 2. Tracking data lineage and provenance
//! 3. Building dependency graphs
//! 4. Querying lineage information
//! 5. Exporting lineage for visualization

use data_versioning::{
    VersionManager, InMemoryStorage, VersionedCrdtMap,
    lineage::{LineageTracker, DataOrigin, DataReference, LineageBuilder, Transformation, TransformationType},
    version::Version,
};
use common::types::AgentId;
use state_sync::crdt_map::CrdtMap;
use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Data Versioning with Lineage Tracking Demo ===\n");

    // 1. Setup version manager and lineage tracker
    let storage = InMemoryStorage::new();
    let version_manager = VersionManager::new(storage);
    let mut lineage_tracker = LineageTracker::new();

    println!("1. Created version manager and lineage tracker");

    // 2. Create some sample data versions
    let agent_id = AgentId::from_u128(12345);
    
    // Create initial sensor data
    let sensor_data_ref = DataReference {
        data_id: "temperature_sensor_1".to_string(),
        version: Version::new(1, agent_id),
        location: Some("sensor://building_a/floor_1".to_string()),
    };

    let sensor_origin = DataOrigin::Sensor {
        sensor_id: "temp_sensor_1".to_string(),
        timestamp: 1000,
        location: Some("Building A, Floor 1".to_string()),
    };

    let sensor_lineage = LineageBuilder::new(sensor_data_ref.clone(), sensor_origin)
        .add_quality_metric("accuracy".to_string(), 0.98)
        .add_quality_metric("sampling_rate".to_string(), 1.0)
        .build();

    lineage_tracker.register_lineage(sensor_lineage)?;
    println!("2. Registered sensor data lineage");

    // 3. Create a transformation (data cleaning)
    let cleaning_transformation = Transformation {
        id: uuid::Uuid::new_v4(),
        transformation_type: TransformationType::Clean,
        parameters: HashMap::from([
            ("method".to_string(), "remove_outliers".to_string()),
            ("threshold".to_string(), "2.5".to_string()),
        ]),
        agent_id: Some(agent_id),
        timestamp: 1100,
        inputs: vec![sensor_data_ref.clone()],
        outputs: vec![DataReference {
            data_id: "cleaned_temperature".to_string(),
            version: Version::new(2, agent_id),
            location: None,
        }],
        metadata: HashMap::from([
            ("description".to_string(), "Remove outliers using IQR method".to_string()),
        ]),
    };

    // 4. Create cleaned data lineage
    let cleaned_data_ref = DataReference {
        data_id: "cleaned_temperature".to_string(),
        version: Version::new(2, agent_id),
        location: None,
    };

    let cleaned_origin = DataOrigin::Agent {
        agent_id,
        timestamp: 1100,
        operation: "data_cleaning".to_string(),
    };

    let cleaned_lineage = LineageBuilder::new(cleaned_data_ref.clone(), cleaned_origin)
        .add_transformation(cleaning_transformation.clone())
        .add_dependency(sensor_data_ref.clone())
        .add_quality_metric("outliers_removed".to_string(), 3.0)
        .add_quality_metric("data_quality".to_string(), 0.95)
        .build();

    lineage_tracker.register_lineage(cleaned_lineage)?;
    println!("3. Registered cleaned data lineage with transformation");

    // 5. Create another transformation (aggregation)
    let aggregation_transformation = Transformation {
        id: uuid::Uuid::new_v4(),
        transformation_type: TransformationType::Aggregate,
        parameters: HashMap::from([
            ("aggregation".to_string(), "hourly_average".to_string()),
            ("window".to_string(), "3600".to_string()),
        ]),
        agent_id: Some(agent_id),
        timestamp: 1200,
        inputs: vec![cleaned_data_ref.clone()],
        outputs: vec![DataReference {
            data_id: "hourly_temperature_avg".to_string(),
            version: Version::new(3, agent_id),
            location: None,
        }],
        metadata: HashMap::from([
            ("unit".to_string(), "celsius".to_string()),
        ]),
    };

    // 6. Create aggregated data lineage
    let aggregated_data_ref = DataReference {
        data_id: "hourly_temperature_avg".to_string(),
        version: Version::new(3, agent_id),
        location: None,
    };

    let aggregated_origin = DataOrigin::Agent {
        agent_id,
        timestamp: 1200,
        operation: "aggregation".to_string(),
    };

    let aggregated_lineage = LineageBuilder::new(aggregated_data_ref.clone(), aggregated_origin)
        .add_transformation(aggregation_transformation.clone())
        .add_dependency(cleaned_data_ref.clone())
        .add_quality_metric("completeness".to_string(), 1.0)
        .add_quality_metric("timeliness".to_string(), 0.9)
        .build();

    lineage_tracker.register_lineage(aggregated_lineage)?;
    println!("4. Registered aggregated data lineage");

    // 7. Query lineage information
    println!("\n5. Querying lineage information:");
    
    // Get provenance for aggregated data
    if let Some(provenance) = lineage_tracker.get_provenance(&aggregated_data_ref) {
        println!("   - Provenance for {}:", aggregated_data_ref.data_id);
        println!("     * Origin: {:?}", provenance.origin);
        println!("     * Total transformations: {}", provenance.total_transformations);
        println!("     * Transformation chain length: {}", provenance.transformation_chain.len());
        
        for (i, transformation) in provenance.transformation_chain.iter().enumerate() {
            println!("     * Transformation {}: {:?} ({:?})", 
                i + 1, 
                transformation.transformation_type,
                transformation.parameters.get("method").unwrap_or(&"N/A".to_string())
            );
        }
    }

    // Find dependents of sensor data
    let dependents = lineage_tracker.find_dependents(&sensor_data_ref);
    println!("\n6. Dependents of sensor data:");
    for dependent in dependents {
        println!("   - {} (v{})", dependent.data_id, dependent.version.seq);
    }

    // 8. Export lineage for visualization
    println!("\n7. Exporting lineage graph:");
    let graphviz = lineage_tracker.export_lineage(data_versioning::lineage::ExportFormat::Graphviz)?;
    
    // Save to file
    std::fs::write("lineage_graph.dot", &graphviz)?;
    println!("   - Graph saved to lineage_graph.dot");
    println!("   - To visualize: dot -Tpng lineage_graph.dot -o lineage_graph.png");

    // 9. Demonstrate version manager integration
    println!("\n8. Demonstrating version manager integration:");
    
    let mut versioned_map = VersionedCrdtMap::new(version_manager);
    versioned_map.map.set("key1", "value1", 0);
    
    let snapshot = versioned_map.snapshot("Initial state".to_string()).await?;
    println!("   - Created snapshot: {}", snapshot.version.to_string());
    
    // 10. Create lineage for the CRDT map snapshot
    let map_data_ref = DataReference {
        data_id: "crdt_map_state".to_string(),
        version: snapshot.version.clone(),
        location: None,
    };

    let map_origin = DataOrigin::Agent {
        agent_id,
        timestamp: 1300,
        operation: "crdt_snapshot".to_string(),
    };

    let map_lineage = LineageBuilder::new(map_data_ref.clone(), map_origin)
        .add_dependency(sensor_data_ref)
        .add_dependency(aggregated_data_ref)
        .add_quality_metric("key_count".to_string(), 1.0)
        .build();

    lineage_tracker.register_lineage(map_lineage)?;
    println!("   - Registered CRDT map lineage with dependencies");

    // 11. Query all lineage records
    let query = data_versioning::lineage::LineageQuery {
        data_id_pattern: Some("temperature".to_string()),
        agent_id: Some(agent_id),
        time_range: Some((1000, 1500)),
        transformation_type: None,
        classification: None,
        max_depth: Some(5),
    };

    let results = lineage_tracker.query_lineage(query);
    println!("\n9. Query results for 'temperature' pattern:");
    println!("   - Found {} lineage records", results.len());
    
    for lineage in results {
        println!("     * {} (v{}) - {} transformations", 
            lineage.data_ref.data_id, 
            lineage.data_ref.version.seq,
            lineage.transformation_chain.len()
        );
    }

    // 12. Export JSON for audit
    println!("\n10. Exporting lineage to JSON for audit:");
    let json_export = lineage_tracker.export_lineage(data_versioning::lineage::ExportFormat::Json)?;
    
    // Save to file
    std::fs::write("lineage_audit.json", &json_export)?;
    println!("   - JSON saved to lineage_audit.json");
    println!("   - Total lineage records: {}", json_export.lines().count());

    println!("\n=== Demo completed successfully ===");
    println!("\nSummary:");
    println!("- Created 4 data lineage records");
    println!("- Tracked 2 transformations (cleaning, aggregation)");
    println!("- Built dependency graph with 3 nodes");
    println!("- Exported visualization (Graphviz) and audit trail (JSON)");
    
    Ok(())
}