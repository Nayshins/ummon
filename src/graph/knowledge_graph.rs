use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

use super::entity::{
    BaseEntity, DomainConceptEntity, Entity, EntityId, EntityType,
    FunctionEntity, ModuleEntity, TypeEntity, VariableEntity,
};
use super::relationship::{Relationship, RelationshipStore, RelationshipType};

/// Enhanced knowledge graph that stores entities and relationships
#[derive(Debug, Serialize, Deserialize)]
pub struct KnowledgeGraph {
    // Entity storage
    entities: HashMap<EntityId, Box<EntityStorage>>,
    
    // Relationship storage
    #[serde(skip)]
    relationship_store: RelationshipStore,
    
    // Serializable relationship data
    relationship_data: Vec<Relationship>,
    
    // Domain concept storage
    domain_concepts: HashMap<String, DomainConceptEntity>,
}

/// Type for storing different entity types
#[derive(Debug, Serialize, Deserialize)]
enum EntityStorage {
    Function(FunctionEntity),
    Type(TypeEntity),
    Module(ModuleEntity),
    Variable(VariableEntity),
    DomainConcept(DomainConceptEntity),
    Base(BaseEntity),
}

impl EntityStorage {
    fn as_entity(&self) -> &dyn Entity {
        match self {
            EntityStorage::Function(f) => f,
            EntityStorage::Type(t) => t,
            EntityStorage::Module(m) => m,
            EntityStorage::Variable(v) => v,
            EntityStorage::DomainConcept(d) => d,
            EntityStorage::Base(b) => b,
        }
    }
}

/// Helper function to downcast a trait object to a concrete type
fn downcast_entity<T: 'static + Clone>(entity: impl Entity + 'static) -> Option<T> {
    // First check if we can directly convert using type_id
    if let Some(concrete) = ((&entity) as &dyn std::any::Any).downcast_ref::<T>() {
        return Some(concrete.clone());
    }
    
    // If direct conversion fails, try the more complex approach
    let boxed: Box<dyn Entity + 'static> = Box::new(entity);
    
    // Convert to Any using transmute 
    let boxed_any: Box<dyn std::any::Any> = unsafe {
        std::mem::transmute(boxed)
    };
    
    match boxed_any.downcast::<T>() {
        Ok(boxed) => Some(*boxed),
        Err(_) => None,
    }
}

impl KnowledgeGraph {
    pub fn new() -> Self {
        Self {
            entities: HashMap::new(),
            relationship_store: RelationshipStore::new(),
            relationship_data: Vec::new(),
            domain_concepts: HashMap::new(),
        }
    }

    /// Add a general entity to the graph
    pub fn add_entity<E: Entity + 'static>(&mut self, entity: E) -> Result<()> {
        let id = entity.id().clone();
        
        // Store based on entity type
        let storage = match entity.entity_type() {
            EntityType::Function | EntityType::Method => {
                if let Some(func_entity) = downcast_entity::<FunctionEntity>(entity) {
                    EntityStorage::Function(func_entity)
                } else {
                    return Err(anyhow::anyhow!("Entity is not a FunctionEntity"));
                }
            },
            EntityType::Class | EntityType::Interface | EntityType::Trait |
            EntityType::Struct | EntityType::Enum | EntityType::Type => {
                if let Some(type_entity) = downcast_entity::<TypeEntity>(entity) {
                    EntityStorage::Type(type_entity)
                } else {
                    return Err(anyhow::anyhow!("Entity is not a TypeEntity"));
                }
            },
            EntityType::Module | EntityType::File => {
                if let Some(module_entity) = downcast_entity::<ModuleEntity>(entity) {
                    EntityStorage::Module(module_entity)
                } else {
                    return Err(anyhow::anyhow!("Entity is not a ModuleEntity"));
                }
            },
            EntityType::Variable | EntityType::Field | EntityType::Constant => {
                if let Some(var_entity) = downcast_entity::<VariableEntity>(entity) {
                    EntityStorage::Variable(var_entity)
                } else {
                    return Err(anyhow::anyhow!("Entity is not a VariableEntity"));
                }
            },
            EntityType::DomainConcept => {
                if let Some(domain_entity) = downcast_entity::<DomainConceptEntity>(entity) {
                    // Also store in domain concepts map
                    self.domain_concepts.insert(domain_entity.name().to_string(), domain_entity.clone());
                    EntityStorage::DomainConcept(domain_entity)
                } else {
                    return Err(anyhow::anyhow!("Entity is not a DomainConceptEntity"));
                }
            },
            _ => {
                if let Some(base_entity) = downcast_entity::<BaseEntity>(entity) {
                    EntityStorage::Base(base_entity)
                } else {
                    return Err(anyhow::anyhow!("Entity could not be converted to a specific type"));
                }
            },
        };
        
        self.entities.insert(id, Box::new(storage));
        Ok(())
    }
    
    /// Get an entity by its ID
    pub fn get_entity(&self, id: &EntityId) -> Option<&dyn Entity> {
        self.entities.get(id).map(|e| e.as_entity())
    }
    
    /// Get all entities
    pub fn get_all_entities(&self) -> Vec<&dyn Entity> {
        self.entities.values().map(|e| e.as_entity()).collect()
    }
    /// Get entities by type
    pub fn get_entities_by_type(&self, entity_type: &EntityType) -> Vec<&dyn Entity> {
        self.entities
            .values()
            .filter(|e| &e.as_entity().entity_type() == entity_type)
            .map(|e| e.as_entity())
            .collect()
    }
    
    /// Add a relationship between entities
    pub fn add_relationship(&mut self, relationship: Relationship) {
        self.relationship_data.push(relationship.clone());
        self.relationship_store.add_relationship(relationship);
    }
    
    /// Create and add a relationship between entities
    pub fn create_relationship(
        &mut self,
        source_id: EntityId,
        target_id: EntityId,
        rel_type: RelationshipType,
    ) -> Result<()> {
        // Allow creating relationships even if entities don't exist yet
        // This makes the knowledge graph more flexible and allows for relationships
        // between entities that are referenced but not yet fully analyzed
        
        // Generate relationship ID
        let rel_id = Relationship::generate_id(&source_id, &target_id, &rel_type);
        let relationship = Relationship::new(rel_id, source_id, target_id, rel_type);
        
        // Add the relationship
        self.add_relationship(relationship);
        
        Ok(())
    }
    
    /// Get relationships by source entity
    pub fn get_outgoing_relationships(&self, source_id: &EntityId) -> Vec<&Relationship> {
        self.relationship_store.get_outgoing_relationships(source_id)
    }
    
    /// Get relationships by target entity
    pub fn get_incoming_relationships(&self, target_id: &EntityId) -> Vec<&Relationship> {
        self.relationship_store.get_incoming_relationships(target_id)
    }
    
    /// Get related entities (outgoing)
    pub fn get_related_entities(&self, source_id: &EntityId, rel_type: Option<&RelationshipType>) -> Vec<&dyn Entity> {
        let relationships = self.relationship_store.get_outgoing_relationships(source_id);
        
        relationships
            .into_iter()
            .filter(|rel| match rel_type {
                Some(rt) => &rel.relationship_type == rt,
                None => true,
            })
            .filter_map(|rel| self.get_entity(&rel.target_id))
            .collect()
    }
    
    /// Get related entities (incoming)
    pub fn get_dependent_entities(&self, target_id: &EntityId, rel_type: Option<&RelationshipType>) -> Vec<&dyn Entity> {
        let relationships = self.relationship_store.get_incoming_relationships(target_id);
        
        relationships
            .into_iter()
            .filter(|rel| match rel_type {
                Some(rt) => &rel.relationship_type == rt,
                None => true,
            })
            .filter_map(|rel| self.get_entity(&rel.source_id))
            .collect()
    }
    
    /// Find paths between entities
    pub fn find_paths(
        &self,
        from_id: &EntityId,
        to_id: &EntityId,
        max_depth: usize,
    ) -> Vec<Vec<&dyn Entity>> {
        let mut paths = Vec::new();
        let mut visited = HashSet::new();
        let mut current_path = Vec::new();
        
        if let Some(entity) = self.get_entity(from_id) {
            visited.insert(from_id.clone());
            current_path.push(entity);
            
            self.find_paths_recursive(
                from_id,
                to_id,
                max_depth,
                &mut visited,
                &mut current_path,
                &mut paths,
            );
        }
        
        paths
    }
    
    fn find_paths_recursive<'a>(
        &'a self,
        current_id: &EntityId,
        target_id: &EntityId,
        max_depth: usize,
        visited: &mut HashSet<EntityId>,
        current_path: &mut Vec<&'a dyn Entity>,
        paths: &mut Vec<Vec<&'a dyn Entity>>,
    ) {
        if current_id == target_id {
            // Found a path
            paths.push(current_path.clone());
            return;
        }
        
        if current_path.len() >= max_depth {
            // Reached maximum depth
            return;
        }
        
        // Get outgoing relationships
        let relationships = self.relationship_store.get_outgoing_relationships(current_id);
        
        for rel in relationships {
            let next_id = &rel.target_id;
            
            if !visited.contains(next_id) {
                if let Some(next_entity) = self.get_entity(next_id) {
                    // Mark as visited
                    visited.insert(next_id.clone());
                    current_path.push(next_entity);
                    
                    // Recursive call
                    self.find_paths_recursive(
                        next_id,
                        target_id,
                        max_depth,
                        visited,
                        current_path,
                        paths,
                    );
                    
                    // Backtrack
                    current_path.pop();
                    visited.remove(next_id);
                }
            }
        }
    }
    
    /// Get domain concepts
    pub fn get_domain_concepts(&self) -> Vec<&DomainConceptEntity> {
        self.domain_concepts.values().collect()
    }
    
    /// Get the total number of relationships
    pub fn get_relationship_count(&self) -> usize {
        self.relationship_data.len()
    }
    
    /// Find entities related to a domain concept
    #[allow(dead_code)]
    pub fn get_entities_for_domain_concept(&self, concept_name: &str) -> Vec<&dyn Entity> {
        if let Some(concept) = self.domain_concepts.get(concept_name) {
            self.get_related_entities(concept.id(), Some(&RelationshipType::RepresentedBy))
        } else {
            Vec::new()
        }
    }
    /// Find domain concepts for a code entity
    pub fn get_domain_concepts_for_entity(&self, entity_id: &EntityId) -> Vec<&DomainConceptEntity> {
        self.get_dependent_entities(entity_id, Some(&RelationshipType::RepresentedBy))
            .into_iter()
            .filter_map(|e| {
                if let EntityType::DomainConcept = e.entity_type() {
                    // This is a bit hacky but we know we have a DomainConceptEntity
                    self.domain_concepts.get(e.name())
                } else {
                    None
                }
            })
            .collect()
    }
    
    /// Calculate impact of a change to an entity
    pub fn calculate_impact(&self, entity_id: &EntityId, max_depth: usize) -> HashMap<EntityId, f32> {
        let mut impact = HashMap::new();
        let mut queue = Vec::new();
        
        // Start with the entity itself
        queue.push((entity_id.clone(), 1.0f32));
        
        for _ in 0..max_depth {
            if queue.is_empty() {
                break;
            }
            
            let mut next_queue = Vec::new();
            
            for (current_id, current_weight) in queue {
                // Record impact
                let entry = impact.entry(current_id.clone()).or_insert(0.0);
                if current_weight > *entry {
                    *entry = current_weight;
                }
                
                // Find dependent entities
                let relationships = self.relationship_store.get_incoming_relationships(&current_id);
                
                for rel in relationships {
                    let source_id = &rel.source_id;
                    
                    // Calculate new weight based on relationship type and current weight
                    let rel_weight = match rel.relationship_type {
                        RelationshipType::Calls => 0.9,
                        RelationshipType::Implements => 0.9,
                        RelationshipType::Inherits => 0.9,
                        RelationshipType::Contains => 0.8,
                        RelationshipType::Imports => 0.7,
                        RelationshipType::References => 0.6,
                        RelationshipType::Uses => 0.5,
                        RelationshipType::Depends => 0.8,
                        RelationshipType::DependsOn => 0.7,
                        _ => 0.5,
                    };
                    
                    let new_weight = current_weight * rel_weight * rel.weight;
                    
                    // Only continue if the impact is significant
                    if new_weight > 0.1 {
                        next_queue.push((source_id.clone(), new_weight));
                    }
                }
            }
            
            queue = next_queue;
        }
        
        impact
    }

    // No more legacy methods

    pub fn save_to_file(&self, path: &str) -> Result<()> {
        let json = serde_json::to_string_pretty(&self)?;
        std::fs::write(path, json)?;
        Ok(())
    }

    pub fn load_from_file(path: &str) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let mut graph: Self = serde_json::from_str(&content)?;
        
        // Rebuild relationship store from serialized data
        for rel in &graph.relationship_data {
            graph.relationship_store.add_relationship(rel.clone());
        }
        
        Ok(graph)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::entity::{EntityId, BaseEntity, EntityType, FunctionEntity, DomainConceptEntity, Visibility};
    use crate::graph::relationship::{Relationship, RelationshipType};

    #[test]
    fn test_new_knowledge_graph() {
        let kg = KnowledgeGraph::new();
        assert_eq!(kg.entities.len(), 0);
        assert_eq!(kg.get_relationship_count(), 0);
        assert!(kg.get_domain_concepts().is_empty());
    }

    #[test]
    fn test_add_entity() {
        let mut kg = KnowledgeGraph::new();
        
        // Create a simple function entity
        let id = EntityId::new("test::function");
        let base = BaseEntity::new(
            id.clone(),
            "testFunction".to_string(),
            EntityType::Function,
            Some("test.rs".to_string()),
        );
        
        let function = FunctionEntity {
            base,
            parameters: vec![],
            return_type: None,
            visibility: Visibility::Public,
            is_async: false,
            is_static: false,
            is_constructor: false,
            is_abstract: false,
        };
        
        // Add the entity
        let result = kg.add_entity(function);
        assert!(result.is_ok());
        
        // Verify it was added
        assert_eq!(kg.entities.len(), 1);
        
        // Get the entity
        let entity = kg.get_entity(&id);
        assert!(entity.is_some());
        
        let entity = entity.unwrap();
        assert_eq!(entity.name(), "testFunction");
        assert!(matches!(entity.entity_type(), EntityType::Function));
        assert_eq!(entity.file_path().unwrap(), "test.rs");
    }
    
    #[test]
    fn test_add_relationship() {
        let mut kg = KnowledgeGraph::new();
        
        // Create two entities
        let id1 = EntityId::new("function1");
        let base1 = BaseEntity::new(
            id1.clone(),
            "function1".to_string(),
            EntityType::Function,
            Some("test.rs".to_string()),
        );
        
        let function1 = FunctionEntity {
            base: base1,
            parameters: vec![],
            return_type: None,
            visibility: Visibility::Public,
            is_async: false,
            is_static: false,
            is_constructor: false,
            is_abstract: false,
        };
        
        let id2 = EntityId::new("function2");
        let base2 = BaseEntity::new(
            id2.clone(),
            "function2".to_string(),
            EntityType::Function,
            Some("test.rs".to_string()),
        );
        
        let function2 = FunctionEntity {
            base: base2,
            parameters: vec![],
            return_type: None,
            visibility: Visibility::Public,
            is_async: false,
            is_static: false,
            is_constructor: false,
            is_abstract: false,
        };
        
        // Add entities
        kg.add_entity(function1).unwrap();
        kg.add_entity(function2).unwrap();
        
        // Create and add relationship
        let rel_id = Relationship::generate_id(&id1, &id2, &RelationshipType::Calls);
        let relationship = Relationship::new(rel_id, id1.clone(), id2.clone(), RelationshipType::Calls);
        
        kg.add_relationship(relationship);
        
        // Verify relationship
        assert_eq!(kg.get_relationship_count(), 1);
        
        // Check outgoing relationships
        let outgoing = kg.get_outgoing_relationships(&id1);
        assert_eq!(outgoing.len(), 1);
        assert!(matches!(outgoing[0].relationship_type, RelationshipType::Calls));
        assert_eq!(outgoing[0].target_id.as_str(), id2.as_str());
        
        // Check incoming relationships
        let incoming = kg.get_incoming_relationships(&id2);
        assert_eq!(incoming.len(), 1);
        assert!(matches!(incoming[0].relationship_type, RelationshipType::Calls));
        assert_eq!(incoming[0].source_id.as_str(), id1.as_str());
    }
    
    #[test]
    fn test_get_entities_by_type() {
        let mut kg = KnowledgeGraph::new();
        
        // Add a function entity
        let id1 = EntityId::new("function1");
        let base1 = BaseEntity::new(
            id1.clone(),
            "function1".to_string(),
            EntityType::Function,
            Some("test.rs".to_string()),
        );
        
        let function = FunctionEntity {
            base: base1,
            parameters: vec![],
            return_type: None,
            visibility: Visibility::Public,
            is_async: false,
            is_static: false,
            is_constructor: false,
            is_abstract: false,
        };
        
        // Add a domain concept entity
        let id2 = EntityId::new("concept1");
        let base2 = BaseEntity::new(
            id2.clone(),
            "User".to_string(),
            EntityType::DomainConcept,
            None,
        );
        
        let concept = DomainConceptEntity {
            base: base2,
            attributes: vec!["username".to_string()],
            description: Some("A user in the system".to_string()),
            confidence: 0.9,
        };
        
        // Add entities
        kg.add_entity(function).unwrap();
        kg.add_entity(concept).unwrap();
        
        // Get entities by type
        let functions = kg.get_entities_by_type(&EntityType::Function);
        assert_eq!(functions.len(), 1);
        assert_eq!(functions[0].name(), "function1");
        
        let concepts = kg.get_entities_by_type(&EntityType::DomainConcept);
        assert_eq!(concepts.len(), 1);
        assert_eq!(concepts[0].name(), "User");
        
        let methods = kg.get_entities_by_type(&EntityType::Method);
        assert_eq!(methods.len(), 0);
    }
    
    #[test]
    fn test_find_paths() {
        let mut kg = KnowledgeGraph::new();
        
        // Create three entities in a chain: A -> B -> C
        let id_a = EntityId::new("A");
        let base_a = BaseEntity::new(
            id_a.clone(),
            "A".to_string(),
            EntityType::Function,
            Some("test.rs".to_string()),
        );
        
        let function_a = FunctionEntity {
            base: base_a,
            parameters: vec![],
            return_type: None,
            visibility: Visibility::Public,
            is_async: false,
            is_static: false,
            is_constructor: false,
            is_abstract: false,
        };
        
        let id_b = EntityId::new("B");
        let base_b = BaseEntity::new(
            id_b.clone(),
            "B".to_string(),
            EntityType::Function,
            Some("test.rs".to_string()),
        );
        
        let function_b = FunctionEntity {
            base: base_b,
            parameters: vec![],
            return_type: None,
            visibility: Visibility::Public,
            is_async: false,
            is_static: false,
            is_constructor: false,
            is_abstract: false,
        };
        
        let id_c = EntityId::new("C");
        let base_c = BaseEntity::new(
            id_c.clone(),
            "C".to_string(),
            EntityType::Function,
            Some("test.rs".to_string()),
        );
        
        let function_c = FunctionEntity {
            base: base_c,
            parameters: vec![],
            return_type: None,
            visibility: Visibility::Public,
            is_async: false,
            is_static: false,
            is_constructor: false,
            is_abstract: false,
        };
        
        // Add entities
        kg.add_entity(function_a).unwrap();
        kg.add_entity(function_b).unwrap();
        kg.add_entity(function_c).unwrap();
        
        // Create relationships A -> B and B -> C
        kg.create_relationship(id_a.clone(), id_b.clone(), RelationshipType::Calls).unwrap();
        kg.create_relationship(id_b.clone(), id_c.clone(), RelationshipType::Calls).unwrap();
        
        // Find path from A to C
        let paths = kg.find_paths(&id_a, &id_c, 3);
        assert_eq!(paths.len(), 1);
        assert_eq!(paths[0].len(), 3);
        assert_eq!(paths[0][0].name(), "A");
        assert_eq!(paths[0][1].name(), "B");
        assert_eq!(paths[0][2].name(), "C");
        
        // Try with insufficient depth
        let paths = kg.find_paths(&id_a, &id_c, 1);
        assert_eq!(paths.len(), 0);
        
        // Direct path shouldn't exist
        assert!(kg.get_outgoing_relationships(&id_a).iter().all(|r| r.target_id != id_c));
    }
    
    #[test]
    fn test_domain_concepts() {
        let mut kg = KnowledgeGraph::new();
        
        // Create a domain concept
        let id = EntityId::new("user");
        let base = BaseEntity::new(
            id.clone(),
            "User".to_string(),
            EntityType::DomainConcept,
            None,
        );
        
        let concept = DomainConceptEntity {
            base,
            attributes: vec!["username".to_string(), "email".to_string()],
            description: Some("A user in the system".to_string()),
            confidence: 0.9,
        };
        
        // Add the entity
        kg.add_entity(concept).unwrap();
        
        // Verify domain concepts
        let concepts = kg.get_domain_concepts();
        assert_eq!(concepts.len(), 1);
        assert_eq!(concepts[0].name(), "User");
        assert_eq!(concepts[0].attributes.len(), 2);
        
        // Create a related code entity
        let code_id = EntityId::new("user_class");
        let code_base = BaseEntity::new(
            code_id.clone(),
            "UserClass".to_string(),
            EntityType::Class,
            Some("user.rs".to_string()),
        );
        
        let code_entity = TypeEntity {
            base: code_base,
            fields: vec![],
            methods: vec![],
            supertypes: vec![],
            visibility: Visibility::Public,
            is_abstract: false,
        };
        
        kg.add_entity(code_entity).unwrap();
        
        // Create relationship between domain concept and code
        kg.create_relationship(id.clone(), code_id.clone(), RelationshipType::RepresentedBy).unwrap();
        
        // Check related entities
        let related = kg.get_related_entities(&id, Some(&RelationshipType::RepresentedBy));
        assert_eq!(related.len(), 1);
        assert_eq!(related[0].name(), "UserClass");
    }
}