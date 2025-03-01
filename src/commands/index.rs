use anyhow::Result;
use ignore::WalkBuilder;
use std::collections::{HashMap, HashSet};
use std::time::Instant;

use crate::graph::entity::{
    BaseEntity, EntityId, EntityType, FunctionEntity, ModuleEntity, TypeEntity, VariableEntity,
};
use crate::graph::relationship::RelationshipType;
use crate::graph::KnowledgeGraph;
use crate::parser::domain_model::RelationType;
use crate::parser::language_support::{
    get_parser_for_file, DomainConcept, FunctionDefinition, TypeDefinition,
};

/// Main entry point for the indexing command
pub async fn run(
    path: &str,
    enable_domain_extraction: bool,
    domain_dir: &str,
    llm_provider: Option<&str>,
    llm_model: Option<&str>,
) -> Result<()> {
    println!("Indexing code at path: {}", path);
    let start_time = Instant::now();

    // Set environment variables for the LLM provider and model if specified
    if let Some(provider) = llm_provider {
        std::env::set_var("LLM_PROVIDER", provider);
    }

    if let Some(model) = llm_model {
        std::env::set_var("LLM_MODEL", model);
    }

    let mut kg = KnowledgeGraph::new();
    let mut function_map: HashMap<String, FunctionDefinition> = HashMap::new();
    let mut type_map: HashMap<String, TypeDefinition> = HashMap::new();
    let mut domain_concepts: HashMap<String, DomainConcept> = HashMap::new();

    // Track seen file paths to avoid duplicates
    let mut indexed_files = HashSet::new();

    // First pass: Collect entities
    println!("Pass 1: Collecting entities...");
    index_entities(
        path,
        &mut kg,
        &mut function_map,
        &mut type_map,
        &mut domain_concepts,
        &mut indexed_files,
    )?;

    // Second pass: Build relationships
    println!("Pass 2: Building relationships...");
    index_relationships(
        path,
        &mut kg,
        &function_map,
        &type_map,
        &domain_concepts,
        &indexed_files,
    )?;

    // Third pass: Infer domain concepts and their relationships
    println!("Pass 3: Inferring domain model from all source files...");
    infer_domain_model(
        &mut kg,
        &mut domain_concepts,
        enable_domain_extraction,
        domain_dir,
    )
    .await?;

    let duration = start_time.elapsed();
    kg.save_to_file("knowledge_graph.json")?;

    // Print indexing statistics
    let entity_count = kg.get_all_entities().len();
    let relationship_count = kg.get_relationship_count();
    let domain_concept_count = kg.get_domain_concepts().len();

    println!("Indexing complete in {:.2?}.", duration);
    println!("Knowledge Graph Statistics:");
    println!("  - {} entities indexed", entity_count);
    println!("  - {} relationships established", relationship_count);
    println!("  - {} domain concepts inferred", domain_concept_count);
    println!("Graph saved to knowledge_graph.json.");

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

        indexed_files.insert(file_path.clone());

        if let Some(mut parser) = get_parser_for_file(path) {
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

        if let Some(mut parser) = get_parser_for_file(path) {
            let content = std::fs::read_to_string(path)?;

            // Process calls between functions
            let calls = parser.parse_calls(&content)?;
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
                        println!("Failed to create call relationship: error: {}", e);
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
                            println!("Failed to create inheritance relationship: error: {}", e);
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
                            println!(
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
                            println!(
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
                    println!("Failed to create imports relationship: error: {}", e);
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
                println!(
                    "Failed to create domain representation relationship: error: {}",
                    e
                );
            }
        }
    }

    // Use LLM to extract domain entities from important files
    println!("Using LLM to extract domain models...");

    if enable_domain_extraction {
        use std::collections::HashSet;
        use std::fs;

        use crate::parser::domain_model::EntityType as DomainEntityType;

        let extractor = LlmModelExtractor::new();
        let mut processed_files = HashSet::new();
        let mut domain_entity_count = 0;

        // Process key model/entity files first - they likely contain domain entities
        // We'll look at the specified domain directory
        println!("Analyzing directory for domain extraction: {}", domain_dir);

        // Process code in the specified directory, respecting .gitignore
        let walker = WalkBuilder::new(&domain_dir)
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
            // Get file extension to check if it's a source file
            let extension = file_path_obj
                .extension()
                .and_then(|ext| ext.to_str())
                .unwrap_or("");

            // Skip non-source files (binaries, images, etc.)
            let is_source_file = matches!(
                extension,
                "rs" | "py" | "js" | "ts" | "c" | "cpp" | "h" | "hpp" | "java" | "go" | "rb"
            );

            if !is_source_file {
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

            println!("Analyzing file for domain entities: {}", file_path);

            // Extract insights from more files but limit LLM calls for large codebases
            // Skip very large files - they might exceed token limits even with truncation
            if content.len() > 100000 {
                println!(
                    "  Skipping very large file: {} ({} bytes)",
                    file_path,
                    content.len()
                );
                continue;
            }

            // Extract domain entities using LLM (with await)
            let domain_entities = match extractor.extract_domain_model(&content, &file_path).await {
                Ok(entities) => entities,
                Err(e) => {
                    println!("  Error extracting domain entities: {}", e);
                    continue;
                }
            };

            // Add each entity to the graph
            for entity in domain_entities {
                println!(
                    "  Found domain entity: {} ({})",
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
                let attributes: Vec<String> = entity.attributes.keys().map(|k| k.clone()).collect();

                // Create domain concept entity
                let domain_concept = DomainConceptEntity {
                    base,
                    attributes,
                    description: entity.description,
                    confidence: 0.8, // High confidence since LLM extracted it
                };

                // Add to graph
                if let Err(e) = kg.add_entity(domain_concept) {
                    println!("  Error adding domain entity to graph: {}", e);
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

        println!(
            "LLM domain extraction complete: {} entities found",
            domain_entity_count
        );
    } else {
        println!(
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

            for j in (i + 1)..domain_entities.len() {
                let entity2 = domain_entities[j];
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
                        if kg.find_paths(tech1.id(), tech2.id(), 3).len() > 0 {
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
            println!("Failed to create domain relationship: error: {}", e);
        }
    }

    Ok(())
}
