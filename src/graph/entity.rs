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

impl std::fmt::Display for EntityId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
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

impl std::fmt::Display for EntityType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EntityType::Function => write!(f, "Function"),
            EntityType::Method => write!(f, "Method"),
            EntityType::Class => write!(f, "Class"),
            EntityType::Interface => write!(f, "Interface"),
            EntityType::Trait => write!(f, "Trait"),
            EntityType::Struct => write!(f, "Struct"),
            EntityType::Enum => write!(f, "Enum"),
            EntityType::Module => write!(f, "Module"),
            EntityType::File => write!(f, "File"),
            EntityType::Variable => write!(f, "Variable"),
            EntityType::Field => write!(f, "Field"),
            EntityType::Constant => write!(f, "Constant"),
            EntityType::DomainConcept => write!(f, "DomainConcept"),
            EntityType::Type => write!(f, "Type"),
            EntityType::Other(s) => write!(f, "Other({})", s),
        }
    }
}

/// Base trait for all graph entities
pub trait Entity {
    fn id(&self) -> &EntityId;
    fn name(&self) -> &str;
    fn entity_type(&self) -> EntityType;
    #[allow(dead_code)]
    fn location(&self) -> Option<&Location>;
    fn file_path(&self) -> Option<&String>;
    #[allow(dead_code)]
    fn metadata(&self) -> &HashMap<String, String>;

    // Helper methods for MCP server
    fn path(&self) -> Option<&str> {
        self.file_path().map(|s| s.as_str())
    }
    #[allow(dead_code)]
    fn metadata_mut(&mut self) -> &mut HashMap<String, String>;

    /// Serialize the entity data to a string for database storage
    /// Default implementation provides empty JSON object
    #[allow(dead_code)]
    fn serialize_data(&self) -> anyhow::Result<String> {
        Ok("{}".to_string())
    }
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

    fn file_path(&self) -> Option<&String> {
        self.file_path.as_ref()
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

/// Serializable data for function entities
#[derive(serde::Serialize, serde::Deserialize)]
pub struct FunctionEntityData {
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

    fn file_path(&self) -> Option<&String> {
        self.base.file_path.as_ref()
    }

    fn metadata(&self) -> &HashMap<String, String> {
        &self.base.metadata
    }

    fn metadata_mut(&mut self) -> &mut HashMap<String, String> {
        &mut self.base.metadata
    }

    fn serialize_data(&self) -> anyhow::Result<String> {
        let data = FunctionEntityData {
            parameters: self.parameters.clone(),
            return_type: self.return_type.clone(),
            visibility: self.visibility.clone(),
            is_async: self.is_async,
            is_static: self.is_static,
            is_constructor: self.is_constructor,
            is_abstract: self.is_abstract,
        };
        serde_json::to_string(&data).map_err(Into::into)
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

/// Serializable data for type entities
#[derive(serde::Serialize, serde::Deserialize)]
pub struct TypeEntityData {
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

    fn file_path(&self) -> Option<&String> {
        self.base.file_path.as_ref()
    }

    fn metadata(&self) -> &HashMap<String, String> {
        &self.base.metadata
    }

    fn metadata_mut(&mut self) -> &mut HashMap<String, String> {
        &mut self.base.metadata
    }

    fn serialize_data(&self) -> anyhow::Result<String> {
        let data = TypeEntityData {
            fields: self.fields.clone(),
            methods: self.methods.clone(),
            supertypes: self.supertypes.clone(),
            visibility: self.visibility.clone(),
            is_abstract: self.is_abstract,
        };
        serde_json::to_string(&data).map_err(Into::into)
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

/// Serializable data for module entities
#[derive(serde::Serialize, serde::Deserialize)]
pub struct ModuleEntityData {
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

    fn file_path(&self) -> Option<&String> {
        self.base.file_path.as_ref()
    }

    fn metadata(&self) -> &HashMap<String, String> {
        &self.base.metadata
    }

    fn metadata_mut(&mut self) -> &mut HashMap<String, String> {
        &mut self.base.metadata
    }

    fn serialize_data(&self) -> anyhow::Result<String> {
        let data = ModuleEntityData {
            path: self.path.clone(),
            children: self.children.clone(),
            imports: self.imports.clone(),
        };
        serde_json::to_string(&data).map_err(Into::into)
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

/// Serializable data for variable entities
#[derive(serde::Serialize, serde::Deserialize)]
pub struct VariableEntityData {
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

    fn file_path(&self) -> Option<&String> {
        self.base.file_path.as_ref()
    }

    fn metadata(&self) -> &HashMap<String, String> {
        &self.base.metadata
    }

    fn metadata_mut(&mut self) -> &mut HashMap<String, String> {
        &mut self.base.metadata
    }

    fn serialize_data(&self) -> anyhow::Result<String> {
        let data = VariableEntityData {
            type_annotation: self.type_annotation.clone(),
            visibility: self.visibility.clone(),
            is_const: self.is_const,
            is_static: self.is_static,
        };
        serde_json::to_string(&data).map_err(Into::into)
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

/// Serializable data for domain concept entities
#[derive(serde::Serialize, serde::Deserialize)]
pub struct DomainConceptEntityData {
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

    fn file_path(&self) -> Option<&String> {
        self.base.file_path.as_ref()
    }

    fn metadata(&self) -> &HashMap<String, String> {
        &self.base.metadata
    }

    fn metadata_mut(&mut self) -> &mut HashMap<String, String> {
        &mut self.base.metadata
    }

    fn serialize_data(&self) -> anyhow::Result<String> {
        let data = DomainConceptEntityData {
            attributes: self.attributes.clone(),
            description: self.description.clone(),
            confidence: self.confidence,
        };
        serde_json::to_string(&data).map_err(Into::into)
    }
}

/// A boxed entity type for storing heterogeneous entities
#[allow(dead_code)]
pub type BoxedEntity = Box<dyn Entity + Send + Sync>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_entity_id() {
        let id = EntityId::new("test::id");
        assert_eq!(id.0, "test::id");
        assert_eq!(id.as_str(), "test::id");
    }

    #[test]
    fn test_base_entity() {
        // Create a base entity
        let id = EntityId::new("test::entity");
        let entity = BaseEntity::new(
            id.clone(),
            "TestEntity".to_string(),
            EntityType::Function,
            Some("test.rs".to_string()),
        );

        // Check basic properties
        assert_eq!(entity.id().0, "test::entity");
        assert_eq!(entity.name(), "TestEntity");
        assert!(matches!(entity.entity_type(), EntityType::Function));
        assert_eq!(entity.file_path, Some("test.rs".to_string()));
        assert_eq!(entity.location(), None);
        assert!(entity.metadata().is_empty());

        // Test metadata operations
        let mut entity_mut = entity.clone();
        entity_mut
            .metadata_mut()
            .insert("key".to_string(), "value".to_string());
        assert_eq!(entity_mut.metadata().len(), 1);
        assert_eq!(entity_mut.metadata().get("key"), Some(&"value".to_string()));
    }

    #[test]
    fn test_function_entity() {
        // Create a function entity
        let id = EntityId::new("test::func");
        let base = BaseEntity::new(
            id,
            "testFunc".to_string(),
            EntityType::Function,
            Some("test.rs".to_string()),
        );

        let param = Parameter {
            name: "param1".to_string(),
            type_annotation: Some("String".to_string()),
            default_value: None,
        };

        let function = FunctionEntity {
            base,
            parameters: vec![param],
            return_type: Some("bool".to_string()),
            visibility: Visibility::Public,
            is_async: true,
            is_static: false,
            is_constructor: false,
            is_abstract: false,
        };

        // Check entity properties
        assert_eq!(function.name(), "testFunc");
        assert!(matches!(function.entity_type(), EntityType::Function));
        assert_eq!(function.parameters.len(), 1);
        assert_eq!(function.parameters[0].name, "param1");
        assert_eq!(
            function.parameters[0].type_annotation,
            Some("String".to_string())
        );
        assert_eq!(function.return_type, Some("bool".to_string()));
        assert!(matches!(function.visibility, Visibility::Public));
        assert!(function.is_async);
        assert!(!function.is_static);
    }

    #[test]
    fn test_type_entity() {
        // Create a type entity
        let id = EntityId::new("test::class");
        let base = BaseEntity::new(
            id,
            "TestClass".to_string(),
            EntityType::Class,
            Some("test.rs".to_string()),
        );

        let method_id = EntityId::new("test::class::method");
        let field_id = EntityId::new("test::class::field");
        let supertype_id = EntityId::new("test::parentclass");

        let type_entity = TypeEntity {
            base,
            fields: vec![field_id],
            methods: vec![method_id],
            supertypes: vec![supertype_id],
            visibility: Visibility::Public,
            is_abstract: false,
        };

        // Check entity properties
        assert_eq!(type_entity.name(), "TestClass");
        assert!(matches!(type_entity.entity_type(), EntityType::Class));
        assert_eq!(type_entity.fields.len(), 1);
        assert_eq!(type_entity.methods.len(), 1);
        assert_eq!(type_entity.supertypes.len(), 1);
        assert!(matches!(type_entity.visibility, Visibility::Public));
        assert!(!type_entity.is_abstract);
    }

    #[test]
    fn test_module_entity() {
        // Create a module entity
        let id = EntityId::new("test::module");
        let base = BaseEntity::new(
            id,
            "test_module".to_string(),
            EntityType::Module,
            Some("test_module.rs".to_string()),
        );

        let child_id = EntityId::new("test::module::function");

        let module = ModuleEntity {
            base,
            path: "src/test_module.rs".to_string(),
            children: vec![child_id],
            imports: vec!["std::collections::HashMap".to_string()],
        };

        // Check entity properties
        assert_eq!(module.name(), "test_module");
        assert!(matches!(module.entity_type(), EntityType::Module));
        assert_eq!(module.path, "src/test_module.rs");
        assert_eq!(module.children.len(), 1);
        assert_eq!(module.imports.len(), 1);
        assert_eq!(module.imports[0], "std::collections::HashMap");
    }

    #[test]
    fn test_variable_entity() {
        // Create a variable entity
        let id = EntityId::new("test::var");
        let base = BaseEntity::new(
            id,
            "test_var".to_string(),
            EntityType::Variable,
            Some("test.rs".to_string()),
        );

        let variable = VariableEntity {
            base,
            type_annotation: Some("i32".to_string()),
            visibility: Visibility::Private,
            is_const: true,
            is_static: false,
        };

        // Check entity properties
        assert_eq!(variable.name(), "test_var");
        assert!(matches!(variable.entity_type(), EntityType::Variable));
        assert_eq!(variable.type_annotation, Some("i32".to_string()));
        assert!(matches!(variable.visibility, Visibility::Private));
        assert!(variable.is_const);
        assert!(!variable.is_static);
    }

    #[test]
    fn test_domain_concept_entity() {
        // Create a domain concept entity
        let id = EntityId::new("domain::User");
        let base = BaseEntity::new(id, "User".to_string(), EntityType::DomainConcept, None);

        let concept = DomainConceptEntity {
            base,
            attributes: vec!["username".to_string(), "email".to_string()],
            description: Some("A user in the system".to_string()),
            confidence: 0.95,
        };

        // Check entity properties
        assert_eq!(concept.name(), "User");
        assert!(matches!(concept.entity_type(), EntityType::DomainConcept));
        assert_eq!(concept.attributes.len(), 2);
        assert_eq!(concept.attributes[0], "username");
        assert_eq!(concept.attributes[1], "email");
        assert_eq!(
            concept.description,
            Some("A user in the system".to_string())
        );
        assert_eq!(concept.confidence, 0.95);
    }

    #[test]
    fn test_file_path_accessor() {
        // Test the new file_path method on all entity types

        // Base entity
        let id = EntityId::new("test::base");
        let base = BaseEntity::new(
            id.clone(),
            "TestBase".to_string(),
            EntityType::Other("Test".to_string()),
            Some("test.rs".to_string()),
        );

        assert_eq!(base.file_path(), Some(&"test.rs".to_string()));

        // Function entity
        let id = EntityId::new("test::func");
        let base = BaseEntity::new(
            id,
            "testFunc".to_string(),
            EntityType::Function,
            Some("function.rs".to_string()),
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

        assert_eq!(function.file_path(), Some(&"function.rs".to_string()));

        // Domain concept with no file path
        let id = EntityId::new("domain::User");
        let base = BaseEntity::new(id, "User".to_string(), EntityType::DomainConcept, None);

        let concept = DomainConceptEntity {
            base,
            attributes: vec![],
            description: None,
            confidence: 0.0,
        };

        assert_eq!(concept.file_path(), None);
    }

    #[test]
    fn test_entity_type_conversion() {
        // Test different entity types
        let types = vec![
            EntityType::Function,
            EntityType::Method,
            EntityType::Class,
            EntityType::Interface,
            EntityType::Trait,
            EntityType::Struct,
            EntityType::Enum,
            EntityType::Module,
            EntityType::File,
            EntityType::Variable,
            EntityType::Field,
            EntityType::Constant,
            EntityType::DomainConcept,
            EntityType::Type,
            EntityType::Other("CustomType".to_string()),
        ];

        for entity_type in types {
            let id = EntityId::new("test");
            let base = BaseEntity::new(id.clone(), "Test".to_string(), entity_type.clone(), None);

            // Entity type should be preserved
            assert_eq!(
                format!("{:?}", base.entity_type()),
                format!("{:?}", entity_type)
            );
        }
    }
}
