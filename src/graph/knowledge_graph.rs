use anyhow::Result;
use petgraph::graph::{DiGraph, NodeIndex};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

use super::entity::{
    BaseEntity, DomainConceptEntity, Entity, EntityId, EntityType,
    FunctionEntity, ModuleEntity, TypeEntity, VariableEntity,
};
use super::relationship::{Relationship, RelationshipStore, RelationshipType};

use crate::parser::language_support::{CallReference, FunctionDefinition};

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
    
    // Legacy storage for backwards compatibility
    #[serde(skip)]
    functions: HashMap<String, FunctionDefinition>,
    #[serde(skip)]
    call_graph: DiGraph<String, ()>,
    call_edges: Vec<(String, String)>,
    
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

impl KnowledgeGraph {
    pub fn new() -> Self {
        Self {
            entities: HashMap::new(),
            relationship_store: RelationshipStore::new(),
            relationship_data: Vec::new(),
            functions: HashMap::new(),
            call_graph: DiGraph::new(),
            call_edges: Vec::new(),
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
    
    /// Check if an entity exists by ID
    pub fn has_entity(&self, id: &EntityId) -> bool {
        self.entities.contains_key(id)
    }
    
    /// Get the number of entities
    pub fn get_entity_count(&self) -> usize {
        self.entities.len()
    }
    
    /// Get all entity IDs for debugging
    pub fn get_all_entity_ids(&self) -> Vec<&EntityId> {
        self.entities.keys().collect()
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

    // Legacy methods for compatibility with existing code
    
    pub fn get_functions(&self) -> &HashMap<String, FunctionDefinition> {
        &self.functions
    }

    pub fn add_function(&mut self, func: &FunctionDefinition) {
        // Legacy function storage
        let key = format!("{}::{}", func.file_path, func.name);
        self.functions.insert(key.clone(), func.clone());
        
        // Convert to new entity model
        // Use the same key format as in commands/index.rs
        let entity_id = EntityId::new(&key);
        let entity_type = match func.kind {
            crate::parser::language_support::FunctionKind::Function => EntityType::Function,
            crate::parser::language_support::FunctionKind::Method => EntityType::Method,
            crate::parser::language_support::FunctionKind::Constructor => EntityType::Method,
            _ => EntityType::Function,
        };
        
        let base = BaseEntity::new(
            entity_id.clone(),
            func.name.clone(),
            entity_type,
            Some(func.file_path.clone()),
        );
        
        let function_entity = FunctionEntity {
            base,
            parameters: func.parameters.clone(),
            return_type: None, // Not in the legacy model
            visibility: super::entity::Visibility::Default,
            is_async: false,   // Not in the legacy model
            is_static: false,  // Not in the legacy model
            is_constructor: func.kind == crate::parser::language_support::FunctionKind::Constructor,
            is_abstract: false, // Not in the legacy model
        };
        
        // Ignore error since this is just compatibility code
        let _ = self.add_entity(function_entity);
    }

    pub fn add_call(&mut self, caller: &FunctionDefinition, call: &CallReference) {
        let caller_key = format!("{}::{}", caller.file_path, caller.name);
        let callee_key = call
            .fully_qualified_name
            .clone()
            .unwrap_or_else(|| call.callee_name.clone());

        self.call_edges.push((caller_key.clone(), callee_key.clone()));
        
        // Convert to new relationship model
        let source_id = EntityId::new(&caller_key);
        let target_id = EntityId::new(&callee_key);
        
        // Only create the relationship if both entities exist
        if self.entities.contains_key(&source_id) && self.entities.contains_key(&target_id) {
            let _ = self.create_relationship(
                source_id, 
                target_id,
                RelationshipType::Calls,
            );
        }
    }

    pub fn add_edge(&mut self, from: NodeIndex, to: NodeIndex) {
        self.call_graph.add_edge(from, to, ());
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
        
        // Legacy support: Reconstruct the functions map
        for (id, storage) in &graph.entities {
            if let EntityStorage::Function(func) = &**storage {
                if let Some(file_path) = &func.base.file_path {
                    let name = func.base.name.clone();
                    
                    // Create legacy function definition
                    let function_def = FunctionDefinition {
                        name: name.clone(),
                        file_path: file_path.clone(),
                        kind: match func.base.entity_type {
                            EntityType::Method => {
                                if func.is_constructor {
                                    crate::parser::language_support::FunctionKind::Constructor
                                } else {
                                    crate::parser::language_support::FunctionKind::Method
                                }
                            },
                            _ => crate::parser::language_support::FunctionKind::Function,
                        },
                        visibility: match func.visibility {
                            super::entity::Visibility::Public => crate::parser::language_support::Visibility::Public,
                            super::entity::Visibility::Private => crate::parser::language_support::Visibility::Private,
                            super::entity::Visibility::Protected => crate::parser::language_support::Visibility::Protected,
                            super::entity::Visibility::Package => crate::parser::language_support::Visibility::Default,
                            super::entity::Visibility::Internal => crate::parser::language_support::Visibility::Default,
                            super::entity::Visibility::Default => crate::parser::language_support::Visibility::Default,
                        },
                        location: crate::parser::language_support::Location {
                            start: match &func.base.location {
                                Some(loc) => crate::parser::language_support::Position {
                                    line: loc.start.line,
                                    column: loc.start.column,
                                    offset: loc.start.offset,
                                },
                                None => crate::parser::language_support::Position {
                                    line: 0,
                                    column: 0,
                                    offset: 0,
                                },
                            },
                            end: match &func.base.location {
                                Some(loc) => crate::parser::language_support::Position {
                                    line: loc.end.line,
                                    column: loc.end.column,
                                    offset: loc.end.offset,
                                },
                                None => crate::parser::language_support::Position {
                                    line: 0,
                                    column: 0,
                                    offset: 0,
                                },
                            },
                        },
                        containing_type: func.base.containing_entity.as_ref().map(|id| id.as_str().to_string()),
                        parameters: func.parameters.clone(),
                    };
                    
                    let key = format!("{}::{}", file_path, name);
                    graph.functions.insert(key, function_def);
                }
            }
        }
        
        Ok(graph)
    }

    pub fn get_function(&self, file_path: &str, name: &str) -> Option<&FunctionDefinition> {
        self.functions.get(&format!("{}::{}", file_path, name))
    }

    pub fn get_callers(&self, func: &FunctionDefinition) -> Vec<&FunctionDefinition> {
        let key = format!("{}::{}", func.file_path, func.name);
        self.call_edges
            .iter()
            .filter(|(_, callee)| callee == &key)
            .filter_map(|(caller, _)| self.functions.get(caller))
            .collect()
    }

    pub fn get_callees(&self, func: &FunctionDefinition) -> Vec<&FunctionDefinition> {
        let key = format!("{}::{}", func.file_path, func.name);
        self.call_edges
            .iter()
            .filter(|(caller, _)| caller == &key)
            .filter_map(|(_, callee)| self.functions.get(callee))
            .collect()
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

// Visibility conversion functions (commented out since they're no longer needed)
// impl From<&crate::parser::language_support::Visibility> for super::entity::Visibility {
//     fn from(vis: &crate::parser::language_support::Visibility) -> Self {
//         match vis {
//             crate::parser::language_support::Visibility::Public => super::entity::Visibility::Public,
//             crate::parser::language_support::Visibility::Private => super::entity::Visibility::Private,
//             crate::parser::language_support::Visibility::Protected => super::entity::Visibility::Protected,
//             crate::parser::language_support::Visibility::Default => super::entity::Visibility::Default,
//         }
//     }
// }

// Helper function for converting from owned value (commented out since it's no longer needed)
// pub fn visibility_from(vis: crate::parser::language_support::Visibility) -> super::entity::Visibility {
//     match vis {
//         crate::parser::language_support::Visibility::Public => super::entity::Visibility::Public,
//         crate::parser::language_support::Visibility::Private => super::entity::Visibility::Private,
//         crate::parser::language_support::Visibility::Protected => super::entity::Visibility::Protected,
//         crate::parser::language_support::Visibility::Default => super::entity::Visibility::Default,
//     }
// }
