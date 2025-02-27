use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::hash::Hash;

use super::entity::EntityId;

/// Unique identifier for a relationship
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct RelationshipId(pub String);

impl RelationshipId {
    pub fn new(id: &str) -> Self {
        RelationshipId(id.to_string())
    }
}

/// Relationship type enumeration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum RelationshipType {
    // Technical relationships
    Calls,      // Function/method calls another function/method
    Contains,   // An entity contains another (e.g., class contains methods)
    Imports,    // A module imports another module
    Inherits,   // A class inherits from another class
    Implements, // A class implements an interface/trait
    References, // An entity references another entity
    Defines,    // An entity defines a type/constant/etc.
    Uses,       // An entity uses another entity
    Depends,    // An entity depends on another entity

    // Domain relationships
    RepresentedBy, // A domain concept is represented by code entities
    RelatesTo,     // A domain concept relates to another concept
    DependsOn,     // A domain concept depends on another concept

    // Custom relationship
    Other(String), // Custom relationship type
}

impl std::fmt::Display for RelationshipType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RelationshipType::Calls => write!(f, "Calls"),
            RelationshipType::Contains => write!(f, "Contains"),
            RelationshipType::Imports => write!(f, "Imports"),
            RelationshipType::Inherits => write!(f, "Inherits"),
            RelationshipType::Implements => write!(f, "Implements"),
            RelationshipType::References => write!(f, "References"),
            RelationshipType::Defines => write!(f, "Defines"),
            RelationshipType::Uses => write!(f, "Uses"),
            RelationshipType::Depends => write!(f, "Depends"),
            RelationshipType::RepresentedBy => write!(f, "RepresentedBy"),
            RelationshipType::RelatesTo => write!(f, "RelatesTo"),
            RelationshipType::DependsOn => write!(f, "DependsOn"),
            RelationshipType::Other(s) => write!(f, "Other({})", s),
        }
    }
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
    pub fn generate_id(
        source_id: &EntityId,
        target_id: &EntityId,
        rel_type: &RelationshipType,
    ) -> RelationshipId {
        let type_str = match rel_type {
            RelationshipType::Other(name) => name.clone(),
            _ => format!("{:?}", rel_type),
        };

        RelationshipId::new(&format!(
            "{}->{}::{}",
            source_id.as_str(),
            target_id.as_str(),
            type_str
        ))
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::entity::EntityId;

    #[test]
    fn test_relationship_id_creation() {
        let id = RelationshipId::new("test-rel");
        assert_eq!(id.0, "test-rel");
    }

    #[test]
    fn test_relationship_generate_id() {
        let source = EntityId::new("source");
        let target = EntityId::new("target");
        let rel_type = RelationshipType::Calls;

        let id = Relationship::generate_id(&source, &target, &rel_type);

        assert_eq!(id.0, "source->target::Calls");
    }

    #[test]
    fn test_relationship_creation() {
        let source = EntityId::new("source");
        let target = EntityId::new("target");
        let rel_type = RelationshipType::Calls;
        let id = RelationshipId::new("test-rel");

        let relationship =
            Relationship::new(id.clone(), source.clone(), target.clone(), rel_type.clone());

        assert_eq!(relationship.id.0, id.0);
        assert_eq!(relationship.source_id.0, source.0);
        assert_eq!(relationship.target_id.0, target.0);
        assert!(matches!(
            relationship.relationship_type,
            RelationshipType::Calls
        ));
        assert_eq!(relationship.weight, 1.0);
        assert!(relationship.metadata.is_empty());
    }

    #[test]
    fn test_relationship_store() {
        let mut store = RelationshipStore::new();

        // Create relationships
        let source1 = EntityId::new("source1");
        let source2 = EntityId::new("source2");
        let target = EntityId::new("target");

        let rel1 = Relationship::new(
            RelationshipId::new("rel1"),
            source1.clone(),
            target.clone(),
            RelationshipType::Calls,
        );

        let rel2 = Relationship::new(
            RelationshipId::new("rel2"),
            source2.clone(),
            target.clone(),
            RelationshipType::Imports,
        );

        // Add relationships to store
        store.add_relationship(rel1);
        store.add_relationship(rel2);

        // Test outgoing relationships
        let outgoing1 = store.get_outgoing_relationships(&source1);
        assert_eq!(outgoing1.len(), 1);
        assert_eq!(outgoing1[0].id.0, "rel1");
        assert!(matches!(
            outgoing1[0].relationship_type,
            RelationshipType::Calls
        ));

        let outgoing2 = store.get_outgoing_relationships(&source2);
        assert_eq!(outgoing2.len(), 1);
        assert_eq!(outgoing2[0].id.0, "rel2");
        assert!(matches!(
            outgoing2[0].relationship_type,
            RelationshipType::Imports
        ));

        // Test incoming relationships
        let incoming = store.get_incoming_relationships(&target);
        assert_eq!(incoming.len(), 2);

        // Test non-existent relationships
        let nonexistent = store.get_outgoing_relationships(&EntityId::new("nonexistent"));
        assert_eq!(nonexistent.len(), 0);
    }

    #[test]
    fn test_relationship_types() {
        // Test that all relationship types can be properly created and used
        let relationship_types = vec![
            RelationshipType::Calls,
            RelationshipType::Contains,
            RelationshipType::Imports,
            RelationshipType::Inherits,
            RelationshipType::Implements,
            RelationshipType::References,
            RelationshipType::Defines,
            RelationshipType::Uses,
            RelationshipType::Depends,
            RelationshipType::RepresentedBy,
            RelationshipType::RelatesTo,
            RelationshipType::DependsOn,
            RelationshipType::Other("CustomType".to_string()),
        ];

        let mut store = RelationshipStore::new();
        let source = EntityId::new("source");
        let target = EntityId::new("target");

        // Create a relationship for each type
        for (i, rel_type) in relationship_types.iter().enumerate() {
            let id = RelationshipId::new(&format!("rel{}", i));
            let rel = Relationship::new(id, source.clone(), target.clone(), rel_type.clone());

            store.add_relationship(rel);
        }

        // Verify all relationships were added
        let outgoing = store.get_outgoing_relationships(&source);
        assert_eq!(outgoing.len(), relationship_types.len());
    }

    #[test]
    fn test_relationship_metadata() {
        let source = EntityId::new("source");
        let target = EntityId::new("target");
        let rel_type = RelationshipType::Calls;
        let id = RelationshipId::new("test-rel");

        let mut relationship =
            Relationship::new(id.clone(), source.clone(), target.clone(), rel_type.clone());

        // Add metadata
        relationship
            .metadata
            .insert("key1".to_string(), "value1".to_string());
        relationship
            .metadata
            .insert("key2".to_string(), "value2".to_string());

        // Test metadata
        assert_eq!(relationship.metadata.len(), 2);
        assert_eq!(
            relationship.metadata.get("key1"),
            Some(&"value1".to_string())
        );
        assert_eq!(
            relationship.metadata.get("key2"),
            Some(&"value2".to_string())
        );
    }

    #[test]
    fn test_relationship_weight() {
        let source = EntityId::new("source");
        let target = EntityId::new("target");
        let rel_type = RelationshipType::Calls;
        let id = RelationshipId::new("test-rel");

        let mut relationship =
            Relationship::new(id.clone(), source.clone(), target.clone(), rel_type.clone());

        // Default weight
        assert_eq!(relationship.weight, 1.0);

        // Change weight
        relationship.weight = 0.5;
        assert_eq!(relationship.weight, 0.5);
    }

    #[test]
    fn test_relationship_store_empty() {
        let store = RelationshipStore::new();

        // Test with empty store
        let id = EntityId::new("test");
        assert!(store.get_outgoing_relationships(&id).is_empty());
        assert!(store.get_incoming_relationships(&id).is_empty());
    }

    #[test]
    fn test_relationship_store_multiple_relationships() {
        let mut store = RelationshipStore::new();

        // Create multiple relationships between the same entities
        let source = EntityId::new("source");
        let target = EntityId::new("target");

        let rel1 = Relationship::new(
            RelationshipId::new("rel1"),
            source.clone(),
            target.clone(),
            RelationshipType::Calls,
        );

        let rel2 = Relationship::new(
            RelationshipId::new("rel2"),
            source.clone(),
            target.clone(),
            RelationshipType::References,
        );

        store.add_relationship(rel1);
        store.add_relationship(rel2);

        // Test outgoing relationships - should have two different types
        let outgoing = store.get_outgoing_relationships(&source);
        assert_eq!(outgoing.len(), 2);

        let has_calls = outgoing
            .iter()
            .any(|r| matches!(r.relationship_type, RelationshipType::Calls));
        let has_refs = outgoing
            .iter()
            .any(|r| matches!(r.relationship_type, RelationshipType::References));

        assert!(has_calls, "Should have a Calls relationship");
        assert!(has_refs, "Should have a References relationship");
    }
}
