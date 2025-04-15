use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

use super::entity::{
    BaseEntity, DomainConceptEntity, Entity, EntityId, EntityType, FunctionEntity, ModuleEntity,
    TypeEntity, VariableEntity,
};
use super::relationship::{Relationship, RelationshipStore, RelationshipType};
use log::debug;

/// Enhanced knowledge graph that stores entities and relationships
#[derive(Debug, Serialize, Deserialize)]
pub struct KnowledgeGraph {
    entities: HashMap<EntityId, Box<EntityStorage>>,
    #[serde(skip)]
    relationship_store: RelationshipStore,
    relationship_data: Vec<Relationship>,
    #[serde(skip)]
    search_index: HashMap<String, Vec<EntityId>>,
    #[serde(skip)]
    pub database: Option<crate::db::Database>,
}

/// Type for storing different entity types
#[derive(Debug, Clone, Serialize, Deserialize)]
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

impl Default for KnowledgeGraph {
    fn default() -> Self {
        Self::new()
    }
}

impl KnowledgeGraph {
    pub fn new() -> Self {
        Self {
            entities: HashMap::new(),
            relationship_store: RelationshipStore::new(),
            relationship_data: Vec::new(),
            search_index: HashMap::new(),
            database: None,
        }
    }

    /// Create a new knowledge graph with a database connection
    pub fn new_with_db(db: crate::db::Database) -> Self {
        Self {
            entities: HashMap::new(),
            relationship_store: RelationshipStore::new(),
            relationship_data: Vec::new(),
            search_index: HashMap::new(),
            database: Some(db),
        }
    }

    /// Add a boxed entity directly to the graph
    pub fn add_boxed_entity(&mut self, entity: Box<dyn Entity>) -> Result<()> {
        let id = entity.id().clone();
        let entity_type = entity.entity_type();

        let storage = match entity_type {
            EntityType::Function | EntityType::Method => {
                if let Some(func) = (*entity).as_any().downcast_ref::<FunctionEntity>() {
                    EntityStorage::Function(func.clone())
                } else {
                    return Err(anyhow::anyhow!(
                        "Expected FunctionEntity but downcast failed for ID: {}",
                        id.as_str()
                    ));
                }
            }
            EntityType::Class
            | EntityType::Interface
            | EntityType::Trait
            | EntityType::Struct
            | EntityType::Enum
            | EntityType::Type => {
                if let Some(typ) = (*entity).as_any().downcast_ref::<TypeEntity>() {
                    EntityStorage::Type(typ.clone())
                } else {
                    return Err(anyhow::anyhow!(
                        "Expected TypeEntity but downcast failed for ID: {}",
                        id.as_str()
                    ));
                }
            }
            EntityType::Module | EntityType::File => {
                if let Some(module) = (*entity).as_any().downcast_ref::<ModuleEntity>() {
                    EntityStorage::Module(module.clone())
                } else {
                    return Err(anyhow::anyhow!(
                        "Expected ModuleEntity but downcast failed for ID: {}",
                        id.as_str()
                    ));
                }
            }
            EntityType::Variable | EntityType::Field | EntityType::Constant => {
                if let Some(var) = (*entity).as_any().downcast_ref::<VariableEntity>() {
                    EntityStorage::Variable(var.clone())
                } else {
                    return Err(anyhow::anyhow!(
                        "Expected VariableEntity but downcast failed for ID: {}",
                        id.as_str()
                    ));
                }
            }
            EntityType::DomainConcept => {
                if let Some(domain) = (*entity).as_any().downcast_ref::<DomainConceptEntity>() {
                    EntityStorage::DomainConcept(domain.clone())
                } else {
                    return Err(anyhow::anyhow!(
                        "Expected DomainConceptEntity but downcast failed for ID: {}",
                        id.as_str()
                    ));
                }
            }
            _ => {
                if let Some(base) = (*entity).as_any().downcast_ref::<BaseEntity>() {
                    EntityStorage::Base(base.clone())
                } else {
                    return Err(anyhow::anyhow!(
                        "Unknown entity type for ID: {}",
                        id.as_str()
                    ));
                }
            }
        };

        let entity_name = entity.name().to_lowercase();
        let entity_path = entity.path().map(|p| p.to_lowercase());
        let entity_type_str = entity.entity_type().to_string().to_lowercase();

        let metadata_values: Vec<String> = entity
            .metadata()
            .iter()
            .map(|(_key, v)| v.to_lowercase())
            .collect();

        self.entities.insert(id.clone(), Box::new(storage));

        self.search_index
            .entry(entity_name)
            .or_default()
            .push(id.clone());

        if let Some(path_lower) = entity_path {
            self.search_index
                .entry(path_lower)
                .or_default()
                .push(id.clone());
        }

        self.search_index
            .entry(entity_type_str)
            .or_default()
            .push(id.clone());

        for value_lower in metadata_values {
            self.search_index
                .entry(value_lower)
                .or_default()
                .push(id.clone());
        }

        Ok(())
    }

    /// Add a general entity to the graph
    pub fn add_entity<E: Entity + 'static>(&mut self, entity: E) -> Result<()> {
        let id = entity.id().clone();

        // First, we'll try a direct cast if entity is already a concrete type
        let storage = if let Some(func) =
            (&entity as &dyn std::any::Any).downcast_ref::<FunctionEntity>()
        {
            EntityStorage::Function(func.clone())
        } else if let Some(typ) = (&entity as &dyn std::any::Any).downcast_ref::<TypeEntity>() {
            EntityStorage::Type(typ.clone())
        } else if let Some(module) = (&entity as &dyn std::any::Any).downcast_ref::<ModuleEntity>()
        {
            EntityStorage::Module(module.clone())
        } else if let Some(var) = (&entity as &dyn std::any::Any).downcast_ref::<VariableEntity>() {
            EntityStorage::Variable(var.clone())
        } else if let Some(domain) =
            (&entity as &dyn std::any::Any).downcast_ref::<DomainConceptEntity>()
        {
            EntityStorage::DomainConcept(domain.clone())
        } else if let Some(base) = (&entity as &dyn std::any::Any).downcast_ref::<BaseEntity>() {
            EntityStorage::Base(base.clone())
        } else {
            // If direct cast fails, fall back to the entity type check method
            match entity.entity_type() {
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
            }
        };

        let storage_ref = &storage;
        let entity_name = storage_ref.as_entity().name().to_lowercase();
        let entity_path = storage_ref.as_entity().path().map(|p| p.to_lowercase());
        let entity_type = storage_ref
            .as_entity()
            .entity_type()
            .to_string()
            .to_lowercase();

        let metadata_values: Vec<String> = storage_ref
            .as_entity()
            .metadata()
            .iter()
            .map(|(_key, v)| v.to_lowercase())
            .collect();

        self.entities.insert(id.clone(), Box::new(storage));

        self.search_index
            .entry(entity_name)
            .or_default()
            .push(id.clone());

        if let Some(path_lower) = entity_path {
            self.search_index
                .entry(path_lower)
                .or_default()
                .push(id.clone());
        }

        self.search_index
            .entry(entity_type)
            .or_default()
            .push(id.clone());

        for value_lower in metadata_values {
            self.search_index
                .entry(value_lower)
                .or_default()
                .push(id.clone());
        }

        Ok(())
    }

    /// Get an entity by its ID
    pub fn get_entity(&self, id: &EntityId) -> Option<&dyn Entity> {
        self.entities.get(id).map(|e| e.as_entity())
    }

    /// Get an entity by its ID with standardized error handling
    pub fn get_entity_result(&self, id: &EntityId) -> Result<&dyn Entity> {
        self.entities
            .get(id)
            .map(|e| e.as_entity())
            .ok_or_else(|| anyhow::anyhow!("Entity with ID {} not found", id.as_str()))
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
    /// Stores the relationship in the relationship store. The relationship
    /// will be automatically included in relationship_data during serialization.
    pub fn add_relationship(&mut self, relationship: Relationship) {
        self.relationship_store.add_relationship(relationship);
    }

    /// Create and add a relationship between entities
    ///
    /// Tries to locate both source and target entities in the graph.
    /// If the source entity doesn't exist, the relationship creation fails.
    /// If the target entity doesn't exist, it may be an external reference (like a standard library function),
    /// so we create a placeholder BaseEntity to represent it.
    pub fn create_relationship(
        &mut self,
        source_id: EntityId,
        target_id: EntityId,
        rel_type: RelationshipType,
    ) -> Result<()> {
        self.get_entity_result(&source_id)?;

        // Check if target entity exists, if not, create a placeholder
        if self.get_entity(&target_id).is_none() {
            // It might be a standard library or external reference
            // Create a placeholder BaseEntity for the target
            let target_name = target_id
                .as_str()
                .split("::")
                .last()
                .unwrap_or(target_id.as_str());
            let base_entity = crate::graph::entity::BaseEntity::new(
                target_id.clone(),
                target_name.to_string(),
                crate::graph::entity::EntityType::Function, // Assume it's a function for now
                None,                                       // No file path for external entities
            );

            // Add the placeholder entity to the graph
            self.add_entity(base_entity)?;
        }

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

    /// Find paths between entities using an iterative approach to avoid stack overflow
    pub fn find_paths(
        &self,
        from_id: &EntityId,
        to_id: &EntityId,
        max_depth: usize,
    ) -> Vec<Vec<&dyn Entity>> {
        let mut result_paths = Vec::new();

        let start_entity = match self.get_entity(from_id) {
            Some(entity) => entity,
            None => return Vec::new(),
        };

        // Stack of (current_id, current_path, visited_set)
        let mut stack = vec![(
            from_id.clone(),
            vec![start_entity],
            HashSet::<EntityId>::from([from_id.clone()]),
        )];

        while let Some((current_id, current_path, visited)) = stack.pop() {
            if &current_id == to_id {
                result_paths.push(current_path);
                continue;
            }

            if current_path.len() >= max_depth {
                continue;
            }

            let relationships = self
                .relationship_store
                .get_outgoing_relationships(&current_id);

            // Add relationships to stack in reverse order (so they're processed in the original order)
            for rel in relationships.into_iter().rev() {
                let next_id = &rel.target_id;

                if visited.contains(next_id) {
                    continue;
                }

                if let Some(next_entity) = self.get_entity(next_id) {
                    let mut new_path = current_path.clone();
                    new_path.push(next_entity);

                    let mut new_visited = visited.clone();
                    new_visited.insert(next_id.clone());

                    stack.push((next_id.clone(), new_path, new_visited));
                }
            }
        }

        result_paths
    }

    /// Get domain concepts
    pub fn get_domain_concepts(&self) -> Vec<&DomainConceptEntity> {
        self.get_entities_by_type(&EntityType::DomainConcept)
            .into_iter()
            .filter_map(|entity| {
                if let Some(boxed_storage) = self.entities.get(entity.id()) {
                    match &**boxed_storage {
                        EntityStorage::DomainConcept(domain_concept) => Some(domain_concept),
                        _ => None,
                    }
                } else {
                    None
                }
            })
            .collect()
    }

    /// Get the total number of relationships
    pub fn get_relationship_count(&self) -> usize {
        self.relationship_store.get_all_relationships().len()
    }

    /// Get all relationships
    /// Returns a vector of all relationships in the graph directly from the relationship store
    pub fn get_all_relationships(&self) -> Result<Vec<Relationship>> {
        Ok(self.relationship_store.get_all_relationships())
    }

    /// Prune entities and relationships associated with specific files
    ///
    /// This method removes all entities and their relationships that belong
    /// to the specified file paths. It's used to selectively update parts of
    /// the knowledge graph without rebuilding everything.
    pub fn prune(&self, modified_files: &[String]) -> anyhow::Result<()> {
        debug!("Pruning graph for {} modified files", modified_files.len());

        if let Some(db) = &self.database {
            // Remove entities and relationships for these files from the database
            db.remove_entities_and_relationships_by_files(modified_files)?;
            debug!("Successfully pruned entries for modified files");
            Ok(())
        } else {
            Err(anyhow::anyhow!(
                "No database connection available for pruning"
            ))
        }
    }

    /// Purge all entities and relationships from the graph
    ///
    /// This method completely clears the knowledge graph and its database.
    /// Use with caution, as it will remove all indexed data.
    pub fn purge(&self) -> anyhow::Result<()> {
        debug!("Purging entire knowledge graph");

        if let Some(db) = &self.database {
            // Remove all entities and relationships from the database
            db.purge_graph()?;
            debug!("Successfully purged knowledge graph");
            Ok(())
        } else {
            Err(anyhow::anyhow!(
                "No database connection available for purging"
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::entity::{
        BaseEntity, DomainConceptEntity, EntityId, EntityType, FunctionEntity, Parameter,
        Visibility,
    };
    use crate::graph::relationship::{Relationship, RelationshipType};

    #[test]
    fn test_add_entity() {
        let mut kg = KnowledgeGraph::new();

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

        let result = kg.add_entity(function);
        assert!(result.is_ok());

        assert_eq!(kg.entities.len(), 1);

        let entity = kg.get_entity(&id);
        assert!(entity.is_some());

        let entity = entity.unwrap();
        assert_eq!(entity.name(), "testFunction");
        assert!(matches!(entity.entity_type(), EntityType::Function));
        assert_eq!(entity.file_path().unwrap(), "test.rs");
    }

    #[test]
    fn test_add_boxed_entity_preserves_function_data() {
        let mut kg = KnowledgeGraph::new();

        let id = EntityId::new("test::function_with_params");
        let base = BaseEntity::new(
            id.clone(),
            "testFunctionWithParams".to_string(),
            EntityType::Function,
            Some("test.rs".to_string()),
        );

        let param1 = Parameter {
            name: "arg1".to_string(),
            type_annotation: Some("String".to_string()),
            default_value: None,
        };

        let param2 = Parameter {
            name: "arg2".to_string(),
            type_annotation: Some("i32".to_string()),
            default_value: Some("42".to_string()),
        };

        let function = FunctionEntity {
            base,
            parameters: vec![param1, param2],
            return_type: Some("bool".to_string()),
            visibility: Visibility::Public,
            is_async: true,
            is_static: false,
            is_constructor: false,
            is_abstract: false,
        };

        let boxed_entity: Box<dyn Entity> = Box::new(function);

        let result = kg.add_boxed_entity(boxed_entity);
        assert!(result.is_ok());

        let entity = kg.get_entity(&id);
        assert!(entity.is_some());

        let storage = kg.entities.get(&id).unwrap();
        match &**storage {
            EntityStorage::Function(func) => {
                assert_eq!(func.parameters.len(), 2);
                assert_eq!(func.parameters[0].name, "arg1");
                assert_eq!(
                    func.parameters[0].type_annotation,
                    Some("String".to_string())
                );
                assert_eq!(func.parameters[1].name, "arg2");
                assert_eq!(func.parameters[1].type_annotation, Some("i32".to_string()));
                assert_eq!(func.parameters[1].default_value, Some("42".to_string()));
                assert_eq!(func.return_type, Some("bool".to_string()));
                assert!(func.is_async);
                assert!(!func.is_static);
            }
            _ => panic!("Expected FunctionEntity but got a different entity type"),
        }
    }

    #[test]
    fn test_add_boxed_entity_preserves_type_data() {
        let mut kg = KnowledgeGraph::new();

        let id = EntityId::new("test::Class");
        let base = BaseEntity::new(
            id.clone(),
            "TestClass".to_string(),
            EntityType::Class,
            Some("test_class.rs".to_string()),
        );

        let field_id = EntityId::new("test::Class::field");
        let method_id = EntityId::new("test::Class::method");
        let supertype_id = EntityId::new("test::ParentClass");

        let type_entity = TypeEntity {
            base,
            fields: vec![field_id.clone()],
            methods: vec![method_id.clone()],
            supertypes: vec![supertype_id.clone()],
            visibility: Visibility::Public,
            is_abstract: true,
        };

        let boxed_entity: Box<dyn Entity> = Box::new(type_entity);

        let result = kg.add_boxed_entity(boxed_entity);
        assert!(result.is_ok());

        let entity = kg.get_entity(&id);
        assert!(entity.is_some());

        let storage = kg.entities.get(&id).unwrap();
        match &**storage {
            EntityStorage::Type(typ) => {
                assert_eq!(typ.fields.len(), 1);
                assert_eq!(typ.fields[0].as_str(), field_id.as_str());

                assert_eq!(typ.methods.len(), 1);
                assert_eq!(typ.methods[0].as_str(), method_id.as_str());

                assert_eq!(typ.supertypes.len(), 1);
                assert_eq!(typ.supertypes[0].as_str(), supertype_id.as_str());

                assert!(matches!(typ.visibility, Visibility::Public));
                assert!(typ.is_abstract);
            }
            _ => panic!("Expected TypeEntity but got a different entity type"),
        }
    }

    #[test]
    fn test_add_boxed_entity_wrong_type_fails() {
        let mut kg = KnowledgeGraph::new();

        let id = EntityId::new("test::mismatch");
        let base = BaseEntity::new(
            id.clone(),
            "MismatchEntity".to_string(),
            // Here's the mismatch - we're declaring it as a Function but using BaseEntity
            EntityType::Function,
            Some("test.rs".to_string()),
        );

        let entity: Box<dyn Entity> = Box::new(base);

        let result = kg.add_boxed_entity(entity);
        assert!(result.is_err());

        let err = result.unwrap_err();
        let err_msg = err.to_string();
        assert!(err_msg.contains("Expected FunctionEntity but downcast failed"));
    }

    #[test]
    fn test_add_relationship() {
        let mut kg = KnowledgeGraph::new();

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

        kg.add_entity(function1).unwrap();
        kg.add_entity(function2).unwrap();

        let rel_id = Relationship::generate_id(&id1, &id2, &RelationshipType::Calls);
        let relationship =
            Relationship::new(rel_id, id1.clone(), id2.clone(), RelationshipType::Calls);

        kg.add_relationship(relationship);

        assert_eq!(kg.get_relationship_count(), 1);

        let outgoing = kg.get_outgoing_relationships(&id1);
        assert_eq!(outgoing.len(), 1);
        assert!(matches!(
            outgoing[0].relationship_type,
            RelationshipType::Calls
        ));
        assert_eq!(outgoing[0].target_id.as_str(), id2.as_str());
    }

    #[test]
    fn test_get_entities_by_type() {
        let mut kg = KnowledgeGraph::new();

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

        kg.add_entity(function).unwrap();
        kg.add_entity(concept).unwrap();

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

        let paths = kg.find_paths(&id_a, &id_c, 1);
        assert_eq!(paths.len(), 0);

        assert!(kg
            .get_outgoing_relationships(&id_a)
            .iter()
            .all(|r| r.target_id != id_c));
    }

    #[test]
    fn test_domain_concepts() {
        let mut kg = KnowledgeGraph::new();

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

        kg.add_entity(concept).unwrap();

        let concepts = kg.get_domain_concepts();
        assert_eq!(concepts.len(), 1);
        assert_eq!(concepts[0].name(), "User");
        assert_eq!(concepts[0].attributes.len(), 2);

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

        kg.create_relationship(id.clone(), code_id.clone(), RelationshipType::RepresentedBy)
            .unwrap();

        let related = kg.get_related_entities(&id, Some(&RelationshipType::RepresentedBy));
        assert_eq!(related.len(), 1);
        assert_eq!(related[0].name(), "UserClass");
    }

    #[test]
    fn test_add_entity_duplicate() {
        let mut kg = KnowledgeGraph::new();

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

        kg.add_entity(function1).unwrap();

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

        kg.add_entity(function2).unwrap();

        assert_eq!(kg.entities.len(), 1);
        assert_eq!(kg.get_entity(&id).unwrap().name(), "duplicateFunc");
    }

    #[test]
    fn test_add_relationship_with_nonexistent_entity() {
        let mut kg = KnowledgeGraph::new();

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

        kg.add_entity(function_a).unwrap();

        let id_nonexistent = EntityId::new("NonExistent");

        let result = kg.create_relationship(
            id_a.clone(),
            id_nonexistent.clone(),
            RelationshipType::Calls,
        );

        assert!(result.is_ok());

        let nonexistent_entity = kg.get_entity(&id_nonexistent);
        assert!(nonexistent_entity.is_some());

        let related = kg.get_related_entities(&id_a, Some(&RelationshipType::Calls));
        assert_eq!(related.len(), 1);
        assert_eq!(related[0].id().as_str(), "NonExistent");
    }

    #[test]
    fn test_add_bidirectional_relationship() {
        let mut kg = KnowledgeGraph::new();

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
        kg.create_relationship(id_b.clone(), id_a.clone(), RelationshipType::Calls)
            .unwrap();

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

        kg.add_entity(function1).unwrap();
        kg.add_entity(function2).unwrap();
        kg.add_entity(class).unwrap();

        let all_functions = kg.get_entities_by_type(&EntityType::Function);
        assert_eq!(all_functions.len(), 2);

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

        let paths = kg.find_paths(&id_a, &id_c, 3);

        assert_eq!(paths.len(), 2);

        let path_names: Vec<Vec<String>> = paths
            .iter()
            .map(|path| path.iter().map(|e| e.name().to_string()).collect())
            .collect();

        let path1 = vec!["A".to_string(), "B".to_string(), "C".to_string()];
        let path2 = vec!["A".to_string(), "D".to_string(), "C".to_string()];

        assert!(path_names.contains(&path1) || path_names.contains(&path2));
    }

    #[test]
    fn test_domain_concept_relationships() {
        let mut kg = KnowledgeGraph::new();

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

        kg.add_entity(user).unwrap();
        kg.add_entity(order).unwrap();

        kg.create_relationship(
            user_id.clone(),
            order_id.clone(),
            RelationshipType::RelatesTo,
        )
        .unwrap();

        let outgoing = kg.get_outgoing_relationships(&user_id);
        assert_eq!(outgoing.len(), 1);
        assert_eq!(outgoing[0].target_id.0, "domain::Order");
        assert!(matches!(
            outgoing[0].relationship_type,
            RelationshipType::RelatesTo
        ));

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

        let order_rels = kg.get_outgoing_relationships(&order_id);
        assert_eq!(order_rels.len(), 1);
        assert_eq!(order_rels[0].target_id.0, "func::place_order");
        assert!(matches!(
            order_rels[0].relationship_type,
            RelationshipType::RepresentedBy
        ));
    }

    #[test]
    fn test_large_graph() {
        let mut kg = KnowledgeGraph::new();

        for i in 0..1000 {
            let id = EntityId::new(&format!("entity{}", i));
            let base = BaseEntity::new(
                id.clone(),
                format!("Entity{}", i),
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

            kg.add_entity(function).unwrap();

            if i > 0 {
                let source_id = EntityId::new(&format!("entity{}", i - 1));
                let target_id = EntityId::new(&format!("entity{}", i));
                kg.create_relationship(source_id, target_id, RelationshipType::Calls)
                    .unwrap();
            }
        }

        let entity500 = kg.get_entity(&EntityId::new("entity500"));
        assert!(entity500.is_some());
        assert_eq!(entity500.unwrap().name(), "Entity500");

        let paths_small = kg.find_paths(&EntityId::new("entity0"), &EntityId::new("entity10"), 15);
        assert_eq!(paths_small.len(), 1);
        assert_eq!(paths_small[0].len(), 11);

        let result = kg.create_relationship(
            EntityId::new("entity999"),
            EntityId::new("nonexistent"),
            RelationshipType::Calls,
        );
        assert!(result.is_ok());

        let nonexistent = kg.get_entity(&EntityId::new("nonexistent"));
        assert!(nonexistent.is_some());
    }
}
