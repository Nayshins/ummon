use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::hash::Hash;

/// Position in a source file
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Position {
    pub line: usize,
    pub column: usize,
    pub offset: usize,
}

/// Location range in a source file
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Location {
    pub start: Position,
    pub end: Position,
}

/// Visibility level for code entities
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Visibility {
    Public,
    Private,
    Protected,
    Package,
    Internal,
    Default,
}

/// Parameter in a function or method
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Parameter {
    pub name: String,
    pub type_annotation: Option<String>,
    pub default_value: Option<String>,
}

/// Unique identifier for an entity
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct EntityId(pub String);

impl EntityId {
    pub fn new(id: &str) -> Self {
        EntityId(id.to_string())
    }
    
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Entity type enumeration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum EntityType {
    Function,
    Method,
    Class,
    Interface,
    Trait,
    Struct,
    Enum,
    Module,
    File,
    Variable,
    Field,
    Constant,
    DomainConcept,
    Type,
    Other(String),
}

/// Base trait for all graph entities
pub trait Entity {
    fn id(&self) -> &EntityId;
    fn name(&self) -> &str;
    fn entity_type(&self) -> EntityType;
    fn location(&self) -> Option<&Location>;
    fn metadata(&self) -> &HashMap<String, String>;
    #[allow(dead_code)]
    fn metadata_mut(&mut self) -> &mut HashMap<String, String>;
}

/// Common properties for all entity implementations
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BaseEntity {
    pub id: EntityId,
    pub name: String,
    pub entity_type: EntityType,
    pub location: Option<Location>,
    pub file_path: Option<String>,
    pub containing_entity: Option<EntityId>,
    pub documentation: Option<String>,
    pub metadata: HashMap<String, String>,
}

impl BaseEntity {
    pub fn new(
        id: EntityId,
        name: String,
        entity_type: EntityType,
        file_path: Option<String>,
    ) -> Self {
        Self {
            id,
            name,
            entity_type,
            location: None,
            file_path,
            containing_entity: None,
            documentation: None,
            metadata: HashMap::new(),
        }
    }
}

impl Entity for BaseEntity {
    fn id(&self) -> &EntityId {
        &self.id
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn entity_type(&self) -> EntityType {
        self.entity_type.clone()
    }

    fn location(&self) -> Option<&Location> {
        self.location.as_ref()
    }

    fn metadata(&self) -> &HashMap<String, String> {
        &self.metadata
    }
    
    fn metadata_mut(&mut self) -> &mut HashMap<String, String> {
        &mut self.metadata
    }
}

/// Function or method definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionEntity {
    pub base: BaseEntity,
    pub parameters: Vec<Parameter>,
    pub return_type: Option<String>,
    pub visibility: Visibility,
    pub is_async: bool,
    pub is_static: bool,
    pub is_constructor: bool,
    pub is_abstract: bool,
}

impl Entity for FunctionEntity {
    fn id(&self) -> &EntityId {
        &self.base.id
    }

    fn name(&self) -> &str {
        &self.base.name
    }

    fn entity_type(&self) -> EntityType {
        self.base.entity_type.clone()
    }

    fn location(&self) -> Option<&Location> {
        self.base.location.as_ref()
    }

    fn metadata(&self) -> &HashMap<String, String> {
        &self.base.metadata
    }
    
    fn metadata_mut(&mut self) -> &mut HashMap<String, String> {
        &mut self.base.metadata
    }
}

/// Class, struct, or interface definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeEntity {
    pub base: BaseEntity,
    pub fields: Vec<EntityId>,
    pub methods: Vec<EntityId>,
    pub supertypes: Vec<EntityId>,
    pub visibility: Visibility,
    pub is_abstract: bool,
}

impl Entity for TypeEntity {
    fn id(&self) -> &EntityId {
        &self.base.id
    }

    fn name(&self) -> &str {
        &self.base.name
    }

    fn entity_type(&self) -> EntityType {
        self.base.entity_type.clone()
    }

    fn location(&self) -> Option<&Location> {
        self.base.location.as_ref()
    }

    fn metadata(&self) -> &HashMap<String, String> {
        &self.base.metadata
    }
    
    fn metadata_mut(&mut self) -> &mut HashMap<String, String> {
        &mut self.base.metadata
    }
}

/// Module or file representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleEntity {
    pub base: BaseEntity,
    pub path: String,
    pub children: Vec<EntityId>,
    pub imports: Vec<String>,
}

impl Entity for ModuleEntity {
    fn id(&self) -> &EntityId {
        &self.base.id
    }

    fn name(&self) -> &str {
        &self.base.name
    }

    fn entity_type(&self) -> EntityType {
        self.base.entity_type.clone()
    }

    fn location(&self) -> Option<&Location> {
        self.base.location.as_ref()
    }

    fn metadata(&self) -> &HashMap<String, String> {
        &self.base.metadata
    }
    
    fn metadata_mut(&mut self) -> &mut HashMap<String, String> {
        &mut self.base.metadata
    }
}

/// Variable, field, or constant definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VariableEntity {
    pub base: BaseEntity,
    pub type_annotation: Option<String>,
    pub visibility: Visibility,
    pub is_const: bool,
    pub is_static: bool,
}

impl Entity for VariableEntity {
    fn id(&self) -> &EntityId {
        &self.base.id
    }

    fn name(&self) -> &str {
        &self.base.name
    }

    fn entity_type(&self) -> EntityType {
        self.base.entity_type.clone()
    }

    fn location(&self) -> Option<&Location> {
        self.base.location.as_ref()
    }

    fn metadata(&self) -> &HashMap<String, String> {
        &self.base.metadata
    }
    
    fn metadata_mut(&mut self) -> &mut HashMap<String, String> {
        &mut self.base.metadata
    }
}

/// Business domain concept
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DomainConceptEntity {
    pub base: BaseEntity,
    pub attributes: Vec<String>,
    pub description: Option<String>,
    pub confidence: f32,
}

impl Entity for DomainConceptEntity {
    fn id(&self) -> &EntityId {
        &self.base.id
    }

    fn name(&self) -> &str {
        &self.base.name
    }

    fn entity_type(&self) -> EntityType {
        self.base.entity_type.clone()
    }

    fn location(&self) -> Option<&Location> {
        self.base.location.as_ref()
    }

    fn metadata(&self) -> &HashMap<String, String> {
        &self.base.metadata
    }
    
    fn metadata_mut(&mut self) -> &mut HashMap<String, String> {
        &mut self.base.metadata
    }
}

/// A boxed entity type for storing heterogeneous entities
#[allow(dead_code)]
pub type BoxedEntity = Box<dyn Entity + Send + Sync>;

