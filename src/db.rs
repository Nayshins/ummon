use anyhow::Result;
use indoc::indoc;
use log::{debug, error, info};
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::params;
use std::path::Path;

use crate::graph::entity::{
    BaseEntity, DomainConceptEntity, DomainConceptEntityData, Entity, EntityId, EntityType,
    FunctionEntity, FunctionEntityData, ModuleEntity, ModuleEntityData, TypeEntity, TypeEntityData,
    VariableEntity, VariableEntityData,
};
use crate::graph::relationship::{Relationship, RelationshipType};

/// Get a database instance - this is a convenience method that just calls Database::new
pub fn get_database(path: &str) -> Result<Database> {
    Database::new(path)
}

/// Database wrapper for managing the SQLite knowledge graph storage with connection pooling
pub struct Database {
    pool: Pool<SqliteConnectionManager>,
    db_path: String,
}

impl std::fmt::Debug for Database {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Database")
            .field("pool", &"SQLite Connection Pool")
            .field("db_path", &self.db_path)
            .finish()
    }
}

impl Clone for Database {
    fn clone(&self) -> Self {
        Self {
            pool: self.pool.clone(),
            db_path: self.db_path.clone(),
        }
    }
}

impl Database {
    /// Create a new database connection, initializing the schema if needed
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let db_path = path.as_ref().to_string_lossy().to_string();
        info!("Initializing database connection pool at {}", db_path);

        // Configure SQLite connection
        let manager = SqliteConnectionManager::file(&path);

        // Build a connection pool with a max size of 10 connections
        let pool = Pool::builder().max_size(10).build(manager).map_err(|e| {
            anyhow::anyhow!("Failed to create connection pool for {}: {}", db_path, e)
        })?;

        let db = Self { pool, db_path };

        // Initialize the schema
        db.initialize_schema()?;
        debug!("Database schema initialized successfully");

        Ok(db)
    }

    /// Get a connection from the pool
    pub fn get_connection(&self) -> Result<r2d2::PooledConnection<SqliteConnectionManager>> {
        self.pool.get().map_err(|e| {
            anyhow::anyhow!(
                "Failed to get connection from pool for {}: {}",
                self.db_path,
                e
            )
        })
    }

    /// Initialize the database schema if needed, using a single transaction
    fn initialize_schema(&self) -> Result<()> {
        debug!("Initializing database schema for {}", self.db_path);
        let conn = self.get_connection()?;

        // Simple schema creation - all statements use IF NOT EXISTS
        conn.execute_batch(indoc! {r#"
            CREATE TABLE IF NOT EXISTS schema_version (
                version INTEGER PRIMARY KEY
            );
            
            CREATE TABLE IF NOT EXISTS entities (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                entity_type TEXT NOT NULL,
                file_path TEXT,
                location TEXT,
                documentation TEXT,
                containing_entity TEXT,
                data TEXT NOT NULL
            );
            
            CREATE TABLE IF NOT EXISTS relationships (
                id TEXT PRIMARY KEY,
                source_id TEXT NOT NULL,
                target_id TEXT NOT NULL,
                relationship_type TEXT NOT NULL,
                weight REAL NOT NULL DEFAULT 1.0,
                metadata TEXT,
                FOREIGN KEY(source_id) REFERENCES entities(id),
                FOREIGN KEY(target_id) REFERENCES entities(id)
            );
            
            CREATE INDEX IF NOT EXISTS idx_entity_name ON entities(name);
            CREATE INDEX IF NOT EXISTS idx_entity_type ON entities(entity_type);
            CREATE INDEX IF NOT EXISTS idx_entity_file_path ON entities(file_path);
            
            CREATE INDEX IF NOT EXISTS idx_rel_source ON relationships(source_id);
            CREATE INDEX IF NOT EXISTS idx_rel_target ON relationships(target_id);
            CREATE INDEX IF NOT EXISTS idx_rel_type ON relationships(relationship_type);
            
            -- Add enhanced indexes for SQL-based queries
            CREATE INDEX IF NOT EXISTS entity_type_idx ON entities(entity_type);
            CREATE INDEX IF NOT EXISTS name_idx ON entities(name);
            CREATE INDEX IF NOT EXISTS file_path_idx ON entities(file_path);
            
            -- Initialize version if needed (using OR IGNORE to avoid errors if already exists)
            INSERT OR IGNORE INTO schema_version (version) VALUES (1);
        "#})?;

        debug!("Database schema initialized successfully");
        Ok(())
    }

    /// Save an entity to the database
    ///
    /// Note: This method is part of the storage API but not currently used.
    /// It is preserved for future functionality.
    #[allow(dead_code)]
    pub fn save_entity(&self, entity: &dyn Entity) -> Result<()> {
        info!("Saving entity {} to {}", entity.id().as_str(), self.db_path);

        // Get entity data using the entity's serialize_data method
        let entity_data = entity.serialize_data().map_err(|e| {
            anyhow::anyhow!(
                "Failed to serialize entity data for {}: {}",
                entity.id().as_str(),
                e
            )
        })?;

        // Serialize location data
        let location_json = if let Some(loc) = entity.location() {
            serde_json::to_string(loc).map_err(|e| {
                anyhow::anyhow!(
                    "Failed to serialize location for {}: {}",
                    entity.id().as_str(),
                    e
                )
            })?
        } else {
            "null".to_string()
        };

        // Get a connection from the pool
        let conn = self.get_connection()?;

        // Execute the insert/update
        match conn.execute(
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
        ) {
            Ok(_) => {
                debug!("Successfully saved entity {}", entity.id().as_str());
                Ok(())
            }
            Err(e) => {
                error!("Failed to save entity {}: {}", entity.id().as_str(), e);
                Err(anyhow::anyhow!(
                    "Failed to save entity {}: {}",
                    entity.id().as_str(),
                    e
                ))
            }
        }
    }

    /// Save a relationship to the database
    ///
    /// Note: This method is part of the storage API but not currently used.
    /// It is preserved for future functionality.
    #[allow(dead_code)]
    pub fn save_relationship(&self, relationship: &Relationship) -> Result<()> {
        info!(
            "Saving relationship {} from {} to {}",
            relationship.id.0,
            relationship.source_id.as_str(),
            relationship.target_id.as_str()
        );

        // Serialize metadata
        let metadata_json = if !relationship.metadata.is_empty() {
            Some(serde_json::to_string(&relationship.metadata).map_err(|e| {
                anyhow::anyhow!(
                    "Failed to serialize relationship metadata for {}: {}",
                    relationship.id.0,
                    e
                )
            })?)
        } else {
            None
        };

        // Get a connection from the pool
        let conn = self.get_connection()?;

        // Execute the insert/update
        match conn.execute(
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
        ) {
            Ok(_) => {
                debug!("Successfully saved relationship {}", relationship.id.0);
                Ok(())
            }
            Err(e) => {
                error!("Failed to save relationship {}: {}", relationship.id.0, e);
                Err(anyhow::anyhow!(
                    "Failed to save relationship {}: {}",
                    relationship.id.0,
                    e
                ))
            }
        }
    }

    /// Load a single entity by ID
    pub fn load_entity(&self, id: &EntityId) -> Result<Option<Box<dyn Entity>>> {
        debug!("Loading entity {} from {}", id.as_str(), self.db_path);

        // Get a connection from the pool
        let conn = self.get_connection()?;
        let mut stmt = conn.prepare(
            "SELECT id, name, entity_type, file_path, location, documentation, containing_entity, data 
             FROM entities WHERE id = ?"
        )?;

        let mut rows = stmt.query(params![id.as_str()])?;

        if let Some(row) = rows.next()? {
            let id: String = row.get(0)?;
            let name: String = row.get(1)?;
            let entity_type_str: String = row.get(2)?;
            let file_path: Option<String> = row.get(3)?;
            let location_json: Option<String> = row.get(4)?;
            let documentation: Option<String> = row.get(5)?;
            let containing_entity: Option<String> = row.get(6)?;
            let data_json: String = row.get(7)?;

            // Parse entity type
            let entity_type = parse_entity_type(&entity_type_str);

            // Parse location if present
            let location = if let Some(loc_str) = location_json {
                match serde_json::from_str(&loc_str) {
                    Ok(loc) => Some(loc),
                    Err(e) => {
                        error!("Failed to parse location for entity {}: {}", id, e);
                        None
                    }
                }
            } else {
                None
            };

            // Create BaseEntity
            let mut base =
                BaseEntity::new(EntityId::new(&id), name, entity_type.clone(), file_path);

            base.location = location;
            base.documentation = documentation;
            base.containing_entity = containing_entity.map(|id| EntityId::new(&id));

            // Create specific entity based on type, with improved error handling
            let entity: Box<dyn Entity> = match entity_type {
                EntityType::Function | EntityType::Method => {
                    match serde_json::from_str::<FunctionEntityData>(&data_json) {
                        Ok(data) => Box::new(FunctionEntity {
                            base,
                            parameters: data.parameters,
                            return_type: data.return_type,
                            visibility: data.visibility,
                            is_async: data.is_async,
                            is_static: data.is_static,
                            is_constructor: data.is_constructor,
                            is_abstract: data.is_abstract,
                        }),
                        Err(e) => {
                            error!(
                                "Failed to parse FunctionEntityData for entity {}: {}, using default values",
                                id, e
                            );
                            // Use default values
                            let default_data = FunctionEntityData::default();
                            Box::new(FunctionEntity {
                                base,
                                parameters: default_data.parameters,
                                return_type: default_data.return_type,
                                visibility: default_data.visibility,
                                is_async: default_data.is_async,
                                is_static: default_data.is_static,
                                is_constructor: default_data.is_constructor,
                                is_abstract: default_data.is_abstract,
                            })
                        }
                    }
                }
                EntityType::Class
                | EntityType::Interface
                | EntityType::Trait
                | EntityType::Struct
                | EntityType::Enum
                | EntityType::Type => match serde_json::from_str::<TypeEntityData>(&data_json) {
                    Ok(data) => Box::new(TypeEntity {
                        base,
                        fields: data.fields,
                        methods: data.methods,
                        supertypes: data.supertypes,
                        visibility: data.visibility,
                        is_abstract: data.is_abstract,
                    }),
                    Err(e) => {
                        error!("Failed to parse TypeEntityData for entity {}: {}, using default values", id, e);
                        // Use default values
                        let default_data = TypeEntityData::default();
                        Box::new(TypeEntity {
                            base,
                            fields: default_data.fields,
                            methods: default_data.methods,
                            supertypes: default_data.supertypes,
                            visibility: default_data.visibility,
                            is_abstract: default_data.is_abstract,
                        })
                    }
                },
                EntityType::Module | EntityType::File => {
                    match serde_json::from_str::<ModuleEntityData>(&data_json) {
                        Ok(data) => Box::new(ModuleEntity {
                            base,
                            path: data.path,
                            children: data.children,
                            imports: data.imports,
                        }),
                        Err(e) => {
                            error!("Failed to parse ModuleEntityData for entity {}: {}, using default values", id, e);
                            // Use default values
                            let default_data = ModuleEntityData::default();
                            Box::new(ModuleEntity {
                                base,
                                path: default_data.path,
                                children: default_data.children,
                                imports: default_data.imports,
                            })
                        }
                    }
                }
                EntityType::Variable | EntityType::Field | EntityType::Constant => {
                    match serde_json::from_str::<VariableEntityData>(&data_json) {
                        Ok(data) => Box::new(VariableEntity {
                            base,
                            type_annotation: data.type_annotation,
                            visibility: data.visibility,
                            is_const: data.is_const,
                            is_static: data.is_static,
                        }),
                        Err(e) => {
                            error!(
                                "Failed to parse VariableEntityData for entity {}: {}, using default values",
                                id, e
                            );
                            // Use default values
                            let default_data = VariableEntityData::default();
                            Box::new(VariableEntity {
                                base,
                                type_annotation: default_data.type_annotation,
                                visibility: default_data.visibility,
                                is_const: default_data.is_const,
                                is_static: default_data.is_static,
                            })
                        }
                    }
                }
                EntityType::DomainConcept => {
                    match serde_json::from_str::<DomainConceptEntityData>(&data_json) {
                        Ok(data) => Box::new(DomainConceptEntity {
                            base,
                            attributes: data.attributes,
                            description: data.description,
                            confidence: data.confidence,
                        }),
                        Err(e) => {
                            error!(
                                "Failed to parse DomainConceptEntityData for entity {}: {}, using default values",
                                id, e
                            );
                            // Use default values
                            let default_data = DomainConceptEntityData::default();
                            Box::new(DomainConceptEntity {
                                base,
                                attributes: default_data.attributes,
                                description: default_data.description,
                                confidence: default_data.confidence,
                            })
                        }
                    }
                }
                _ => Box::new(base),
            };

            Ok(Some(entity))
        } else {
            Ok(None)
        }
    }

    /// Load all entities from the database
    pub fn load_entities(&self) -> Result<Vec<Box<dyn Entity>>> {
        info!("Loading all entities from {}", self.db_path);

        // Get a connection from the pool
        let conn = self.get_connection()?;
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
            let result = row_result.map_err(|e| anyhow::anyhow!("Error reading entity row: {}", e));

            let (
                id,
                name,
                entity_type_str,
                file_path,
                location_json,
                documentation,
                containing_entity,
                data_json,
            ) = match result {
                Ok(data) => data,
                Err(e) => {
                    error!("Failed to read entity row: {}", e);
                    continue; // Skip this row and continue with the next one
                }
            };

            // Parse entity type
            let entity_type = parse_entity_type(&entity_type_str);

            // Parse location if present
            let location = if let Some(loc_str) = location_json {
                match serde_json::from_str(&loc_str) {
                    Ok(loc) => Some(loc),
                    Err(e) => {
                        error!("Failed to parse location for entity {}: {}", id, e);
                        None
                    }
                }
            } else {
                None
            };

            // Create BaseEntity
            let mut base =
                BaseEntity::new(EntityId::new(&id), name, entity_type.clone(), file_path);

            base.location = location;
            base.documentation = documentation;
            base.containing_entity = containing_entity.map(|id| EntityId::new(&id));

            // Create specific entity based on type, with improved error handling
            let entity: Box<dyn Entity> = match entity_type {
                EntityType::Function | EntityType::Method => {
                    match serde_json::from_str::<FunctionEntityData>(&data_json) {
                        Ok(data) => Box::new(FunctionEntity {
                            base,
                            parameters: data.parameters,
                            return_type: data.return_type,
                            visibility: data.visibility,
                            is_async: data.is_async,
                            is_static: data.is_static,
                            is_constructor: data.is_constructor,
                            is_abstract: data.is_abstract,
                        }),
                        Err(e) => {
                            error!(
                                "Failed to parse FunctionEntityData for entity {}: {}, trying with default values",
                                id, e
                            );
                            // Try to parse with relaxed settings or use default values
                            let default_data = FunctionEntityData::default();
                            Box::new(FunctionEntity {
                                base,
                                parameters: default_data.parameters,
                                return_type: default_data.return_type,
                                visibility: default_data.visibility,
                                is_async: default_data.is_async,
                                is_static: default_data.is_static,
                                is_constructor: default_data.is_constructor,
                                is_abstract: default_data.is_abstract,
                            })
                        }
                    }
                }
                EntityType::Class
                | EntityType::Interface
                | EntityType::Trait
                | EntityType::Struct
                | EntityType::Enum
                | EntityType::Type => match serde_json::from_str::<TypeEntityData>(&data_json) {
                    Ok(data) => Box::new(TypeEntity {
                        base,
                        fields: data.fields,
                        methods: data.methods,
                        supertypes: data.supertypes,
                        visibility: data.visibility,
                        is_abstract: data.is_abstract,
                    }),
                    Err(e) => {
                        error!("Failed to parse TypeEntityData for entity {}: {}, trying with default values", id, e);
                        // Use default values
                        let default_data = TypeEntityData::default();
                        Box::new(TypeEntity {
                            base,
                            fields: default_data.fields,
                            methods: default_data.methods,
                            supertypes: default_data.supertypes,
                            visibility: default_data.visibility,
                            is_abstract: default_data.is_abstract,
                        })
                    }
                },
                EntityType::Module | EntityType::File => {
                    match serde_json::from_str::<ModuleEntityData>(&data_json) {
                        Ok(data) => Box::new(ModuleEntity {
                            base,
                            path: data.path,
                            children: data.children,
                            imports: data.imports,
                        }),
                        Err(e) => {
                            error!("Failed to parse ModuleEntityData for entity {}: {}, trying with default values", id, e);
                            // Use default values
                            let default_data = ModuleEntityData::default();
                            Box::new(ModuleEntity {
                                base,
                                path: default_data.path,
                                children: default_data.children,
                                imports: default_data.imports,
                            })
                        }
                    }
                }
                EntityType::Variable | EntityType::Field | EntityType::Constant => {
                    match serde_json::from_str::<VariableEntityData>(&data_json) {
                        Ok(data) => Box::new(VariableEntity {
                            base,
                            type_annotation: data.type_annotation,
                            visibility: data.visibility,
                            is_const: data.is_const,
                            is_static: data.is_static,
                        }),
                        Err(e) => {
                            error!(
                                "Failed to parse VariableEntityData for entity {}: {}, trying with default values",
                                id, e
                            );
                            // Use default values
                            let default_data = VariableEntityData::default();
                            Box::new(VariableEntity {
                                base,
                                type_annotation: default_data.type_annotation,
                                visibility: default_data.visibility,
                                is_const: default_data.is_const,
                                is_static: default_data.is_static,
                            })
                        }
                    }
                }
                EntityType::DomainConcept => {
                    match serde_json::from_str::<DomainConceptEntityData>(&data_json) {
                        Ok(data) => Box::new(DomainConceptEntity {
                            base,
                            attributes: data.attributes,
                            description: data.description,
                            confidence: data.confidence,
                        }),
                        Err(e) => {
                            error!(
                                "Failed to parse DomainConceptEntityData for entity {}: {}, trying with default values",
                                id, e
                            );
                            // Use default values
                            let default_data = DomainConceptEntityData::default();
                            Box::new(DomainConceptEntity {
                                base,
                                attributes: default_data.attributes,
                                description: default_data.description,
                                confidence: default_data.confidence,
                            })
                        }
                    }
                }
                _ => Box::new(base),
            };

            entities.push(entity);
        }

        debug!("Loaded {} entities from database", entities.len());
        Ok(entities)
    }

    /// Load all relationships from the database
    pub fn load_relationships(&self) -> Result<Vec<Relationship>> {
        info!("Loading all relationships from {}", self.db_path);

        // Get a connection from the pool
        let conn = self.get_connection()?;
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
            let result =
                row_result.map_err(|e| anyhow::anyhow!("Error reading relationship row: {}", e));

            let (id, source_id, target_id, relationship_type_str, weight, metadata_json) =
                match result {
                    Ok(data) => data,
                    Err(e) => {
                        error!("Failed to read relationship row: {}", e);
                        continue; // Skip this row and continue with the next one
                    }
                };

            // Parse relationship type
            let rel_type = parse_relationship_type(&relationship_type_str);

            // Parse metadata if present with improved error handling
            let metadata = if let Some(meta_str) = metadata_json {
                match serde_json::from_str(&meta_str) {
                    Ok(meta) => meta,
                    Err(e) => {
                        error!("Failed to parse metadata for relationship {}: {}", id, e);
                        std::collections::HashMap::new()
                    }
                }
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

        debug!("Loaded {} relationships from database", relationships.len());
        Ok(relationships)
    }

    /// Load relationships for a specific entity
    pub fn load_relationships_for_entity(&self, entity_id: &EntityId) -> Result<Vec<Relationship>> {
        debug!(
            "Loading relationships for entity {} from {}",
            entity_id.as_str(),
            self.db_path
        );

        // Get a connection from the pool
        let conn = self.get_connection()?;
        let mut stmt = conn.prepare(
            "SELECT id, source_id, target_id, relationship_type, weight, metadata 
             FROM relationships 
             WHERE source_id = ? OR target_id = ?",
        )?;

        let rows = stmt.query_map([entity_id.as_str(), entity_id.as_str()], |row| {
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
            let result =
                row_result.map_err(|e| anyhow::anyhow!("Error reading relationship row: {}", e));

            let (id, source_id, target_id, relationship_type_str, weight, metadata_json) =
                match result {
                    Ok(data) => data,
                    Err(e) => {
                        error!("Failed to read relationship row: {}", e);
                        continue; // Skip this row and continue with the next one
                    }
                };

            // Parse relationship type
            let rel_type = parse_relationship_type(&relationship_type_str);

            // Parse metadata if present with improved error handling
            let metadata = if let Some(meta_str) = metadata_json {
                match serde_json::from_str(&meta_str) {
                    Ok(meta) => meta,
                    Err(e) => {
                        error!("Failed to parse metadata for relationship {}: {}", id, e);
                        std::collections::HashMap::new()
                    }
                }
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

        debug!(
            "Loaded {} relationships for entity {}",
            relationships.len(),
            entity_id.as_str()
        );
        Ok(relationships)
    }

    /// Execute a traversal query using a recursive Common Table Expression (CTE)
    pub fn execute_traversal(
        &self,
        start_id: &EntityId,
        relationship_type: Option<&RelationshipType>,
        direction: &str,
        max_depth: Option<usize>,
    ) -> Result<Vec<(EntityId, usize)>> {
        debug!(
            "Executing traversal query from {} with direction {}",
            start_id.as_str(),
            direction
        );

        let conn = self.get_connection()?;

        // Build the relationship filter condition
        let rel_filter = if let Some(rel_type) = relationship_type {
            format!("AND r.relationship_type = '{}'", rel_type.to_string())
        } else {
            String::new()
        };

        // Build the depth filter condition
        let depth_filter = if let Some(depth) = max_depth {
            format!("AND t.depth <= {}", depth)
        } else {
            String::new()
        };

        // Build the direction condition
        let direction_condition = match direction {
            "outbound" => "r.source_id = t.id",
            "inbound" => "r.target_id = t.id",
            "both" => "(r.source_id = t.id OR r.target_id = t.id)",
            _ => return Err(anyhow::anyhow!("Invalid direction: {}", direction)),
        };

        // Define the target selection based on direction
        let next_id = match direction {
            "outbound" => "r.target_id",
            "inbound" => "r.source_id",
            "both" => "CASE WHEN r.source_id = t.id THEN r.target_id ELSE r.source_id END",
            _ => return Err(anyhow::anyhow!("Invalid direction: {}", direction)),
        };

        // Build the recursive CTE SQL query
        let sql = format!(
            "WITH RECURSIVE traverse(id, depth) AS (
                SELECT id, 0 FROM entities WHERE id = ?
                UNION
                SELECT {}, t.depth + 1
                FROM relationships r
                JOIN traverse t ON {}
                WHERE 1=1 {} {}
            )
            SELECT id, depth FROM traverse ORDER BY depth",
            next_id, direction_condition, rel_filter, depth_filter
        );

        // Execute the query
        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map([start_id.as_str()], |row| {
            let id: String = row.get(0)?;
            let depth: usize = row.get(1)?;
            Ok((EntityId::new(&id), depth))
        })?;

        // Collect the results
        let result = rows.collect::<rusqlite::Result<Vec<_>>>()?;
        debug!(
            "Traversal found {} nodes from {}",
            result.len(),
            start_id.as_str()
        );

        Ok(result)
    }

    /// Execute a SQL query to select entities based on entity type and conditions
    pub fn execute_entity_select(
        &self,
        entity_type: &EntityType,
        conditions: &[(String, String)],
    ) -> Result<Vec<Box<dyn Entity>>> {
        debug!(
            "Executing entity select query for type {:?} with {} conditions",
            entity_type,
            conditions.len()
        );

        // Build the SQL query with conditions
        let mut sql = "SELECT id, name, entity_type, file_path, location, documentation, containing_entity, data FROM entities WHERE entity_type = ?".to_string();

        let mut params: Vec<&dyn rusqlite::ToSql> = vec![&entity_type.to_string()];

        for (i, (field, value)) in conditions.iter().enumerate() {
            sql.push_str(&format!(" AND {} = ?", field));
            params.push(value as &dyn rusqlite::ToSql);
        }

        // Get a connection from the pool
        let conn = self.get_connection()?;
        let mut stmt = conn.prepare(&sql)?;

        // Execute the query with parameters
        let rows = stmt.query_map(rusqlite::params_from_iter(params.iter()), |row| {
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

        // Process the results
        for row_result in rows {
            let result = row_result.map_err(|e| anyhow::anyhow!("Error reading entity row: {}", e));

            let (
                id,
                name,
                entity_type_str,
                file_path,
                location_json,
                documentation,
                containing_entity,
                data_json,
            ) = match result {
                Ok(data) => data,
                Err(e) => {
                    error!("Failed to read entity row: {}", e);
                    continue; // Skip this row and continue with the next one
                }
            };

            // Parse entity type
            let entity_type = parse_entity_type(&entity_type_str);

            // Parse location if present
            let location = if let Some(loc_str) = location_json {
                match serde_json::from_str(&loc_str) {
                    Ok(loc) => Some(loc),
                    Err(e) => {
                        error!("Failed to parse location for entity {}: {}", id, e);
                        None
                    }
                }
            } else {
                None
            };

            // Create BaseEntity
            let mut base =
                BaseEntity::new(EntityId::new(&id), name, entity_type.clone(), file_path);
            base.location = location;
            base.documentation = documentation;
            base.containing_entity = containing_entity.map(|id| EntityId::new(&id));

            // Create specific entity based on type, with improved error handling
            let entity: Box<dyn Entity> = match entity_type {
                EntityType::Function | EntityType::Method => {
                    match serde_json::from_str::<FunctionEntityData>(&data_json) {
                        Ok(data) => Box::new(FunctionEntity {
                            base,
                            parameters: data.parameters,
                            return_type: data.return_type,
                            visibility: data.visibility,
                            is_async: data.is_async,
                            is_static: data.is_static,
                            is_constructor: data.is_constructor,
                            is_abstract: data.is_abstract,
                        }),
                        Err(e) => {
                            error!(
                                "Failed to parse FunctionEntityData for entity {}: {}, using default values",
                                id, e
                            );
                            // Use default values
                            let default_data = FunctionEntityData::default();
                            Box::new(FunctionEntity {
                                base,
                                parameters: default_data.parameters,
                                return_type: default_data.return_type,
                                visibility: default_data.visibility,
                                is_async: default_data.is_async,
                                is_static: default_data.is_static,
                                is_constructor: default_data.is_constructor,
                                is_abstract: default_data.is_abstract,
                            })
                        }
                    }
                }
                EntityType::Class
                | EntityType::Interface
                | EntityType::Trait
                | EntityType::Struct
                | EntityType::Enum
                | EntityType::Type => {
                    match serde_json::from_str::<TypeEntityData>(&data_json) {
                        Ok(data) => Box::new(TypeEntity {
                            base,
                            fields: data.fields,
                            methods: data.methods,
                            supertypes: data.supertypes,
                            visibility: data.visibility,
                            is_abstract: data.is_abstract,
                        }),
                        Err(e) => {
                            error!("Failed to parse TypeEntityData for entity {}: {}, using default values", id, e);
                            // Use default values
                            let default_data = TypeEntityData::default();
                            Box::new(TypeEntity {
                                base,
                                fields: default_data.fields,
                                methods: default_data.methods,
                                supertypes: default_data.supertypes,
                                visibility: default_data.visibility,
                                is_abstract: default_data.is_abstract,
                            })
                        }
                    }
                }
                EntityType::Module | EntityType::File => {
                    match serde_json::from_str::<ModuleEntityData>(&data_json) {
                        Ok(data) => Box::new(ModuleEntity {
                            base,
                            path: data.path,
                            children: data.children,
                            imports: data.imports,
                        }),
                        Err(e) => {
                            error!("Failed to parse ModuleEntityData for entity {}: {}, using default values", id, e);
                            // Use default values
                            let default_data = ModuleEntityData::default();
                            Box::new(ModuleEntity {
                                base,
                                path: default_data.path,
                                children: default_data.children,
                                imports: default_data.imports,
                            })
                        }
                    }
                }
                EntityType::Variable | EntityType::Field | EntityType::Constant => {
                    match serde_json::from_str::<VariableEntityData>(&data_json) {
                        Ok(data) => Box::new(VariableEntity {
                            base,
                            type_annotation: data.type_annotation,
                            visibility: data.visibility,
                            is_const: data.is_const,
                            is_static: data.is_static,
                        }),
                        Err(e) => {
                            error!(
                                "Failed to parse VariableEntityData for entity {}: {}, using default values",
                                id, e
                            );
                            // Use default values
                            let default_data = VariableEntityData::default();
                            Box::new(VariableEntity {
                                base,
                                type_annotation: default_data.type_annotation,
                                visibility: default_data.visibility,
                                is_const: default_data.is_const,
                                is_static: default_data.is_static,
                            })
                        }
                    }
                }
                EntityType::DomainConcept => {
                    match serde_json::from_str::<DomainConceptEntityData>(&data_json) {
                        Ok(data) => Box::new(DomainConceptEntity {
                            base,
                            attributes: data.attributes,
                            description: data.description,
                            confidence: data.confidence,
                        }),
                        Err(e) => {
                            error!(
                                "Failed to parse DomainConceptEntityData for entity {}: {}, using default values",
                                id, e
                            );
                            // Use default values
                            let default_data = DomainConceptEntityData::default();
                            Box::new(DomainConceptEntity {
                                base,
                                attributes: default_data.attributes,
                                description: default_data.description,
                                confidence: default_data.confidence,
                            })
                        }
                    }
                }
                _ => Box::new(base),
            };

            entities.push(entity);
        }

        debug!(
            "Selected {} entities of type {:?}",
            entities.len(),
            entity_type
        );
        Ok(entities)
    }

    /// Save multiple entities and relationships in a single transaction
    pub fn save_all_in_transaction(
        &self,
        entities: &[&dyn Entity],
        relationships: &[&Relationship],
    ) -> Result<()> {
        info!(
            "Saving {} entities and {} relationships in transaction to {}",
            entities.len(),
            relationships.len(),
            self.db_path
        );

        // Get a connection from the pool
        let mut conn = self.get_connection()?;
        let tx = conn.transaction()?;

        // Process each entity
        for &entity in entities {
            // Get entity data using the entity's serialize_data method
            let entity_data = entity.serialize_data().map_err(|e| {
                anyhow::anyhow!(
                    "Failed to serialize entity data for {}: {}",
                    entity.id().as_str(),
                    e
                )
            })?;

            // Serialize location data
            let location_json = if let Some(loc) = entity.location() {
                serde_json::to_string(loc).map_err(|e| {
                    anyhow::anyhow!(
                        "Failed to serialize location for {}: {}",
                        entity.id().as_str(),
                        e
                    )
                })?
            } else {
                "null".to_string()
            };

            // Save entity to database
            match tx.execute(
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
            ) {
                Ok(_) => {
                    debug!(
                        "Successfully saved entity {} in transaction",
                        entity.id().as_str()
                    );
                }
                Err(e) => {
                    error!(
                        "Failed to save entity {} in transaction: {}",
                        entity.id().as_str(),
                        e
                    );
                    return Err(anyhow::anyhow!("Transaction failed: {}", e));
                }
            }
        }

        // Process each relationship
        for relationship in relationships {
            // Serialize metadata
            let metadata_json = if !relationship.metadata.is_empty() {
                Some(serde_json::to_string(&relationship.metadata).map_err(|e| {
                    anyhow::anyhow!(
                        "Failed to serialize relationship metadata for {}: {}",
                        relationship.id.0,
                        e
                    )
                })?)
            } else {
                None
            };

            // Save relationship to database
            match tx.execute(
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
            ) {
                Ok(_) => {
                    debug!(
                        "Successfully saved relationship {} in transaction",
                        relationship.id.0
                    );
                }
                Err(e) => {
                    error!(
                        "Failed to save relationship {} in transaction: {}",
                        relationship.id.0, e
                    );
                    return Err(anyhow::anyhow!("Transaction failed: {}", e));
                }
            }
        }

        match tx.commit() {
            Ok(_) => {
                info!(
                    "Successfully committed transaction with {} entities and {} relationships",
                    entities.len(),
                    relationships.len()
                );
                Ok(())
            }
            Err(e) => {
                error!("Failed to commit transaction: {}", e);
                Err(anyhow::anyhow!("Failed to commit transaction: {}", e))
            }
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::entity::{Parameter, Visibility};
    use std::sync::Arc;
    use std::thread;
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

    #[test]
    fn test_concurrent_saves() {
        // Test concurrent access using the connection pool
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let db = Database::new(&db_path).unwrap();
        let db_arc = Arc::new(db);

        // Number of threads to use
        let thread_count = 10;
        let mut handles = vec![];

        // Create multiple threads, each saving entities concurrently
        for i in 0..thread_count {
            let db_clone = db_arc.clone();
            let handle = thread::spawn(move || {
                // Create a unique entity for this thread
                let id = EntityId::new(&format!("entity{}", i));
                let base = BaseEntity::new(
                    id.clone(),
                    format!("Entity{}", i),
                    EntityType::Function,
                    Some("test.rs".to_string()),
                );

                // Create a function entity
                let function = FunctionEntity {
                    base,
                    parameters: vec![],
                    return_type: Some(format!("Type{}", i)),
                    visibility: Visibility::Public,
                    is_async: false,
                    is_static: false,
                    is_constructor: false,
                    is_abstract: false,
                };

                // Save the entity
                db_clone.save_entity(&function).unwrap();
            });

            handles.push(handle);
        }

        // Wait for all threads to complete
        for handle in handles {
            handle.join().unwrap();
        }

        // Load entities and verify
        let entities = db_arc.load_entities().unwrap();

        // Check that all entities were saved
        assert_eq!(entities.len(), thread_count);

        // Verify each entity exists
        for i in 0..thread_count {
            let entity_id = format!("entity{}", i);
            let entity = entities.iter().find(|e| e.id().as_str() == entity_id);
            assert!(entity.is_some(), "Entity {} not found", entity_id);
        }
    }

    #[test]
    fn test_transaction_integrity() {
        // Test transaction integrity - all entities should be saved or none
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let db = Database::new(&db_path).unwrap();

        // Create 5 entities
        let mut entities: Vec<Box<dyn Entity>> = Vec::new();
        let mut entity_refs: Vec<&dyn Entity> = Vec::new();

        for i in 0..5 {
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
                return_type: Some(format!("Type{}", i)),
                visibility: Visibility::Public,
                is_async: false,
                is_static: false,
                is_constructor: false,
                is_abstract: false,
            };

            entities.push(Box::new(function));
        }

        // Get references for the transaction
        for entity in &entities {
            entity_refs.push(entity.as_ref());
        }

        // Create a relationship
        let rel = Relationship::new(
            crate::graph::relationship::RelationshipId::new("rel1"),
            EntityId::new("entity0"),
            EntityId::new("entity1"),
            RelationshipType::Calls,
        );

        // Save all in a transaction
        db.save_all_in_transaction(&entity_refs, &[&rel]).unwrap();

        // Load back and verify
        let loaded_entities = db.load_entities().unwrap();
        let loaded_relationships = db.load_relationships().unwrap();

        assert_eq!(loaded_entities.len(), 5);
        assert_eq!(loaded_relationships.len(), 1);
    }
}
