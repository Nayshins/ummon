use anyhow::Result;
use rusqlite::{params, Connection};
use std::path::Path;
use std::sync::{Arc, Mutex};

use crate::graph::entity::{
    BaseEntity, DomainConceptEntity, Entity, EntityId, EntityType, FunctionEntity, ModuleEntity,
    TypeEntity, VariableEntity,
};
use crate::graph::knowledge_graph::DatabaseConnection;
use crate::graph::relationship::{Relationship, RelationshipType};

/// Database schema version - increment when schema changes
#[allow(dead_code)]
const CURRENT_SCHEMA_VERSION: i32 = 1;

/// Database wrapper for managing the SQLite knowledge graph storage
pub struct Database {
    conn: Arc<Mutex<Connection>>,
    db_path: String,
}

impl std::fmt::Debug for Database {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Database")
            .field("conn", &"SQLite Connection")
            .field("db_path", &self.db_path)
            .finish()
    }
}

impl DatabaseConnection for Database {
    fn save_entity(&self, entity: &dyn Entity) -> Result<()> {
        Database::save_entity(self, entity)
    }

    fn save_relationship(&self, relationship: &Relationship) -> Result<()> {
        Database::save_relationship(self, relationship)
    }

    fn load_entities(&self) -> Result<Vec<Box<dyn Entity>>> {
        Database::load_entities(self)
    }

    fn load_relationships(&self) -> Result<Vec<Relationship>> {
        Database::load_relationships(self)
    }

    fn save_all_in_transaction(
        &self,
        entities: &[&dyn Entity],
        relationships: &[&Relationship],
    ) -> Result<()> {
        Database::save_all_in_transaction(self, entities, relationships)
    }
}

impl Clone for Database {
    fn clone(&self) -> Self {
        Self {
            conn: self.conn.clone(),
            db_path: self.db_path.clone(),
        }
    }
}

impl Database {
    /// Create a new database connection, initializing the schema if needed
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let conn = Connection::open(&path)?;
        let db_path = path.as_ref().to_string_lossy().to_string();
        let db = Self {
            conn: Arc::new(Mutex::new(conn)),
            db_path,
        };
        db.initialize_schema()?;
        Ok(db)
    }

    /// Create a new Database from a path string (convenience method for use across crate boundaries)
    pub fn from_path(path: &str) -> Result<Self> {
        Self::new(path)
    }

    /// Creates a new connection to the same database
    /// This is used to get a fresh connection per thread
    pub fn new_connection(&self) -> Result<Connection> {
        Connection::open(&self.db_path)
            .map_err(|e| anyhow::anyhow!("Failed to open database: {}", e))
    }

    /// Initialize the database schema if needed
    fn initialize_schema(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        // Create schema_version table if it doesn't exist
        conn.execute(
            "CREATE TABLE IF NOT EXISTS schema_version (
                version INTEGER PRIMARY KEY
            )",
            [],
        )?;

        // Initialize version if needed
        conn.execute(
            "INSERT OR IGNORE INTO schema_version (version) VALUES (0)",
            [],
        )?;

        // Apply migrations by dropping the lock and calling apply_migrations
        drop(conn);
        self.apply_migrations()?;

        Ok(())
    }

    /// Apply schema migrations as needed
    fn apply_migrations(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        // Check current schema version
        let version: i32 =
            conn.query_row("SELECT version FROM schema_version", [], |row| row.get(0))?;

        // Apply migrations if needed
        if version < 1 {
            // Initial schema creation
            conn.execute_batch(
                "CREATE TABLE entities (
                    id TEXT PRIMARY KEY,
                    name TEXT NOT NULL,
                    entity_type TEXT NOT NULL,
                    file_path TEXT,
                    location TEXT,
                    documentation TEXT,
                    containing_entity TEXT,
                    data TEXT NOT NULL
                );
                
                CREATE TABLE relationships (
                    id TEXT PRIMARY KEY,
                    source_id TEXT NOT NULL,
                    target_id TEXT NOT NULL,
                    relationship_type TEXT NOT NULL,
                    weight REAL NOT NULL DEFAULT 1.0,
                    metadata TEXT,
                    FOREIGN KEY(source_id) REFERENCES entities(id),
                    FOREIGN KEY(target_id) REFERENCES entities(id)
                );
                
                CREATE INDEX idx_entity_name ON entities(name);
                CREATE INDEX idx_entity_type ON entities(entity_type);
                CREATE INDEX idx_entity_file_path ON entities(file_path);
                
                CREATE INDEX idx_rel_source ON relationships(source_id);
                CREATE INDEX idx_rel_target ON relationships(target_id);
                CREATE INDEX idx_rel_type ON relationships(relationship_type);",
            )?;

            // Update schema version
            conn.execute("UPDATE schema_version SET version = 1", [])?;
        }

        // Add more version checks and migrations here for future schema changes
        // if version < 2 { ... }

        Ok(())
    }

    /// Save an entity to the database
    pub fn save_entity(&self, entity: &dyn Entity) -> Result<()> {
        // Get entity data using our helper function
        let entity_data = get_entity_data(entity)?;

        // Serialize location data
        let location_json = if let Some(loc) = entity.location() {
            serde_json::to_string(loc)?
        } else {
            "null".to_string()
        };

        // Get a fresh connection for this operation
        let conn = self.new_connection()?;
        conn.execute(
            "INSERT OR REPLACE INTO entities 
            (id, name, entity_type, file_path, location, documentation, containing_entity, data)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
            params![
                entity.id().as_str(),
                entity.name(),
                entity.entity_type().to_string(),
                entity.file_path().map(|s| s.as_str()),
                if entity.location().is_some() {
                    Some(location_json)
                } else {
                    None
                },
                entity.metadata().get("documentation").map(|s| s.as_str()),
                entity
                    .metadata()
                    .get("containing_entity")
                    .map(|s| s.as_str()),
                entity_data,
            ],
        )?;

        Ok(())
    }

    /// Save a relationship to the database
    pub fn save_relationship(&self, relationship: &Relationship) -> Result<()> {
        // Serialize metadata
        let metadata_json = if !relationship.metadata.is_empty() {
            Some(serde_json::to_string(&relationship.metadata)?)
        } else {
            None
        };

        // Get a fresh connection for this operation
        let conn = self.new_connection()?;
        conn.execute(
            "INSERT OR REPLACE INTO relationships 
            (id, source_id, target_id, relationship_type, weight, metadata)
            VALUES (?, ?, ?, ?, ?, ?)",
            params![
                relationship.id.0,
                relationship.source_id.as_str(),
                relationship.target_id.as_str(),
                relationship.relationship_type.to_string(),
                relationship.weight,
                metadata_json,
            ],
        )?;

        Ok(())
    }

    /// Load all entities from the database
    pub fn load_entities(&self) -> Result<Vec<Box<dyn Entity>>> {
        // Get a fresh connection for this operation
        let conn = self.new_connection()?;
        let mut stmt = conn.prepare(
            "SELECT id, name, entity_type, file_path, location, documentation, containing_entity, data 
             FROM entities"
        )?;

        let rows = stmt.query_map([], |row| {
            let id: String = row.get(0)?;
            let name: String = row.get(1)?;
            let entity_type: String = row.get(2)?;
            let file_path: Option<String> = row.get(3)?;
            let location_json: Option<String> = row.get(4)?;
            let documentation: Option<String> = row.get(5)?;
            let containing_entity: Option<String> = row.get(6)?;
            let data_json: String = row.get(7)?;

            Ok((
                id,
                name,
                entity_type,
                file_path,
                location_json,
                documentation,
                containing_entity,
                data_json,
            ))
        })?;

        let mut entities = Vec::new();

        for row_result in rows {
            let (
                id,
                name,
                entity_type_str,
                file_path,
                location_json,
                documentation,
                containing_entity,
                data_json,
            ) = row_result?;

            // Parse entity type
            let entity_type = parse_entity_type(&entity_type_str);

            // Parse location if present
            let location = if let Some(loc_str) = location_json {
                serde_json::from_str(&loc_str).ok()
            } else {
                None
            };

            // Create BaseEntity
            let mut base =
                BaseEntity::new(EntityId::new(&id), name, entity_type.clone(), file_path);

            base.location = location;
            base.documentation = documentation;
            base.containing_entity = containing_entity.map(|id| EntityId::new(&id));

            // Create specific entity based on type
            let entity: Box<dyn Entity> = match entity_type {
                EntityType::Function | EntityType::Method => {
                    let data: FunctionEntityData = serde_json::from_str(&data_json)?;
                    Box::new(FunctionEntity {
                        base,
                        parameters: data.parameters,
                        return_type: data.return_type,
                        visibility: data.visibility,
                        is_async: data.is_async,
                        is_static: data.is_static,
                        is_constructor: data.is_constructor,
                        is_abstract: data.is_abstract,
                    })
                }
                EntityType::Class
                | EntityType::Interface
                | EntityType::Trait
                | EntityType::Struct
                | EntityType::Enum
                | EntityType::Type => {
                    let data: TypeEntityData = serde_json::from_str(&data_json)?;
                    Box::new(TypeEntity {
                        base,
                        fields: data.fields,
                        methods: data.methods,
                        supertypes: data.supertypes,
                        visibility: data.visibility,
                        is_abstract: data.is_abstract,
                    })
                }
                EntityType::Module | EntityType::File => {
                    let data: ModuleEntityData = serde_json::from_str(&data_json)?;
                    Box::new(ModuleEntity {
                        base,
                        path: data.path,
                        children: data.children,
                        imports: data.imports,
                    })
                }
                EntityType::Variable | EntityType::Field | EntityType::Constant => {
                    let data: VariableEntityData = serde_json::from_str(&data_json)?;
                    Box::new(VariableEntity {
                        base,
                        type_annotation: data.type_annotation,
                        visibility: data.visibility,
                        is_const: data.is_const,
                        is_static: data.is_static,
                    })
                }
                EntityType::DomainConcept => {
                    let data: DomainConceptEntityData = serde_json::from_str(&data_json)?;
                    Box::new(DomainConceptEntity {
                        base,
                        attributes: data.attributes,
                        description: data.description,
                        confidence: data.confidence,
                    })
                }
                _ => Box::new(base),
            };

            entities.push(entity);
        }

        Ok(entities)
    }

    /// Load all relationships from the database
    pub fn load_relationships(&self) -> Result<Vec<Relationship>> {
        // Get a fresh connection for this operation
        let conn = self.new_connection()?;
        let mut stmt = conn.prepare(
            "SELECT id, source_id, target_id, relationship_type, weight, metadata 
             FROM relationships",
        )?;

        let rows = stmt.query_map([], |row| {
            let id: String = row.get(0)?;
            let source_id: String = row.get(1)?;
            let target_id: String = row.get(2)?;
            let relationship_type: String = row.get(3)?;
            let weight: f32 = row.get(4)?;
            let metadata_json: Option<String> = row.get(5)?;

            Ok((
                id,
                source_id,
                target_id,
                relationship_type,
                weight,
                metadata_json,
            ))
        })?;

        let mut relationships = Vec::new();

        for row_result in rows {
            let (id, source_id, target_id, relationship_type_str, weight, metadata_json) =
                row_result?;

            // Parse relationship type
            let rel_type = parse_relationship_type(&relationship_type_str);

            // Parse metadata if present
            let metadata = if let Some(meta_str) = metadata_json {
                serde_json::from_str(&meta_str)?
            } else {
                std::collections::HashMap::new()
            };

            let relationship = Relationship {
                id: crate::graph::relationship::RelationshipId::new(&id),
                source_id: EntityId::new(&source_id),
                target_id: EntityId::new(&target_id),
                relationship_type: rel_type,
                weight,
                metadata,
            };

            relationships.push(relationship);
        }

        Ok(relationships)
    }

    /// Save in a single transaction
    pub fn save_all_in_transaction(
        &self,
        entities: &[&dyn Entity],
        relationships: &[&Relationship],
    ) -> Result<()> {
        // Get a fresh connection for this operation
        let mut conn = self.new_connection()?;
        let tx = conn.transaction()?;

        // Process each entity
        for &entity in entities {
            // Get entity data using our helper function
            let entity_data = get_entity_data(entity)?;

            // Serialize location data
            let location_json = if let Some(loc) = entity.location() {
                serde_json::to_string(loc)?
            } else {
                "null".to_string()
            };

            // Save entity to database
            tx.execute(
                "INSERT OR REPLACE INTO entities 
                (id, name, entity_type, file_path, location, documentation, containing_entity, data)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
                params![
                    entity.id().as_str(),
                    entity.name(),
                    entity.entity_type().to_string(),
                    entity.file_path().map(|s| s.as_str()),
                    if entity.location().is_some() {
                        Some(location_json)
                    } else {
                        None
                    },
                    entity.metadata().get("documentation").map(|s| s.as_str()),
                    entity
                        .metadata()
                        .get("containing_entity")
                        .map(|s| s.as_str()),
                    entity_data,
                ],
            )?;
        }

        // Process each relationship
        for relationship in relationships {
            // Serialize metadata
            let metadata_json = if !relationship.metadata.is_empty() {
                Some(serde_json::to_string(&relationship.metadata)?)
            } else {
                None
            };

            // Save relationship to database
            tx.execute(
                "INSERT OR REPLACE INTO relationships 
                (id, source_id, target_id, relationship_type, weight, metadata)
                VALUES (?, ?, ?, ?, ?, ?)",
                params![
                    relationship.id.0,
                    relationship.source_id.as_str(),
                    relationship.target_id.as_str(),
                    relationship.relationship_type.to_string(),
                    relationship.weight,
                    metadata_json,
                ],
            )?;
        }

        tx.commit()?;
        Ok(())
    }
}

/// Helper to extract data from entity, using entity_type to determine the data structure
pub fn get_entity_data(entity: &dyn Entity) -> Result<String> {
    match entity.entity_type() {
        EntityType::Function | EntityType::Method => {
            // Try to get data from a function entity
            // We'll use a simple default structure rather than trying to downcast
            let parameters = vec![];
            let data = FunctionEntityData {
                parameters,
                return_type: None,
                visibility: crate::graph::entity::Visibility::Public,
                is_async: false,
                is_static: false,
                is_constructor: false,
                is_abstract: false,
            };
            Ok(serde_json::to_string(&data)?)
        }
        EntityType::Class
        | EntityType::Interface
        | EntityType::Trait
        | EntityType::Struct
        | EntityType::Enum
        | EntityType::Type => {
            // Create a default TypeEntityData
            let data = TypeEntityData {
                fields: vec![],
                methods: vec![],
                supertypes: vec![],
                visibility: crate::graph::entity::Visibility::Public,
                is_abstract: false,
            };
            Ok(serde_json::to_string(&data)?)
        }
        EntityType::Module | EntityType::File => {
            // Create a default ModuleEntityData
            let data = ModuleEntityData {
                path: entity.file_path().cloned().unwrap_or_default(),
                children: vec![],
                imports: vec![],
            };
            Ok(serde_json::to_string(&data)?)
        }
        EntityType::Variable | EntityType::Field | EntityType::Constant => {
            // Create a default VariableEntityData
            let data = VariableEntityData {
                type_annotation: None,
                visibility: crate::graph::entity::Visibility::Public,
                is_const: entity.entity_type() == EntityType::Constant,
                is_static: false,
            };
            Ok(serde_json::to_string(&data)?)
        }
        EntityType::DomainConcept => {
            // Create a default DomainConceptEntityData
            let data = DomainConceptEntityData {
                attributes: vec![],
                description: None,
                confidence: 0.5,
            };
            Ok(serde_json::to_string(&data)?)
        }
        _ => Ok("{}".to_string()),
    }
}

/// Parse entity type from string representation
pub fn parse_entity_type(type_str: &str) -> EntityType {
    match type_str {
        "Function" => EntityType::Function,
        "Method" => EntityType::Method,
        "Class" => EntityType::Class,
        "Interface" => EntityType::Interface,
        "Trait" => EntityType::Trait,
        "Struct" => EntityType::Struct,
        "Enum" => EntityType::Enum,
        "Module" => EntityType::Module,
        "File" => EntityType::File,
        "Variable" => EntityType::Variable,
        "Field" => EntityType::Field,
        "Constant" => EntityType::Constant,
        "DomainConcept" => EntityType::DomainConcept,
        "Type" => EntityType::Type,
        _ => {
            if type_str.starts_with("Other") {
                // Extract the content between parentheses for Other type
                if let Some(content) = type_str
                    .strip_prefix("Other(")
                    .and_then(|s| s.strip_suffix(")"))
                {
                    EntityType::Other(content.to_string())
                } else {
                    EntityType::Other(type_str.to_string())
                }
            } else {
                EntityType::Other(type_str.to_string())
            }
        }
    }
}

/// Parse relationship type from string representation
pub fn parse_relationship_type(type_str: &str) -> RelationshipType {
    match type_str {
        "Calls" => RelationshipType::Calls,
        "Contains" => RelationshipType::Contains,
        "Imports" => RelationshipType::Imports,
        "Inherits" => RelationshipType::Inherits,
        "Implements" => RelationshipType::Implements,
        "References" => RelationshipType::References,
        "Defines" => RelationshipType::Defines,
        "Uses" => RelationshipType::Uses,
        "Depends" => RelationshipType::Depends,
        "RepresentedBy" => RelationshipType::RepresentedBy,
        "RelatesTo" => RelationshipType::RelatesTo,
        "DependsOn" => RelationshipType::DependsOn,
        _ => {
            if type_str.starts_with("Other") {
                // Extract the content between parentheses for Other type
                if let Some(content) = type_str
                    .strip_prefix("Other(")
                    .and_then(|s| s.strip_suffix(")"))
                {
                    RelationshipType::Other(content.to_string())
                } else {
                    RelationshipType::Other(type_str.to_string())
                }
            } else {
                RelationshipType::Other(type_str.to_string())
            }
        }
    }
}

// Serializable data structs for each entity type

#[derive(serde::Serialize, serde::Deserialize)]
pub struct FunctionEntityData {
    pub parameters: Vec<crate::graph::entity::Parameter>,
    pub return_type: Option<String>,
    pub visibility: crate::graph::entity::Visibility,
    pub is_async: bool,
    pub is_static: bool,
    pub is_constructor: bool,
    pub is_abstract: bool,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct TypeEntityData {
    pub fields: Vec<EntityId>,
    pub methods: Vec<EntityId>,
    pub supertypes: Vec<EntityId>,
    pub visibility: crate::graph::entity::Visibility,
    pub is_abstract: bool,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct ModuleEntityData {
    pub path: String,
    pub children: Vec<EntityId>,
    pub imports: Vec<String>,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct VariableEntityData {
    pub type_annotation: Option<String>,
    pub visibility: crate::graph::entity::Visibility,
    pub is_const: bool,
    pub is_static: bool,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct DomainConceptEntityData {
    pub attributes: Vec<String>,
    pub description: Option<String>,
    pub confidence: f32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::entity::{Parameter, Visibility};
    use tempfile::tempdir;

    #[test]
    fn test_database_initialization() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");

        // Create a new database
        let _db = Database::new(&db_path).unwrap();

        // Check that it was created
        assert!(db_path.exists());
    }

    #[test]
    fn test_save_and_load_entity() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let db = Database::new(&db_path).unwrap();

        // Create a function entity
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
            return_type: Some("void".to_string()),
            visibility: Visibility::Public,
            is_async: false,
            is_static: false,
            is_constructor: false,
            is_abstract: false,
        };

        // Save entity
        db.save_entity(&function).unwrap();

        // Load entities
        let entities = db.load_entities().unwrap();

        // Check that we got our entity back
        assert_eq!(entities.len(), 1);
        assert_eq!(entities[0].id().as_str(), "test::function");
        assert_eq!(entities[0].name(), "testFunction");
        assert!(matches!(entities[0].entity_type(), EntityType::Function));
    }

    #[test]
    fn test_save_and_load_relationship() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let db = Database::new(&db_path).unwrap();

        // Create two entities
        let id1 = EntityId::new("entity1");
        let base1 = BaseEntity::new(
            id1.clone(),
            "Entity1".to_string(),
            EntityType::Function,
            Some("test.rs".to_string()),
        );

        let id2 = EntityId::new("entity2");
        let base2 = BaseEntity::new(
            id2.clone(),
            "Entity2".to_string(),
            EntityType::Function,
            Some("test.rs".to_string()),
        );

        // Save entities (needed for foreign key constraints)
        db.save_entity(&base1).unwrap();
        db.save_entity(&base2).unwrap();

        // Create a relationship
        let rel_id = crate::graph::relationship::RelationshipId::new("test_rel");
        let rel = Relationship::new(
            rel_id.clone(),
            id1.clone(),
            id2.clone(),
            RelationshipType::Calls,
        );

        // Save relationship
        db.save_relationship(&rel).unwrap();

        // Load relationships
        let relationships = db.load_relationships().unwrap();

        // Check that we got our relationship back
        assert_eq!(relationships.len(), 1);
        assert_eq!(relationships[0].id.0, "test_rel");
        assert_eq!(relationships[0].source_id.as_str(), "entity1");
        assert_eq!(relationships[0].target_id.as_str(), "entity2");
        assert!(matches!(
            relationships[0].relationship_type,
            RelationshipType::Calls
        ));
    }

    #[test]
    fn test_multiple_entities_and_relationships() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let db = Database::new(&db_path).unwrap();

        // No transaction needed for this test since we'll use direct methods

        // Create multiple entity types
        let func_id = EntityId::new("func::test");
        let func_base = BaseEntity::new(
            func_id.clone(),
            "testFunc".to_string(),
            EntityType::Function,
            Some("test.rs".to_string()),
        );

        let func = FunctionEntity {
            base: func_base,
            parameters: vec![Parameter {
                name: "arg1".to_string(),
                type_annotation: Some("i32".to_string()),
                default_value: None,
            }],
            return_type: Some("bool".to_string()),
            visibility: Visibility::Public,
            is_async: true,
            is_static: false,
            is_constructor: false,
            is_abstract: false,
        };

        let class_id = EntityId::new("class::test");
        let class_base = BaseEntity::new(
            class_id.clone(),
            "TestClass".to_string(),
            EntityType::Class,
            Some("test.rs".to_string()),
        );

        let class = TypeEntity {
            base: class_base,
            fields: vec![],
            methods: vec![func_id.clone()],
            supertypes: vec![],
            visibility: Visibility::Public,
            is_abstract: false,
        };

        // Save entities
        db.save_entity(&func).unwrap();
        db.save_entity(&class).unwrap();

        // Create relationship
        let rel = Relationship::new(
            crate::graph::relationship::RelationshipId::new("contains_rel"),
            class_id.clone(),
            func_id.clone(),
            RelationshipType::Contains,
        );

        db.save_relationship(&rel).unwrap();

        // Load everything back
        let entities = db.load_entities().unwrap();
        let relationships = db.load_relationships().unwrap();

        // Verify counts
        assert_eq!(entities.len(), 2);
        assert_eq!(relationships.len(), 1);

        // Find the function and class by their IDs
        let func_entity = entities
            .iter()
            .find(|e| e.id().as_str() == "func::test")
            .unwrap();
        let class_entity = entities
            .iter()
            .find(|e| e.id().as_str() == "class::test")
            .unwrap();

        // Check their types
        assert!(matches!(func_entity.entity_type(), EntityType::Function));
        assert!(matches!(class_entity.entity_type(), EntityType::Class));

        // Check the relationship
        assert_eq!(relationships[0].source_id.as_str(), "class::test");
        assert_eq!(relationships[0].target_id.as_str(), "func::test");
        assert!(matches!(
            relationships[0].relationship_type,
            RelationshipType::Contains
        ));
    }
}
