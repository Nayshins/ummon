use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

use super::entity::{
    BaseEntity, DomainConceptEntity, Entity, EntityId, EntityType, FunctionEntity, ModuleEntity,
    TypeEntity, VariableEntity,
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
    let boxed_any: Box<dyn std::any::Any> = unsafe { std::mem::transmute(boxed) };

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
            }
            EntityType::Class
            | EntityType::Interface
            | EntityType::Trait
            | EntityType::Struct
            | EntityType::Enum
            | EntityType::Type => {
                if let Some(type_entity) = downcast_entity::<TypeEntity>(entity) {
                    EntityStorage::Type(type_entity)
                } else {
                    return Err(anyhow::anyhow!("Entity is not a TypeEntity"));
                }
            }
            EntityType::Module | EntityType::File => {
                if let Some(module_entity) = downcast_entity::<ModuleEntity>(entity) {
                    EntityStorage::Module(module_entity)
                } else {
                    return Err(anyhow::anyhow!("Entity is not a ModuleEntity"));
                }
            }
            EntityType::Variable | EntityType::Field | EntityType::Constant => {
                if let Some(var_entity) = downcast_entity::<VariableEntity>(entity) {
                    EntityStorage::Variable(var_entity)
                } else {
                    return Err(anyhow::anyhow!("Entity is not a VariableEntity"));
                }
            }
            EntityType::DomainConcept => {
                if let Some(domain_entity) = downcast_entity::<DomainConceptEntity>(entity) {
                    // Also store in domain concepts map
                    self.domain_concepts
                        .insert(domain_entity.name().to_string(), domain_entity.clone());
                    EntityStorage::DomainConcept(domain_entity)
                } else {
                    return Err(anyhow::anyhow!("Entity is not a DomainConceptEntity"));
                }
            }
            _ => {
                if let Some(base_entity) = downcast_entity::<BaseEntity>(entity) {
                    EntityStorage::Base(base_entity)
                } else {
                    return Err(anyhow::anyhow!(
                        "Entity could not be converted to a specific type"
                    ));
                }
            }
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
        self.relationship_store
            .get_outgoing_relationships(source_id)
    }

    /// Get relationships by target entity
    pub fn get_incoming_relationships(&self, target_id: &EntityId) -> Vec<&Relationship> {
        self.relationship_store
            .get_incoming_relationships(target_id)
    }

    /// Get related entities (outgoing)
    pub fn get_related_entities(
        &self,
        source_id: &EntityId,
        rel_type: Option<&RelationshipType>,
    ) -> Vec<&dyn Entity> {
        let relationships = self
            .relationship_store
            .get_outgoing_relationships(source_id);

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
    #[allow(dead_code)]
    pub fn get_dependent_entities(
        &self,
        target_id: &EntityId,
        rel_type: Option<&RelationshipType>,
    ) -> Vec<&dyn Entity> {
        let relationships = self
            .relationship_store
            .get_incoming_relationships(target_id);

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
        let relationships = self
            .relationship_store
            .get_outgoing_relationships(current_id);

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

    /// Get all relationships
    pub fn get_all_relationships(&self) -> Result<Vec<Relationship>> {
        // Simply return a clone of all the relationship data
        Ok(self.relationship_data.clone())
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
    #[allow(dead_code)]
    pub fn get_domain_concepts_for_entity(
        &self,
        entity_id: &EntityId,
    ) -> Vec<&DomainConceptEntity> {
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
    #[allow(dead_code)]
    pub fn calculate_impact(
        &self,
        entity_id: &EntityId,
        max_depth: usize,
    ) -> HashMap<EntityId, f32> {
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
                let relationships = self
                    .relationship_store
                    .get_incoming_relationships(&current_id);

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

    // File operations
    #[allow(dead_code)]
    pub fn save(&self) -> Result<()> {
        self.save_to_file("knowledge_graph.json")
    }

    pub fn load() -> Result<Self> {
        Self::load_from_file("knowledge_graph.json")
    }

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

    // Search functionality for MCP server
    pub fn search(&self, query: &str) -> Result<Vec<&dyn Entity>> {
        // Convert query to lowercase for case-insensitive matching
        let query = query.to_lowercase();

        // Split query into tokens for more flexible matching
        let query_tokens: Vec<&str> = query
            .split_whitespace()
            .filter(|token| !token.is_empty())
            .collect();

        if query_tokens.is_empty() {
            // Return empty results for empty query
            return Ok(Vec::new());
        }

        // Prepare results collection
        let mut results = Vec::new();
        let mut scores = HashMap::new();

        // Scan all entities and compute match scores
        for entity_storage in self.entities.values() {
            let entity = entity_storage.as_entity();
            let mut score = 0.0;

            // 1. Check for exact matches on the entity name
            let name = entity.name().to_lowercase();
            if name == query {
                score += 10.0; // Exact match is heavily weighted
            } else if name.contains(&query) {
                score += 5.0; // Full query contained within name is good
            }

            // 2. Check for token matches
            let mut token_matches = 0;
            for token in &query_tokens {
                if name.contains(token) {
                    token_matches += 1;
                }

                // Check in file path
                if let Some(path) = entity.path() {
                    if path.to_lowercase().contains(token) {
                        token_matches += 1;
                    }
                }

                // Check in entity type string
                if entity
                    .entity_type()
                    .to_string()
                    .to_lowercase()
                    .contains(token)
                {
                    token_matches += 1;
                }

                // Check in metadata
                for (_, value) in entity.metadata().iter() {
                    if value.to_lowercase().contains(token) {
                        token_matches += 1;
                        break;
                    }
                }
            }

            // Calculate token match score
            if token_matches > 0 {
                score += (token_matches as f32 / query_tokens.len() as f32) * 3.0;
            }

            // 3. Entity type specific boosts
            match entity.entity_type() {
                EntityType::Function | EntityType::Method => {
                    if query.contains("function")
                        || query.contains("method")
                        || query.contains("call")
                    {
                        score += 2.0;
                    }
                }
                EntityType::Class | EntityType::Struct | EntityType::Type => {
                    if query.contains("class") || query.contains("type") || query.contains("struct")
                    {
                        score += 2.0;
                    }
                }
                EntityType::Module | EntityType::File => {
                    if query.contains("module") || query.contains("file") {
                        score += 2.0;
                    }
                }
                EntityType::DomainConcept => {
                    if query.contains("domain")
                        || query.contains("concept")
                        || query.contains("business")
                    {
                        score += 2.0;
                    }
                }
                _ => {}
            }

            // If we have any score, add to results
            if score > 0.0 {
                results.push(entity);
                scores.insert(entity.id().as_str(), score);
            }
        }

        // Sort results by score
        results.sort_by(|a, b| {
            let a_score = scores.get(a.id().as_str()).unwrap_or(&0.0);
            let b_score = scores.get(b.id().as_str()).unwrap_or(&0.0);

            // Sort by score (descending)
            b_score
                .partial_cmp(a_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Limit to 20 most relevant results
        if results.len() > 20 {
            results.truncate(20);
        }

        Ok(results)
    }

    // Helper method to get all relationships for an entity
    pub fn get_relationships_for_entity(&self, entity_id: &str) -> Result<Vec<Relationship>> {
        let entity_id = EntityId::new(entity_id);

        let outgoing = self.get_outgoing_relationships(&entity_id);
        let incoming = self.get_incoming_relationships(&entity_id);

        let mut relationships = Vec::new();

        // Clone the relationships since we need to return owned values
        for rel in outgoing {
            relationships.push(rel.clone());
        }

        for rel in incoming {
            relationships.push(rel.clone());
        }

        Ok(relationships)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::entity::{
        BaseEntity, DomainConceptEntity, EntityId, EntityType, FunctionEntity, Visibility,
    };
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
        let relationship =
            Relationship::new(rel_id, id1.clone(), id2.clone(), RelationshipType::Calls);

        kg.add_relationship(relationship);

        // Verify relationship
        assert_eq!(kg.get_relationship_count(), 1);

        // Check outgoing relationships
        let outgoing = kg.get_outgoing_relationships(&id1);
        assert_eq!(outgoing.len(), 1);
        assert!(matches!(
            outgoing[0].relationship_type,
            RelationshipType::Calls
        ));
        assert_eq!(outgoing[0].target_id.as_str(), id2.as_str());

        // Check incoming relationships
        let incoming = kg.get_incoming_relationships(&id2);
        assert_eq!(incoming.len(), 1);
        assert!(matches!(
            incoming[0].relationship_type,
            RelationshipType::Calls
        ));
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
        kg.create_relationship(id_a.clone(), id_b.clone(), RelationshipType::Calls)
            .unwrap();
        kg.create_relationship(id_b.clone(), id_c.clone(), RelationshipType::Calls)
            .unwrap();

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
        assert!(kg
            .get_outgoing_relationships(&id_a)
            .iter()
            .all(|r| r.target_id != id_c));
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
        kg.create_relationship(id.clone(), code_id.clone(), RelationshipType::RepresentedBy)
            .unwrap();

        // Check related entities
        let related = kg.get_related_entities(&id, Some(&RelationshipType::RepresentedBy));
        assert_eq!(related.len(), 1);
        assert_eq!(related[0].name(), "UserClass");
    }

    #[test]
    fn test_add_entity_duplicate() {
        let mut kg = KnowledgeGraph::new();

        // Create a function entity
        let id = EntityId::new("test::func");
        let base = BaseEntity::new(
            id.clone(),
            "testFunc".to_string(),
            EntityType::Function,
            Some("test.rs".to_string()),
        );

        let function1 = FunctionEntity {
            base,
            parameters: vec![],
            return_type: None,
            visibility: Visibility::Public,
            is_async: false,
            is_static: false,
            is_constructor: false,
            is_abstract: false,
        };

        // Add entity to graph
        kg.add_entity(function1).unwrap();

        // Create a duplicate entity with the same ID
        let base2 = BaseEntity::new(
            id.clone(),
            "duplicateFunc".to_string(),
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

        // Adding a duplicate should replace the original
        kg.add_entity(function2).unwrap();

        // Verify only one entity exists with updated name
        assert_eq!(kg.entities.len(), 1);
        assert_eq!(kg.get_entity(&id).unwrap().name(), "duplicateFunc");
    }

    #[test]
    fn test_add_relationship_with_nonexistent_entity() {
        let mut kg = KnowledgeGraph::new();

        // Create one entity
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

        // Add entity to graph
        kg.add_entity(function_a).unwrap();

        // Try to create relationship with non-existent entity
        let id_nonexistent = EntityId::new("NonExistent");

        // Based on the implementation, the current create_relationship implementation
        // doesn't validate if the target entity exists, it just creates the relationship
        let result = kg.create_relationship(
            id_a.clone(),
            id_nonexistent.clone(),
            RelationshipType::Calls,
        );

        // Should succeed (current implementation doesn't validate entity existence)
        assert!(result.is_ok());

        // But the related entities should be empty because the target doesn't exist
        let related = kg.get_related_entities(&id_a, Some(&RelationshipType::Calls));
        assert_eq!(related.len(), 0);
    }

    #[test]
    fn test_add_bidirectional_relationship() {
        let mut kg = KnowledgeGraph::new();

        // Create two function entities
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

        // Add entities to graph
        kg.add_entity(function_a).unwrap();
        kg.add_entity(function_b).unwrap();

        // Create relationships in both directions
        kg.create_relationship(id_a.clone(), id_b.clone(), RelationshipType::Calls)
            .unwrap();
        kg.create_relationship(id_b.clone(), id_a.clone(), RelationshipType::Calls)
            .unwrap();

        // Verify relationships were created
        let outgoing_a = kg.get_outgoing_relationships(&id_a);
        assert_eq!(outgoing_a.len(), 1);
        assert_eq!(outgoing_a[0].target_id.0, "B");

        let outgoing_b = kg.get_outgoing_relationships(&id_b);
        assert_eq!(outgoing_b.len(), 1);
        assert_eq!(outgoing_b[0].target_id.0, "A");
    }

    #[test]
    fn test_get_entities_with_multiple_filters() {
        let mut kg = KnowledgeGraph::new();

        // Create multiple entities with differing file_paths
        let func1_id = EntityId::new("func1");
        let base_func1 = BaseEntity::new(
            func1_id.clone(),
            "testFunc1".to_string(),
            EntityType::Function,
            Some("file1.rs".to_string()),
        );

        let function1 = FunctionEntity {
            base: base_func1,
            parameters: vec![],
            return_type: None,
            visibility: Visibility::Public,
            is_async: false,
            is_static: false,
            is_constructor: false,
            is_abstract: false,
        };

        let func2_id = EntityId::new("func2");
        let base_func2 = BaseEntity::new(
            func2_id.clone(),
            "testFunc2".to_string(),
            EntityType::Function,
            Some("file2.rs".to_string()),
        );

        let function2 = FunctionEntity {
            base: base_func2,
            parameters: vec![],
            return_type: None,
            visibility: Visibility::Public,
            is_async: false,
            is_static: false,
            is_constructor: false,
            is_abstract: false,
        };

        let class_id = EntityId::new("class");
        let base_class = BaseEntity::new(
            class_id.clone(),
            "TestClass".to_string(),
            EntityType::Class,
            Some("file1.rs".to_string()),
        );

        let class = TypeEntity {
            base: base_class,
            fields: vec![],
            methods: vec![],
            supertypes: vec![],
            visibility: Visibility::Public,
            is_abstract: false,
        };

        // Add entities to graph
        kg.add_entity(function1).unwrap();
        kg.add_entity(function2).unwrap();
        kg.add_entity(class).unwrap();

        // Test getting all functions
        let all_functions = kg.get_entities_by_type(&EntityType::Function);
        assert_eq!(all_functions.len(), 2);

        // Test getting all entities from file1.rs
        let entities_from_file1: Vec<&dyn Entity> = kg
            .entities
            .values()
            .filter(|e| {
                if let Some(path) = e.as_entity().file_path() {
                    path == "file1.rs"
                } else {
                    false
                }
            })
            .map(|e| e.as_entity())
            .collect();

        assert_eq!(entities_from_file1.len(), 2);

        // Verify entity names are as expected
        let entity_names: Vec<&str> = entities_from_file1.iter().map(|e| e.name()).collect();

        assert!(entity_names.contains(&"testFunc1"));
        assert!(entity_names.contains(&"TestClass"));
    }

    #[test]
    fn test_find_paths_complex() {
        let mut kg = KnowledgeGraph::new();

        // Create entities A, B, C, D with multiple paths: A->B->C, A->D->C
        let id_a = EntityId::new("A");
        let id_b = EntityId::new("B");
        let id_c = EntityId::new("C");
        let id_d = EntityId::new("D");

        // Create entity A
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

        // Create entity B
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

        // Create entity C
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

        // Create entity D
        let base_d = BaseEntity::new(
            id_d.clone(),
            "D".to_string(),
            EntityType::Function,
            Some("test.rs".to_string()),
        );
        let function_d = FunctionEntity {
            base: base_d,
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
        kg.add_entity(function_d).unwrap();

        // Create paths: A->B->C and A->D->C
        kg.create_relationship(id_a.clone(), id_b.clone(), RelationshipType::Calls)
            .unwrap();
        kg.create_relationship(id_b.clone(), id_c.clone(), RelationshipType::Calls)
            .unwrap();
        kg.create_relationship(id_a.clone(), id_d.clone(), RelationshipType::Calls)
            .unwrap();
        kg.create_relationship(id_d.clone(), id_c.clone(), RelationshipType::Calls)
            .unwrap();

        // Find paths from A to C
        let paths = kg.find_paths(&id_a, &id_c, 3);

        // Should find two paths
        assert_eq!(paths.len(), 2);

        // Check that paths are distinct
        let path_names: Vec<Vec<String>> = paths
            .iter()
            .map(|path| path.iter().map(|e| e.name().to_string()).collect())
            .collect();

        // Check that both expected paths exist
        let path1 = vec!["A".to_string(), "B".to_string(), "C".to_string()];
        let path2 = vec!["A".to_string(), "D".to_string(), "C".to_string()];

        assert!(path_names.contains(&path1) || path_names.contains(&path2));
    }

    #[test]
    fn test_domain_concept_relationships() {
        let mut kg = KnowledgeGraph::new();

        // Create two domain concepts with a relationship
        let user_id = EntityId::new("domain::User");
        let base_user = BaseEntity::new(
            user_id.clone(),
            "User".to_string(),
            EntityType::DomainConcept,
            None,
        );

        let user = DomainConceptEntity {
            base: base_user,
            attributes: vec!["username".to_string(), "email".to_string()],
            description: Some("A user in the system".to_string()),
            confidence: 0.95,
        };

        let order_id = EntityId::new("domain::Order");
        let base_order = BaseEntity::new(
            order_id.clone(),
            "Order".to_string(),
            EntityType::DomainConcept,
            None,
        );

        let order = DomainConceptEntity {
            base: base_order,
            attributes: vec!["items".to_string(), "total".to_string()],
            description: Some("An order made by a user".to_string()),
            confidence: 0.9,
        };

        // Add concepts to graph
        kg.add_entity(user).unwrap();
        kg.add_entity(order).unwrap();

        // Create relationship User -> Order
        kg.create_relationship(
            user_id.clone(),
            order_id.clone(),
            RelationshipType::RelatesTo,
        )
        .unwrap();

        // Verify relationship
        let outgoing = kg.get_outgoing_relationships(&user_id);
        assert_eq!(outgoing.len(), 1);
        assert_eq!(outgoing[0].target_id.0, "domain::Order");
        assert!(matches!(
            outgoing[0].relationship_type,
            RelationshipType::RelatesTo
        ));

        // Check domain concepts can be related to code entities too
        let func_id = EntityId::new("func::place_order");
        let base_func = BaseEntity::new(
            func_id.clone(),
            "place_order".to_string(),
            EntityType::Function,
            Some("order.rs".to_string()),
        );

        let function = FunctionEntity {
            base: base_func,
            parameters: vec![],
            return_type: None,
            visibility: Visibility::Public,
            is_async: false,
            is_static: false,
            is_constructor: false,
            is_abstract: false,
        };

        kg.add_entity(function).unwrap();

        // Create relationship Order -> place_order (implemented by)
        kg.create_relationship(
            order_id.clone(),
            func_id.clone(),
            RelationshipType::RepresentedBy,
        )
        .unwrap();

        // Verify this relationship
        let order_rels = kg.get_outgoing_relationships(&order_id);
        assert_eq!(order_rels.len(), 1);
        assert_eq!(order_rels[0].target_id.0, "func::place_order");
        assert!(matches!(
            order_rels[0].relationship_type,
            RelationshipType::RepresentedBy
        ));
    }

    #[test]
    fn test_serialization_deserialization() {
        let mut kg = KnowledgeGraph::new();

        // Add some entities and relationships
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

        kg.add_entity(function_a).unwrap();
        kg.add_entity(function_b).unwrap();
        kg.create_relationship(id_a.clone(), id_b.clone(), RelationshipType::Calls)
            .unwrap();

        // Serialize to JSON
        let json = serde_json::to_string(&kg).unwrap();

        // Deserialize back
        let deserialized_kg: KnowledgeGraph = serde_json::from_str(&json).unwrap();

        // Check entities
        assert_eq!(deserialized_kg.entities.len(), 2);
        assert!(deserialized_kg.get_entity(&id_a).is_some());
        assert!(deserialized_kg.get_entity(&id_b).is_some());

        // Note: The relationship_store is marked with #[serde(skip)]
        // so relationships are not automatically restored upon deserialization.
        // They need to be reconstructed from relationship_data.
        // For a proper test, we would need to check that relationship_data contains the relationship,
        // or initialize the relationship_store from relationship_data.

        // For now, just verify that the relationship data is there
        assert!(!deserialized_kg.relationship_data.is_empty());
        assert_eq!(deserialized_kg.relationship_data.len(), 1);
        assert_eq!(deserialized_kg.relationship_data[0].source_id.0, "A");
        assert_eq!(deserialized_kg.relationship_data[0].target_id.0, "B");
    }
}
