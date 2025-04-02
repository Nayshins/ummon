use anyhow::Result;
use chrono::prelude::*;
use ignore::WalkBuilder;
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::time::Instant;

use crate::graph::entity::{
    BaseEntity, Entity, EntityId, EntityType, FunctionEntity, ModuleEntity, TypeEntity,
    VariableEntity,
};
use crate::graph::relationship::{Relationship, RelationshipType};
use crate::graph::KnowledgeGraph;
use crate::parser::domain_model::RelationType;
use crate::parser::language_support::{
    get_parser_for_file, is_supported_source_file, DomainConcept, FunctionDefinition,
    TypeDefinition,
};

/// Main entry point for the indexing command
pub async fn run(
    path: &str,
    full_rebuild: bool,
    enable_domain_extraction: bool,
    domain_dir: &str,
    llm_provider: Option<&str>,
    llm_model: Option<&str>,
) -> Result<()> {
    tracing::info!("Indexing code at path: {}", path);
    let start_time = Instant::now();

    // Set environment variables for the LLM provider and model if specified
    if let Some(provider) = llm_provider {
        std::env::set_var("LLM_PROVIDER", provider);
    }

    if let Some(model) = llm_model {
        std::env::set_var("LLM_MODEL", model);
    }

    let db = crate::db::get_database("ummon.db")?;
    let mut kg = KnowledgeGraph::new_with_db(db.clone());

    let mut function_map: HashMap<String, FunctionDefinition> = HashMap::new();
    let mut type_map: HashMap<String, TypeDefinition> = HashMap::new();
    let mut domain_concepts: HashMap<String, DomainConcept> = HashMap::new();
    let mut indexed_files = HashSet::new();

    if full_rebuild {
        tracing::info!("Performing full rebuild of the knowledge graph...");
        kg.purge()?;
    } else {
        tracing::info!("Performing incremental update of the knowledge graph...");
        let last_index_time = db.get_metadata("last_index_time")?;

        if let Some(timestamp) = last_index_time {
            let modified_files = get_modified_files(path, &timestamp)?;

            if modified_files.is_empty() {
                tracing::info!("No files modified since last index. Nothing to do.");
                return Ok(());
            }

            tracing::info!("Found {} modified files to update", modified_files.len());
            kg.prune(&modified_files)?;

            for file in modified_files {
                indexed_files.insert(file);
            }
        } else {
            tracing::info!("No previous index found, performing full initial index.");
        }
    }

    tracing::info!("Pass 1: Collecting entities...");
    if indexed_files.is_empty() {
        index_entities(
            path,
            &mut kg,
            &mut function_map,
            &mut type_map,
            &mut domain_concepts,
            &mut indexed_files,
        )?;
    } else {
        index_specific_entities(
            &indexed_files,
            &mut kg,
            &mut function_map,
            &mut type_map,
            &mut domain_concepts,
        )?;
    }

    tracing::info!("Pass 2: Building relationships...");
    if indexed_files.is_empty() {
        index_relationships(
            path,
            &mut kg,
            &function_map,
            &type_map,
            &domain_concepts,
            &indexed_files,
        )?;
    } else {
        index_specific_relationships(
            &indexed_files,
            &mut kg,
            &function_map,
            &type_map,
            &domain_concepts,
        )?;
    }

    tracing::info!("Pass 3: Inferring domain model from source files...");
    infer_domain_model(
        &mut kg,
        &mut domain_concepts,
        enable_domain_extraction,
        domain_dir,
    )
    .await?;

    let duration = start_time.elapsed();

    let entities: Vec<&dyn Entity> = kg.get_all_entities();
    let relationships = kg.get_all_relationships()?;
    let rel_refs: Vec<&Relationship> = relationships.iter().collect();

    db.save_all_in_transaction(&entities, &rel_refs)?;

    let now = Utc::now().to_rfc3339();
    db.set_metadata("last_index_time", &now)?;

    let entity_count = kg.get_all_entities().len();
    let relationship_count = kg.get_relationship_count();
    let domain_concept_count = kg.get_domain_concepts().len();

    tracing::info!("Indexing complete in {:.2?}.", duration);
    tracing::info!("Knowledge Graph Statistics:");
    tracing::info!("  - {} entities indexed", entity_count);
    tracing::info!("  - {} relationships established", relationship_count);
    tracing::info!("  - {} domain concepts inferred", domain_concept_count);
    tracing::info!("Graph saved to database 'ummon.db'.");

    Ok(())
}

/// First pass: Index all entities from code
fn index_entities(
    path: &str,
    kg: &mut KnowledgeGraph,
    function_map: &mut HashMap<String, FunctionDefinition>,
    type_map: &mut HashMap<String, TypeDefinition>,
    domain_concepts: &mut HashMap<String, DomainConcept>,
    indexed_files: &mut HashSet<String>,
) -> Result<()> {
    let walker = WalkBuilder::new(path).hidden(false).ignore(true).build();

    for entry in walker {
        let entry = entry?;
        let path = entry.path();

        if !path.is_file() {
            continue;
        }

        let file_path = path.to_string_lossy().to_string();

        // Skip if already processed
        if indexed_files.contains(&file_path) {
            continue;
        }

        // Skip if file has unsupported extension
        if !is_supported_source_file(path) {
            tracing::debug!("Skipping unsupported file type: {}", file_path);
            continue;
        }

        indexed_files.insert(file_path.clone());

        if let Ok(Some(mut parser)) = get_parser_for_file(path) {
            let content = std::fs::read_to_string(path)?;

            // Process file as a module
            let module_info = parser.parse_modules(&content, &file_path)?;
            let module_id = EntityId::new(&file_path);

            let module_entity = ModuleEntity {
                base: BaseEntity::new(
                    module_id.clone(),
                    module_info.name.clone(),
                    EntityType::Module,
                    Some(file_path.clone()),
                ),
                path: file_path.clone(),
                children: Vec::new(),
                imports: module_info
                    .imports
                    .iter()
                    .map(|imp| imp.module_name.clone())
                    .collect(),
            };

            // Add module to the knowledge graph
            kg.add_entity(module_entity)?;

            // Process functions
            let functions = parser.parse_functions(&content, &file_path)?;
            for func in functions {
                let key = format!("{}::{}", func.file_path, func.name);
                function_map.insert(key.clone(), func.clone());

                // Create entity ID and entity type
                let entity_id = EntityId::new(&key);
                let entity_type = match func.kind {
                    crate::parser::language_support::FunctionKind::Function => EntityType::Function,
                    crate::parser::language_support::FunctionKind::Method => EntityType::Method,
                    crate::parser::language_support::FunctionKind::Constructor => {
                        EntityType::Method
                    }
                    _ => EntityType::Function,
                };

                // Extract documentation if available
                let doc = parser.extract_documentation(&content, &func.location)?;

                let mut base = BaseEntity::new(
                    entity_id.clone(),
                    func.name.clone(),
                    entity_type,
                    Some(func.file_path.clone()),
                );

                base.location = Some(func.location.clone());
                base.documentation = doc;
                base.containing_entity = func
                    .containing_type
                    .as_ref()
                    .map(|t| EntityId::new(&format!("type::{}", t)));

                let function_entity = FunctionEntity {
                    base,
                    parameters: func.parameters.clone(),
                    return_type: None,
                    visibility: func.visibility.clone(),
                    is_async: false,
                    is_static: false,
                    is_constructor: func.kind
                        == crate::parser::language_support::FunctionKind::Constructor,
                    is_abstract: false,
                };

                kg.add_entity(function_entity)?;
            }

            // Process types (classes, structs, etc.)
            let types = parser.parse_types(&content, &file_path)?;
            for type_def in types {
                let key = format!("{}::{}", type_def.file_path, type_def.name);
                type_map.insert(key.clone(), type_def.clone());

                // Create entity ID and entity type
                let entity_id = EntityId::new(&format!("type::{}", key));
                let entity_type = match type_def.kind {
                    crate::parser::language_support::TypeKind::Class => EntityType::Class,
                    crate::parser::language_support::TypeKind::Struct => EntityType::Struct,
                    crate::parser::language_support::TypeKind::Interface => EntityType::Interface,
                    crate::parser::language_support::TypeKind::Trait => EntityType::Trait,
                    crate::parser::language_support::TypeKind::Enum => EntityType::Enum,
                    _ => EntityType::Type,
                };

                let mut base = BaseEntity::new(
                    entity_id.clone(),
                    type_def.name.clone(),
                    entity_type,
                    Some(type_def.file_path.clone()),
                );

                base.location = Some(type_def.location.clone());
                base.documentation = type_def.documentation.clone();

                let type_entity = TypeEntity {
                    base,
                    fields: type_def
                        .fields
                        .iter()
                        .map(|f| EntityId::new(&format!("field::{}::{}", key, f.name)))
                        .collect(),
                    methods: type_def
                        .methods
                        .iter()
                        .map(|m| EntityId::new(&format!("method::{}::{}", key, m)))
                        .collect(),
                    supertypes: type_def
                        .super_types
                        .iter()
                        .map(|s| EntityId::new(&format!("type::{}", s)))
                        .collect(),
                    visibility: type_def.visibility.clone(),
                    is_abstract: false,
                };

                kg.add_entity(type_entity)?;

                // Add fields as entities
                for field in &type_def.fields {
                    let field_id = EntityId::new(&format!("{}::field::{}", key, field.name));

                    let mut base = BaseEntity::new(
                        field_id.clone(),
                        field.name.clone(),
                        EntityType::Field,
                        Some(type_def.file_path.clone()),
                    );

                    base.location = Some(field.location.clone());
                    base.containing_entity = Some(entity_id.clone());

                    let var_entity = VariableEntity {
                        base,
                        type_annotation: field.type_annotation.clone(),
                        visibility: field.visibility.clone(),
                        is_const: false,
                        is_static: field.is_static,
                    };

                    kg.add_entity(var_entity)?;
                }
            }

            // Extract domain concepts
            let concepts = parser.infer_domain_concepts(&content, &file_path)?;
            for concept in concepts {
                domain_concepts.insert(concept.name.clone(), concept);
            }
        }
    }

    Ok(())
}

/// Second pass: Build relationships between entities
fn index_relationships(
    path: &str,
    kg: &mut KnowledgeGraph,
    _function_map: &HashMap<String, FunctionDefinition>,
    type_map: &HashMap<String, TypeDefinition>,
    _domain_concepts: &HashMap<String, DomainConcept>,
    indexed_files: &HashSet<String>,
) -> Result<()> {
    let walker = WalkBuilder::new(path).hidden(false).ignore(true).build();

    for entry in walker {
        let entry = entry?;
        let path = entry.path();

        if !path.is_file() {
            continue;
        }

        let file_path = path.to_string_lossy().to_string();

        // Skip if not previously indexed
        if !indexed_files.contains(&file_path) {
            continue;
        }

        // Skip if file has unsupported extension
        if !is_supported_source_file(path) {
            continue;
        }

        if let Ok(Some(mut parser)) = get_parser_for_file(path) {
            let content = std::fs::read_to_string(path)?;

            // Process calls between functions
            let calls = parser.parse_calls(&content, &file_path)?;
            for call in calls {
                // Add relationship between entities if they exist
                // Create a more tolerant implementation that will try to make relationships even if exact IDs don't match

                // First try with the callee function name being used in the calling file
                let caller_id = EntityId::new(&format!("{}::{}", file_path, call.callee_name));

                if let Some(callee_key) = &call.fully_qualified_name {
                    // For the target, try to find the most likely target entity
                    let callee_id = EntityId::new(callee_key);

                    if let Err(e) =
                        kg.create_relationship(caller_id, callee_id, RelationshipType::Calls)
                    {
                        tracing::warn!("Failed to create call relationship: error: {}", e);
                    }
                }
            }

            // Create type inheritance/implementation relationships
            for (key, type_def) in type_map {
                if type_def.file_path == file_path {
                    let type_id = EntityId::new(key);

                    // Create relationships to super types
                    for super_type in &type_def.super_types {
                        let super_id = EntityId::new(super_type);

                        // Determine if this is inheritance or implementation
                        let rel_type = match type_def.kind {
                            crate::parser::language_support::TypeKind::Class => {
                                RelationshipType::Inherits
                            }
                            crate::parser::language_support::TypeKind::Struct => {
                                RelationshipType::Inherits
                            }
                            _ => RelationshipType::Implements,
                        };

                        if let Err(e) = kg.create_relationship(type_id.clone(), super_id, rel_type)
                        {
                            tracing::warn!(
                                "Failed to create inheritance relationship: error: {}",
                                e
                            );
                        }
                    }

                    // Create containment relationships to methods
                    for method in &type_def.methods {
                        let method_id = EntityId::new(&format!("{}::{}", key, method));
                        if let Err(e) = kg.create_relationship(
                            type_id.clone(),
                            method_id,
                            RelationshipType::Contains,
                        ) {
                            tracing::warn!(
                                "Failed to create contains relationship (method): error: {}",
                                e
                            );
                        }
                    }

                    // Create containment relationships to fields
                    for field in &type_def.fields {
                        let field_id = EntityId::new(&format!("{}::field::{}", key, field.name));
                        if let Err(e) = kg.create_relationship(
                            type_id.clone(),
                            field_id,
                            RelationshipType::Contains,
                        ) {
                            tracing::warn!(
                                "Failed to create contains relationship (field): error: {}",
                                e
                            );
                        }
                    }
                }
            }

            // Process the module's imports
            let module_id = EntityId::new(&file_path);
            let module_info = parser.parse_modules(&content, &file_path)?;

            for import in &module_info.imports {
                let imported_module_id = EntityId::new(&import.module_name);
                if let Err(e) = kg.create_relationship(
                    module_id.clone(),
                    imported_module_id,
                    RelationshipType::Imports,
                ) {
                    tracing::warn!("Failed to create imports relationship: error: {}", e);
                }
            }
        }
    }

    Ok(())
}

/// Get files that have been modified since the last indexing
fn get_modified_files(path: &str, last_index_time: &str) -> Result<Vec<String>> {
    let last_index_datetime = DateTime::parse_from_rfc3339(last_index_time)
        .map_err(|e| anyhow::anyhow!("Failed to parse last index time: {}", e))?;

    let last_index_utc = last_index_datetime.with_timezone(&Utc);
    let walker = WalkBuilder::new(path).hidden(false).ignore(true).build();
    let mut modified_files = Vec::new();

    for entry in walker {
        let entry = entry?;
        let path = entry.path();

        if !path.is_file() || !is_supported_source_file(path) {
            continue;
        }

        if let Ok(metadata) = std::fs::metadata(path) {
            if let Ok(modified) = metadata.modified() {
                let modified_time = DateTime::<Utc>::from(modified);

                if modified_time > last_index_utc {
                    modified_files.push(path.to_string_lossy().to_string());
                }
            }
        }
    }

    Ok(modified_files)
}

/// Index entities from a specific set of files
fn index_specific_entities(
    file_paths: &HashSet<String>,
    kg: &mut KnowledgeGraph,
    function_map: &mut HashMap<String, FunctionDefinition>,
    type_map: &mut HashMap<String, TypeDefinition>,
    domain_concepts: &mut HashMap<String, DomainConcept>,
) -> Result<()> {
    for file_path in file_paths {
        let path = Path::new(file_path);

        if !path.is_file() || !is_supported_source_file(path) {
            continue;
        }

        if let Ok(Some(mut parser)) = get_parser_for_file(path) {
            let content = std::fs::read_to_string(path)?;

            let module_info = parser.parse_modules(&content, file_path)?;
            let module_id = EntityId::new(file_path);

            let module_entity = ModuleEntity {
                base: BaseEntity::new(
                    module_id.clone(),
                    module_info.name.clone(),
                    EntityType::Module,
                    Some(file_path.clone()),
                ),
                path: file_path.clone(),
                children: Vec::new(),
                imports: module_info
                    .imports
                    .iter()
                    .map(|imp| imp.module_name.clone())
                    .collect(),
            };

            kg.add_entity(module_entity)?;

            let functions = parser.parse_functions(&content, file_path)?;
            for func in functions {
                let key = format!("{}::{}", func.file_path, func.name);
                function_map.insert(key.clone(), func.clone());

                let entity_id = EntityId::new(&key);
                let entity_type = match func.kind {
                    crate::parser::language_support::FunctionKind::Function => EntityType::Function,
                    crate::parser::language_support::FunctionKind::Method => EntityType::Method,
                    crate::parser::language_support::FunctionKind::Constructor => {
                        EntityType::Method
                    }
                    _ => EntityType::Function,
                };

                let doc = parser.extract_documentation(&content, &func.location)?;

                let mut base = BaseEntity::new(
                    entity_id.clone(),
                    func.name.clone(),
                    entity_type,
                    Some(func.file_path.clone()),
                );

                base.location = Some(func.location.clone());
                base.documentation = doc;
                base.containing_entity = func
                    .containing_type
                    .as_ref()
                    .map(|t| EntityId::new(&format!("type::{}", t)));

                let function_entity = FunctionEntity {
                    base,
                    parameters: func.parameters.clone(),
                    return_type: None,
                    visibility: func.visibility.clone(),
                    is_async: false,
                    is_static: false,
                    is_constructor: func.kind
                        == crate::parser::language_support::FunctionKind::Constructor,
                    is_abstract: false,
                };

                kg.add_entity(function_entity)?;
            }

            let types = parser.parse_types(&content, file_path)?;
            for type_def in types {
                let key = format!("{}::{}", type_def.file_path, type_def.name);
                type_map.insert(key.clone(), type_def.clone());

                let entity_id = EntityId::new(&format!("type::{}", key));
                let entity_type = match type_def.kind {
                    crate::parser::language_support::TypeKind::Class => EntityType::Class,
                    crate::parser::language_support::TypeKind::Struct => EntityType::Struct,
                    crate::parser::language_support::TypeKind::Interface => EntityType::Interface,
                    crate::parser::language_support::TypeKind::Trait => EntityType::Trait,
                    crate::parser::language_support::TypeKind::Enum => EntityType::Enum,
                    _ => EntityType::Type,
                };

                let mut base = BaseEntity::new(
                    entity_id.clone(),
                    type_def.name.clone(),
                    entity_type,
                    Some(type_def.file_path.clone()),
                );

                base.location = Some(type_def.location.clone());
                base.documentation = type_def.documentation.clone();

                let type_entity = TypeEntity {
                    base,
                    fields: type_def
                        .fields
                        .iter()
                        .map(|f| EntityId::new(&format!("field::{}::{}", key, f.name)))
                        .collect(),
                    methods: type_def
                        .methods
                        .iter()
                        .map(|m| EntityId::new(&format!("method::{}::{}", key, m)))
                        .collect(),
                    supertypes: type_def
                        .super_types
                        .iter()
                        .map(|s| EntityId::new(&format!("type::{}", s)))
                        .collect(),
                    visibility: type_def.visibility.clone(),
                    is_abstract: false,
                };

                kg.add_entity(type_entity)?;

                for field in &type_def.fields {
                    let field_id = EntityId::new(&format!("{}::field::{}", key, field.name));

                    let mut base = BaseEntity::new(
                        field_id.clone(),
                        field.name.clone(),
                        EntityType::Field,
                        Some(type_def.file_path.clone()),
                    );

                    base.location = Some(field.location.clone());
                    base.containing_entity = Some(entity_id.clone());

                    let var_entity = VariableEntity {
                        base,
                        type_annotation: field.type_annotation.clone(),
                        visibility: field.visibility.clone(),
                        is_const: false,
                        is_static: field.is_static,
                    };

                    kg.add_entity(var_entity)?;
                }
            }

            let concepts = parser.infer_domain_concepts(&content, file_path)?;
            for concept in concepts {
                domain_concepts.insert(concept.name.clone(), concept);
            }
        }
    }

    Ok(())
}

/// Build relationships between entities for specific files
fn index_specific_relationships(
    file_paths: &HashSet<String>,
    kg: &mut KnowledgeGraph,
    _function_map: &HashMap<String, FunctionDefinition>,
    type_map: &HashMap<String, TypeDefinition>,
    _domain_concepts: &HashMap<String, DomainConcept>,
) -> Result<()> {
    for file_path in file_paths {
        let path = Path::new(file_path);

        if !path.is_file() || !is_supported_source_file(path) {
            continue;
        }

        if let Ok(Some(mut parser)) = get_parser_for_file(path) {
            let content = std::fs::read_to_string(path)?;

            let calls = parser.parse_calls(&content, file_path)?;
            for call in calls {
                let caller_id = EntityId::new(&format!("{}::{}", file_path, call.callee_name));

                if let Some(callee_key) = &call.fully_qualified_name {
                    let callee_id = EntityId::new(callee_key);

                    if let Err(e) =
                        kg.create_relationship(caller_id, callee_id, RelationshipType::Calls)
                    {
                        tracing::warn!("Failed to create call relationship: error: {}", e);
                    }
                }
            }

            for (key, type_def) in type_map {
                if type_def.file_path == *file_path {
                    let type_id = EntityId::new(key);

                    for super_type in &type_def.super_types {
                        let super_id = EntityId::new(super_type);

                        let rel_type = match type_def.kind {
                            crate::parser::language_support::TypeKind::Class
                            | crate::parser::language_support::TypeKind::Struct => {
                                RelationshipType::Inherits
                            }
                            _ => RelationshipType::Implements,
                        };

                        if let Err(e) = kg.create_relationship(type_id.clone(), super_id, rel_type)
                        {
                            tracing::warn!(
                                "Failed to create inheritance relationship: error: {}",
                                e
                            );
                        }
                    }

                    for method in &type_def.methods {
                        let method_id = EntityId::new(&format!("{}::{}", key, method));
                        if let Err(e) = kg.create_relationship(
                            type_id.clone(),
                            method_id,
                            RelationshipType::Contains,
                        ) {
                            tracing::warn!(
                                "Failed to create contains relationship (method): error: {}",
                                e
                            );
                        }
                    }

                    for field in &type_def.fields {
                        let field_id = EntityId::new(&format!("{}::field::{}", key, field.name));
                        if let Err(e) = kg.create_relationship(
                            type_id.clone(),
                            field_id,
                            RelationshipType::Contains,
                        ) {
                            tracing::warn!(
                                "Failed to create contains relationship (field): error: {}",
                                e
                            );
                        }
                    }
                }
            }

            let module_id = EntityId::new(file_path);
            let module_info = parser.parse_modules(&content, file_path)?;

            for import in &module_info.imports {
                let imported_module_id = EntityId::new(&import.module_name);
                if let Err(e) = kg.create_relationship(
                    module_id.clone(),
                    imported_module_id,
                    RelationshipType::Imports,
                ) {
                    tracing::warn!("Failed to create imports relationship: error: {}", e);
                }
            }
        }
    }

    Ok(())
}

/// Third pass: Infer domain model from code entities
async fn infer_domain_model(
    kg: &mut KnowledgeGraph,
    domain_concepts: &mut HashMap<String, DomainConcept>,
    enable_domain_extraction: bool,
    domain_dir: &str,
) -> Result<()> {
    use crate::graph::entity::{BaseEntity, DomainConceptEntity, EntityId, EntityType};
    use crate::graph::relationship::RelationshipType;
    use crate::parser::domain_model::DomainModelBuilder;
    use crate::prompt::domain_extraction::LlmModelExtractor;

    // Combine all extracted domain concepts into the graph (from language parsers)
    for (name, concept) in domain_concepts {
        let entity_id = EntityId::new(name);

        let base = BaseEntity::new(
            entity_id.clone(),
            name.clone(),
            EntityType::DomainConcept,
            None,
        );

        let domain_entity = DomainConceptEntity {
            base,
            attributes: concept.attributes.clone(),
            description: concept.description.clone(),
            confidence: concept.confidence,
        };

        kg.add_entity(domain_entity)?;

        // Create relationships to technical entities
        for entity_ref in &concept.related_entities {
            let related_id = EntityId::new(entity_ref);
            if let Err(e) = kg.create_relationship(
                entity_id.clone(),
                related_id,
                RelationshipType::RepresentedBy,
            ) {
                tracing::warn!(
                    "Failed to create domain representation relationship: error: {}",
                    e
                );
            }
        }
    }

    // Use LLM to extract domain entities from important files
    tracing::info!("Using LLM to extract domain models...");

    if enable_domain_extraction {
        use std::collections::HashSet;
        use std::fs;

        use crate::parser::domain_model::EntityType as DomainEntityType;

        let extractor = LlmModelExtractor::new();
        let mut processed_files = HashSet::new();
        let mut domain_entity_count = 0;

        // Process key model/entity files first - they likely contain domain entities
        // We'll look at the specified domain directory
        tracing::info!("Analyzing directory for domain extraction: {}", domain_dir);

        // Process code in the specified directory, respecting .gitignore
        let walker = WalkBuilder::new(domain_dir)
            .hidden(false)
            .ignore(true) // Respect .gitignore files
            .git_global(true) // Use global git ignore files
            .git_exclude(true) // Use git exclude files
            .build();

        for entry in walker {
            let entry = entry?;
            let file_path_obj = entry.path();

            if !file_path_obj.is_file() {
                continue;
            }

            let file_path = file_path_obj.to_string_lossy().to_string();

            // Skip if already processed
            if processed_files.contains(&file_path) {
                continue;
            }

            // Process all source files, not just domain-specific files
            // Skip non-source files (binaries, images, etc.)
            if !is_supported_source_file(file_path_obj) {
                continue;
            }

            processed_files.insert(file_path.clone());

            // Read the file content
            let content = match fs::read_to_string(file_path_obj) {
                Ok(content) => content,
                Err(_) => continue, // Skip if can't read
            };

            // Skip empty or very small files
            if content.len() < 100 {
                continue;
            }

            tracing::info!("Analyzing file for domain entities: {}", file_path);

            // Extract insights from more files but limit LLM calls for large codebases
            // Skip very large files - they might exceed token limits even with truncation
            if content.len() > 100000 {
                tracing::info!(
                    "Skipping very large file: {} ({} bytes)",
                    file_path,
                    content.len()
                );
                continue;
            }

            // Extract domain entities using LLM (with await)
            let domain_entities = match extractor.extract_domain_model(&content, &file_path).await {
                Ok(entities) => entities,
                Err(e) => {
                    tracing::error!("Error extracting domain entities: {}", e);
                    continue;
                }
            };

            // Add each entity to the graph
            for entity in domain_entities {
                tracing::info!(
                    "Found domain entity: {} ({})",
                    entity.name,
                    match entity.entity_type {
                        DomainEntityType::Class => "Class",
                        DomainEntityType::Struct => "Struct",
                        DomainEntityType::Enum => "Enum",
                        DomainEntityType::Interface => "Interface",
                    }
                );

                // Create entity ID
                let entity_id = EntityId::new(&entity.name);

                // Create base entity
                let base = BaseEntity::new(
                    entity_id.clone(),
                    entity.name.clone(),
                    EntityType::DomainConcept,
                    Some(file_path.clone()),
                );

                // Convert attributes to strings
                let attributes: Vec<String> = entity.attributes.keys().cloned().collect();

                // Create domain concept entity
                let domain_concept = DomainConceptEntity {
                    base,
                    attributes,
                    description: entity.description,
                    confidence: 0.8, // High confidence since LLM extracted it
                };

                // Add to graph
                if let Err(e) = kg.add_entity(domain_concept) {
                    tracing::error!("Error adding domain entity to graph: {}", e);
                    continue;
                }

                domain_entity_count += 1;

                // Add relationships
                for relationship in entity.relationships {
                    let target_id = EntityId::new(&relationship.target_entity);

                    let rel_type = match relationship.relation_type {
                        RelationType::Inherits => RelationshipType::Inherits,
                        RelationType::Contains => RelationshipType::Contains,
                        RelationType::References => RelationshipType::References,
                        RelationType::Implements => RelationshipType::Implements,
                    };

                    let _ = kg.create_relationship(entity_id.clone(), target_id, rel_type);
                }
            }
        }

        tracing::info!(
            "LLM domain extraction complete: {} entities found",
            domain_entity_count
        );
    } else {
        tracing::info!(
            "Skipping LLM-based domain extraction (use --enable-domain-extraction flag to enable)"
        );
    }

    // Infer relationships between domain concepts
    // We need to collect all data first before modifying the knowledge graph
    let relationships_to_create: Vec<(EntityId, EntityId)> = {
        let domain_entities: Vec<_> = kg.get_entities_by_type(&EntityType::DomainConcept);
        let mut to_create = Vec::new();

        for i in 0..domain_entities.len() {
            let entity1 = domain_entities[i];
            let entity1_id = entity1.id().clone();

            // Find technical entities related to this domain concept
            let tech_entities1 =
                kg.get_related_entities(&entity1_id, Some(&RelationshipType::RepresentedBy));

            for entity2 in domain_entities.iter().skip(i + 1) {
                let entity2_id = entity2.id().clone();

                // Find technical entities related to the second domain concept
                let tech_entities2 =
                    kg.get_related_entities(&entity2_id, Some(&RelationshipType::RepresentedBy));

                // Check for overlapping technical entities
                let mut has_relationship = false;

                // If any technical entity of concept1 relates to any technical entity of concept2
                for tech1 in &tech_entities1 {
                    for tech2 in &tech_entities2 {
                        // Check if there's any relationship between these technical entities
                        if !kg.find_paths(tech1.id(), tech2.id(), 3).is_empty() {
                            has_relationship = true;
                            break;
                        }
                    }
                    if has_relationship {
                        break;
                    }
                }

                // If a relationship is found, schedule it for creation
                if has_relationship {
                    to_create.push((entity1_id.clone(), entity2_id.clone()));
                }
            }
        }

        to_create
    };

    // Now create all the relationships
    for (entity1_id, entity2_id) in relationships_to_create {
        if let Err(e) = kg.create_relationship(
            entity1_id.clone(),
            entity2_id.clone(),
            RelationshipType::RelatesTo,
        ) {
            tracing::warn!("Failed to create domain relationship: error: {}", e);
        }
    }

    Ok(())
}
