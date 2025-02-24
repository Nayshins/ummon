use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::hash::{Hash, Hasher};

use super::entity::EntityId;

/// Unique identifier for a relationship
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct RelationshipId(pub String);

impl RelationshipId {
    pub fn new(id: &str) -> Self {
        RelationshipId(id.to_string())
    }
    
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Relationship type enumeration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum RelationshipType {
    // Technical relationships
    Calls,                 // Function/method calls another function/method
    Contains,              // An entity contains another (e.g., class contains methods)
    Imports,               // A module imports another module
    Inherits,              // A class inherits from another class
    Implements,            // A class implements an interface/trait
    References,            // An entity references another entity
    Defines,               // An entity defines a type/constant/etc.
    Uses,                  // An entity uses another entity
    Depends,               // An entity depends on another entity
    
    // Domain relationships
    RepresentedBy,         // A domain concept is represented by code entities
    RelatesTo,             // A domain concept relates to another concept
    DependsOn,             // A domain concept depends on another concept
    
    // Custom relationship
    Other(String),         // Custom relationship type
}

/// Relationship between entities in the knowledge graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relationship {
    pub id: RelationshipId,
    pub source_id: EntityId,
    pub target_id: EntityId,
    pub relationship_type: RelationshipType,
    pub weight: f32,
    pub metadata: HashMap<String, String>,
}

impl Relationship {
    pub fn new(
        id: RelationshipId,
        source_id: EntityId,
        target_id: EntityId,
        relationship_type: RelationshipType,
    ) -> Self {
        Self {
            id,
            source_id,
            target_id,
            relationship_type,
            weight: 1.0,
            metadata: HashMap::new(),
        }
    }
    
    /// Generate a default relationship ID based on source, target, and type
    pub fn generate_id(source_id: &EntityId, target_id: &EntityId, rel_type: &RelationshipType) -> RelationshipId {
        let type_str = match rel_type {
            RelationshipType::Other(name) => name.clone(),
            _ => format!("{:?}", rel_type),
        };
        
        RelationshipId::new(&format!("{}->{}::{}", source_id.as_str(), target_id.as_str(), type_str))
    }
}

/// A store for efficiently retrieving relationships
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RelationshipStore {
    relationships: HashMap<RelationshipId, Relationship>,
    outgoing_relationships: HashMap<EntityId, Vec<RelationshipId>>,
    incoming_relationships: HashMap<EntityId, Vec<RelationshipId>>,
    relationship_types: HashMap<RelationshipType, Vec<RelationshipId>>,
}

impl RelationshipStore {
    pub fn new() -> Self {
        Self {
            relationships: HashMap::new(),
            outgoing_relationships: HashMap::new(),
            incoming_relationships: HashMap::new(),
            relationship_types: HashMap::new(),
        }
    }
    
    /// Add a relationship to the store
    pub fn add_relationship(&mut self, relationship: Relationship) {
        let rel_id = relationship.id.clone();
        let source_id = relationship.source_id.clone();
        let target_id = relationship.target_id.clone();
        let rel_type = relationship.relationship_type.clone();
        
        // Add to main relationships map
        self.relationships.insert(rel_id.clone(), relationship);
        
        // Add to outgoing index
        self.outgoing_relationships
            .entry(source_id)
            .or_insert_with(Vec::new)
            .push(rel_id.clone());
            
        // Add to incoming index
        self.incoming_relationships
            .entry(target_id)
            .or_insert_with(Vec::new)
            .push(rel_id.clone());
            
        // Add to type index
        let key = match &rel_type {
            RelationshipType::Other(name) => RelationshipType::Other(name.clone()),
            _ => rel_type,
        };
        
        self.relationship_types
            .entry(key)
            .or_insert_with(Vec::new)
            .push(rel_id);
    }
    
    /// Get a relationship by ID
    pub fn get_relationship(&self, id: &RelationshipId) -> Option<&Relationship> {
        self.relationships.get(id)
    }
    
    /// Get all outgoing relationships from an entity
    pub fn get_outgoing_relationships(&self, entity_id: &EntityId) -> Vec<&Relationship> {
        match self.outgoing_relationships.get(entity_id) {
            Some(rel_ids) => rel_ids
                .iter()
                .filter_map(|id| self.relationships.get(id))
                .collect(),
            None => Vec::new(),
        }
    }
    
    /// Get all incoming relationships to an entity
    pub fn get_incoming_relationships(&self, entity_id: &EntityId) -> Vec<&Relationship> {
        match self.incoming_relationships.get(entity_id) {
            Some(rel_ids) => rel_ids
                .iter()
                .filter_map(|id| self.relationships.get(id))
                .collect(),
            None => Vec::new(),
        }
    }
    
    /// Get all relationships of a specific type
    pub fn get_relationships_by_type(&self, rel_type: &RelationshipType) -> Vec<&Relationship> {
        match self.relationship_types.get(rel_type) {
            Some(rel_ids) => rel_ids
                .iter()
                .filter_map(|id| self.relationships.get(id))
                .collect(),
            None => Vec::new(),
        }
    }
    
    /// Get relationships by source, target, and optional type
    pub fn get_relationships_between(
        &self,
        source_id: &EntityId,
        target_id: &EntityId,
        rel_type: Option<&RelationshipType>,
    ) -> Vec<&Relationship> {
        let outgoing = match self.outgoing_relationships.get(source_id) {
            Some(rel_ids) => rel_ids,
            None => return Vec::new(),
        };
        
        outgoing
            .iter()
            .filter_map(|id| self.relationships.get(id))
            .filter(|rel| {
                rel.target_id == *target_id
                    && match rel_type {
                        Some(rt) => &rel.relationship_type == rt,
                        None => true,
                    }
            })
            .collect()
    }
    
    /// Check if a relationship exists between two entities
    pub fn has_relationship_between(
        &self,
        source_id: &EntityId,
        target_id: &EntityId,
        rel_type: Option<&RelationshipType>,
    ) -> bool {
        !self.get_relationships_between(source_id, target_id, rel_type).is_empty()
    }
    
    /// Get all relationships
    pub fn get_all_relationships(&self) -> Vec<&Relationship> {
        self.relationships.values().collect()
    }
    
    /// Count the number of relationships
    pub fn relationship_count(&self) -> usize {
        self.relationships.len()
    }
}