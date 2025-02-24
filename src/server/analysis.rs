use anyhow::Result;
use std::collections::HashMap;
use tracing::info;

use crate::graph::entity::{Entity, EntityId, EntityType};
use crate::graph::relationship::RelationshipType;
use crate::graph::KnowledgeGraph;

/// Model for impact analysis results
#[derive(Debug, Clone)]
pub struct ImpactAnalysisResult {
    /// The entity that was analyzed
    pub target_entity: EntityId,
    /// Map of entities and their impact score (0.0 to 1.0)
    pub impacts: HashMap<EntityId, f32>,
    /// Human-readable entity names for clients
    pub entity_names: HashMap<String, String>,
    /// Entity types for visualization/filtering
    pub entity_types: HashMap<String, String>,
    /// Path relationships between entities for visualization
    pub paths: Vec<Vec<String>>,
}

/// Analyze the impact of changing a specific entity
pub async fn analyze_impact(target_path: &str, depth: usize) -> Result<ImpactAnalysisResult> {
    // Load the knowledge graph
    let kg = KnowledgeGraph::load_from_file("knowledge_graph.json")?;
    
    info!("Analyzing impact of changes to {}", target_path);
    
    // Find the entity by path
    let entity_id = find_entity_by_path(&kg, target_path)?;
    
    // Calculate impact using the knowledge graph's algorithm
    let impacts = kg.calculate_impact(&entity_id, depth);
    
    // Prepare the result with human-readable names
    let mut entity_names = HashMap::new();
    let mut entity_types = HashMap::new();
    
    for (id, _) in &impacts {
        if let Some(entity) = kg.get_entity(id) {
            entity_names.insert(id.as_str().to_string(), entity.name().to_string());
            entity_types.insert(id.as_str().to_string(), format!("{:?}", entity.entity_type()));
        }
    }
    
    // Find paths between high-impact entities
    let mut paths = Vec::new();
    let high_impact_entities: Vec<_> = impacts
        .iter()
        .filter(|(_, score)| **score > 0.6)
        .map(|(id, _)| id)
        .collect();
        
    // Only generate paths for a reasonable number of entities
    if high_impact_entities.len() <= 10 {
        for target_id in high_impact_entities {
            let entity_paths = kg.find_paths(&entity_id, target_id, depth);
            
            // Convert paths to strings for serialization
            for path in entity_paths {
                let path_ids: Vec<String> = path
                    .iter()
                    .map(|e| e.id().as_str().to_string())
                    .collect();
                    
                paths.push(path_ids);
            }
        }
    }
    
    let result = ImpactAnalysisResult {
        target_entity: entity_id,
        impacts,
        entity_names,
        entity_types,
        paths,
    };
    
    Ok(result)
}

/// Helper to find an entity by its file path
fn find_entity_by_path(kg: &KnowledgeGraph, path: &str) -> Result<EntityId> {
    // Try to find a module entity first
    for entity in kg.get_entities_by_type(&EntityType::Module) {
        if let Some(file_path) = entity.metadata().get("path") {
            if file_path == path {
                return Ok(entity.id().clone());
            }
        }
    }
    
    // Also try file entities
    for entity in kg.get_entities_by_type(&EntityType::File) {
        if let Some(file_path) = entity.metadata().get("path") {
            if file_path == path {
                return Ok(entity.id().clone());
            }
        }
    }
    
    // Fall back to checking any entity with a file_path
    for entity in kg.get_all_entities() {
        if let Some(file_path) = entity.metadata().get("file_path") {
            if file_path == path {
                return Ok(entity.id().clone());
            }
        }
    }
    
    anyhow::bail!("Could not find entity for path: {}", path)
}

/// Get detailed impact information for a change
pub async fn get_impact_details(kg: &KnowledgeGraph, entity_id: &EntityId) -> Result<HashMap<String, serde_json::Value>> {
    let mut details = HashMap::new();
    
    if let Some(entity) = kg.get_entity(entity_id) {
        // Basic entity information
        details.insert("id".to_string(), serde_json::json!(entity_id.as_str()));
        details.insert("name".to_string(), serde_json::json!(entity.name()));
        details.insert("type".to_string(), serde_json::json!(format!("{:?}", entity.entity_type())));
        
        // Get callers (entities that depend on this one)
        let callers = kg.get_dependent_entities(entity_id, Some(&RelationshipType::Calls));
        let caller_names: Vec<String> = callers.iter().map(|e| e.name().to_string()).collect();
        details.insert("callers".to_string(), serde_json::json!(caller_names));
        
        // Get dependencies (entities this one depends on)
        let dependencies = kg.get_related_entities(entity_id, Some(&RelationshipType::Calls));
        let dependency_names: Vec<String> = dependencies.iter().map(|e| e.name().to_string()).collect();
        details.insert("dependencies".to_string(), serde_json::json!(dependency_names));
        
        // Get related domain concepts
        let domain_concepts = kg.get_domain_concepts_for_entity(entity_id);
        let concept_names: Vec<String> = domain_concepts.iter().map(|e| e.name().to_string()).collect();
        details.insert("domain_concepts".to_string(), serde_json::json!(concept_names));
    }
    
    Ok(details)
}

/// Analyze multiple files for potential impacts
pub async fn analyze_multiple_entities(
    kg: &KnowledgeGraph,
    entity_ids: &[EntityId],
    depth: usize,
) -> Result<HashMap<String, f32>> {
    let mut combined_impacts = HashMap::new();
    
    // Calculate impacts for each entity
    for entity_id in entity_ids {
        let impacts = kg.calculate_impact(entity_id, depth);
        
        // Combine impacts
        for (id, score) in impacts {
            let entry = combined_impacts.entry(id.as_str().to_string()).or_insert(0.0);
            if score > *entry {
                *entry = score;
            }
        }
    }
    
    Ok(combined_impacts)
}