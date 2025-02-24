use anyhow::{anyhow, Result};
use std::collections::HashMap;
use tracing::info;

use crate::graph::entity::{DomainConceptEntity, Entity, EntityId, EntityType};
use crate::graph::relationship::RelationshipType;
use crate::graph::KnowledgeGraph;

/// Model for domain concept to code mapping results
#[derive(Debug, Clone)]
pub struct DomainCodeMapping {
    /// The domain concept
    pub concept: String,
    /// Entities that represent this concept
    pub entities: Vec<EntityMapping>,
    /// Related domain concepts
    pub related_concepts: Vec<String>,
}

/// Entity mapping with metadata
#[derive(Debug, Clone)]
pub struct EntityMapping {
    /// Entity ID
    pub id: String,
    /// Entity name
    pub name: String,
    /// Entity type
    pub entity_type: String,
    /// Importance/relevance score (0.0 to 1.0)
    pub relevance: f32,
    /// File path if available
    pub file_path: Option<String>,
}

/// Map a domain concept to code implementations
pub async fn map_domain_to_code(concept_name: &str) -> Result<DomainCodeMapping> {
    // Load the knowledge graph
    let kg = KnowledgeGraph::load_from_file("knowledge_graph.json")?;
    
    info!("Mapping domain concept '{}' to code", concept_name);
    
    // Find the domain concept entity
    let domain_concepts = kg.get_domain_concepts();
    let concept = domain_concepts
        .iter()
        .find(|c| c.name().to_lowercase() == concept_name.to_lowercase())
        .ok_or_else(|| anyhow!("Domain concept '{}' not found", concept_name))?;
    
    // Get entities related to this concept
    let related_entities = kg.get_related_entities(concept.id(), Some(&RelationshipType::RepresentedBy));
    
    // Transform to our mapping model
    let mut entities = Vec::new();
    for entity in related_entities {
        let relevance = entity.metadata()
            .get("relevance")
            .and_then(|s| s.parse::<f32>().ok())
            .unwrap_or(1.0);
            
        entities.push(EntityMapping {
            id: entity.id().as_str().to_string(),
            name: entity.name().to_string(),
            entity_type: format!("{:?}", entity.entity_type()),
            relevance,
            file_path: entity.metadata().get("file_path").cloned(),
        });
    }
    
    // Sort by relevance
    entities.sort_by(|a, b| b.relevance.partial_cmp(&a.relevance).unwrap());
    
    // Find related concepts
    let related_concepts = kg
        .get_related_entities(concept.id(), Some(&RelationshipType::RelatesTo))
        .iter()
        .filter_map(|e| {
            if let EntityType::DomainConcept = e.entity_type() {
                Some(e.name().to_string())
            } else {
                None
            }
        })
        .collect();
    
    let mapping = DomainCodeMapping {
        concept: concept.name().to_string(),
        entities,
        related_concepts,
    };
    
    Ok(mapping)
}

/// Map code to domain concepts
pub async fn map_code_to_domain(file_path: &str) -> Result<Vec<DomainCodeMapping>> {
    // Load the knowledge graph
    let kg = KnowledgeGraph::load_from_file("knowledge_graph.json")?;
    
    info!("Mapping code at '{}' to domain concepts", file_path);
    
    // Find entities in this file
    let entity_id = find_entity_by_path(&kg, file_path)?;
    
    // Find all domain concepts related to this entity
    let related_concepts = get_all_related_domain_concepts(&kg, &entity_id);
    
    // For each concept, map it back to code
    let mut mappings = Vec::new();
    for concept in related_concepts {
        // Get this concept's mapping
        if let Ok(mapping) = map_domain_to_code(concept.name()).await {
            mappings.push(mapping);
        }
    }
    
    Ok(mappings)
}

/// Find an entity by file path
fn find_entity_by_path(kg: &KnowledgeGraph, path: &str) -> Result<EntityId> {
    // Try to find the file or module
    for entity in kg.get_all_entities() {
        if let Some(file_path) = entity.metadata().get("file_path") {
            if file_path == path {
                return Ok(entity.id().clone());
            }
        }
        
        if let Some(file_path) = entity.metadata().get("path") {
            if file_path == path {
                return Ok(entity.id().clone());
            }
        }
    }
    
    anyhow::bail!("Could not find entity for path: {}", path)
}

/// Get all domain concepts related to an entity
fn get_all_related_domain_concepts<'a>(kg: &'a KnowledgeGraph, entity_id: &EntityId) -> Vec<&'a DomainConceptEntity> {
    // Start with direct domain concepts
    let mut concepts = kg.get_domain_concepts_for_entity(entity_id);
    
    // For each directly related entity, also check its domain concepts
    for related_entity in kg.get_related_entities(entity_id, None) {
        for concept in kg.get_domain_concepts_for_entity(related_entity.id()) {
            if !concepts.contains(&concept) {
                concepts.push(concept);
            }
        }
    }
    
    // Sort by confidence
    concepts.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap_or(std::cmp::Ordering::Equal));
    
    concepts
}

/// Get detailed information about a domain concept
pub async fn get_domain_concept_details(
    kg: &KnowledgeGraph, 
    concept_name: &str
) -> Result<HashMap<String, serde_json::Value>> {
    let mut details = HashMap::new();
    
    // Find the domain concept
    let domain_concepts = kg.get_domain_concepts();
    let concept = domain_concepts
        .iter()
        .find(|c| c.name().to_lowercase() == concept_name.to_lowercase())
        .ok_or_else(|| anyhow!("Domain concept '{}' not found", concept_name))?;
    
    // Basic information
    details.insert("name".to_string(), serde_json::json!(concept.name()));
    details.insert("description".to_string(), serde_json::json!(concept.description.clone().unwrap_or_default()));
    details.insert("confidence".to_string(), serde_json::json!(concept.confidence));
    details.insert("attributes".to_string(), serde_json::json!(concept.attributes));
    
    // Related concepts
    let related_concepts: Vec<String> = kg
        .get_related_entities(concept.id(), Some(&RelationshipType::RelatesTo))
        .iter()
        .filter_map(|e| {
            if let EntityType::DomainConcept = e.entity_type() {
                Some(e.name().to_string())
            } else {
                None
            }
        })
        .collect();
    details.insert("related_concepts".to_string(), serde_json::json!(related_concepts));
    
    // Implementation entities
    let implementations: Vec<String> = kg
        .get_related_entities(concept.id(), Some(&RelationshipType::RepresentedBy))
        .iter()
        .map(|e| format!("{} ({})", e.name(), format!("{:?}", e.entity_type())))
        .collect();
    details.insert("implementations".to_string(), serde_json::json!(implementations));
    
    Ok(details)
}