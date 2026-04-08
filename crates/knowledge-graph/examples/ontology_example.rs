//! Example demonstrating ontology usage with knowledge graph.

use knowledge_graph::{KnowledgeGraph, Ontology, Class, Property, PropertyType};
use serde_json::json;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Knowledge Graph with Ontology Example ===\n");

    // 1. Create an ontology for agent systems
    let mut ontology = Ontology::new("http://example.org/agent#");

    // Define classes
    let mut agent_class = Class::new("Agent", "Autonomous agent");
    agent_class.description = Some("An autonomous entity that can perceive and act".to_string());
    
    let mut task_class = Class::new("Task", "Task to be performed");
    task_class.description = Some("A unit of work that can be assigned to an agent".to_string());
    
    let mut resource_class = Class::new("Resource", "Computational resource");
    resource_class.description = Some("CPU, memory, network, or other resource".to_string());

    // Add classes to ontology
    ontology.add_class(agent_class)?;
    ontology.add_class(task_class)?;
    ontology.add_class(resource_class)?;

    // Define properties
    let mut has_task = Property::new("hasTask", "has task", PropertyType::ObjectProperty);
    has_task.add_domain("Agent");
    has_task.add_range("Task");
    has_task.description = Some("Links an agent to a task it performs".to_string());

    let mut requires_resource = Property::new("requiresResource", "requires resource", PropertyType::ObjectProperty);
    requires_resource.add_domain("Task");
    requires_resource.add_range("Resource");
    requires_resource.description = Some("Links a task to required resources".to_string());

    let mut has_capability = Property::new("hasCapability", "has capability", PropertyType::ObjectProperty);
    has_capability.add_domain("Agent");
    has_capability.add_range("Resource");
    has_capability.description = Some("Links an agent to its capabilities".to_string());

    // Add properties to ontology
    ontology.add_property(has_task)?;
    ontology.add_property(requires_resource)?;
    ontology.add_property(has_capability)?;

    // Validate the ontology
    ontology.validate()?;
    println!("✓ Ontology created and validated successfully");
    println!("  - Classes: {}", ontology.classes.len());
    println!("  - Properties: {}", ontology.properties.len());

    // 2. Create a knowledge graph
    let graph = KnowledgeGraph::new();
    println!("\n✓ Knowledge graph created");

    // 3. Create entities that conform to the ontology
    use knowledge_graph::Entity;

    // Create an agent entity
    let mut agent = Entity::new("Agent");
    agent.set_property("name", json!("Agent-001"));
    agent.set_property("status", json!("active"));
    agent.set_property("hasCapability", json!(["cpu", "memory"]));

    // Create a task entity
    let mut task = Entity::new("Task");
    task.set_property("name", json!("Data processing"));
    task.set_property("priority", json!(5));
    task.set_property("requiresResource", json!(["cpu", "network"]));

    // Create resource entities
    let mut cpu = Entity::new("Resource");
    cpu.set_property("name", json!("CPU"));
    cpu.set_property("type", json!("computational"));
    cpu.set_property("capacity", json!(100));

    let mut memory = Entity::new("Resource");
    memory.set_property("name", json!("Memory"));
    memory.set_property("type", json!("storage"));
    memory.set_property("capacity", json!(8192));

    // 4. Infer entity types using the ontology
    let agent_type = ontology.infer_type(&agent.properties);
    let task_type = ontology.infer_type(&task.properties);
    
    println!("\n✓ Entity type inference:");
    println!("  - Agent inferred types: {:?}", agent_type);
    println!("  - Task inferred types: {:?}", task_type);

    // 5. Export ontology to RDF/Turtle
    let turtle = ontology.to_turtle();
    println!("\n✓ Ontology exported to RDF/Turtle format (first 500 chars):");
    println!("{}", &turtle[..turtle.len().min(500)]);

    // 6. Demonstrate reasoning
    println!("\n✓ Reasoning examples:");
    
    // Check if Agent class exists
    if let Some(agent_class) = ontology.get_class("Agent") {
        println!("  - Agent class label: {}", agent_class.label);
        if let Some(desc) = &agent_class.description {
            println!("  - Description: {}", desc);
        }
    }

    // Get all properties with domain Agent
    println!("  - Properties with domain 'Agent':");
    for (prop_id, property) in &ontology.properties {
        if property.domain.contains("Agent") {
            println!("    * {} ({})", property.label, prop_id);
        }
    }

    println!("\n=== Example completed successfully ===");
    Ok(())
}