use crate::graph::{
    entity::{Entity, EntityType},
    KnowledgeGraph,
};
use crate::prompt::context_builder::build_context;
use crate::prompt::llm_integration::query_llm;
use anyhow::Result;
use regex::Regex;
use serde_json::{json, Map, Value};

/// Tiered query processor that uses different strategies based on query complexity
/// Query options for refining search results
pub struct QueryOptions {
    /// Format of the output (text, json)
    pub format: String,
    /// Filter results by entity type
    pub type_filter: Option<String>,
    /// Filter results by file path pattern
    pub path_filter: Option<String>,
    /// Only include exact ID matches
    pub exact_match: bool,
    /// Maximum number of results to return
    pub limit: usize,
    /// Skip LLM and only use direct knowledge graph queries
    pub no_llm: bool,
}

pub async fn run(
    instruction: &str,
    format: &str,
    type_filter: Option<&str>,
    path_filter: Option<&str>,
    exact_match: bool,
    limit: usize,
    no_llm: bool,
) -> Result<()> {
    println!("Querying knowledge graph: {}", instruction);

    // Load the KG
    let kg = KnowledgeGraph::load_from_file("knowledge_graph.json")?;

    // Set up query options
    let options = QueryOptions {
        format: format.to_string(),
        type_filter: type_filter.map(|s| s.to_string()),
        path_filter: path_filter.map(|s| s.to_string()),
        exact_match,
        limit,
        no_llm,
    };

    // Try to process the query using the most efficient method
    let response = match process_query(&kg, instruction, &options).await? {
        QueryResult::Direct(value) => {
            // Direct result from KG queries
            serde_json::to_string_pretty(&value)?
        }
        QueryResult::LLM(text) => {
            // Complex query that required LLM
            text
        }
    };

    // Format the output according to the requested format
    match format {
        "json" => {
            // Try to parse the response as JSON for structured output
            match serde_json::from_str::<serde_json::Value>(&response) {
                Ok(json) => println!("{}", serde_json::to_string_pretty(&json)?),
                Err(_) => println!("{}", response), // Fallback to plain text if not valid JSON
            }
        }
        _ => println!("{}", response), // Default to plain text
    }

    Ok(())
}

/// Represents different types of query results
enum QueryResult {
    /// Direct result from knowledge graph without LLM
    Direct(Value),
    /// Result that required LLM processing
    LLM(String),
}

/// Process a query using the most efficient method available
async fn process_query(
    kg: &KnowledgeGraph,
    instruction: &str,
    options: &QueryOptions,
) -> Result<QueryResult> {
    // 1. Try to match known direct query patterns first
    if let Some(result) = try_direct_query(kg, instruction, options)? {
        return Ok(QueryResult::Direct(apply_filters(result, options)));
    }

    // 2. Try pattern-based queries for common cases
    if let Some(result) = try_pattern_query(kg, instruction, options)? {
        return Ok(QueryResult::Direct(apply_filters(result, options)));
    }

    // 3. Fall back to LLM for complex semantic queries (unless disabled)
    if !options.no_llm {
        let context = build_context(kg, instruction);
        let api_key = std::env::var("OPENROUTER_API_KEY").unwrap_or_default();
        let response = query_llm(&context, &api_key).await?;
        return Ok(QueryResult::LLM(response));
    }

    // 4. If LLM is disabled and no patterns matched, do a general search
    let search_results = kg.search(instruction)?;
    let result = entities_to_json(search_results);
    Ok(QueryResult::Direct(apply_filters(result, options)))
}

/// Apply filters based on query options
fn apply_filters(mut result: Value, options: &QueryOptions) -> Value {
    // Skip filtering for non-array results
    if !result.is_array() {
        return result;
    }

    // Get the array to filter
    let mut entities = result.as_array().unwrap().clone();

    // Apply type filter if specified
    if let Some(type_filter) = &options.type_filter {
        let type_filter_lower = type_filter.to_lowercase();
        entities.retain(|entity| {
            if let Some(entity_type) = entity.get("type") {
                if let Some(type_str) = entity_type.as_str() {
                    return type_str.to_lowercase().contains(&type_filter_lower);
                }
            }
            false
        });
    }

    // Apply path filter if specified
    if let Some(path_filter) = &options.path_filter {
        let path_filter_lower = path_filter.to_lowercase();
        entities.retain(|entity| {
            if let Some(path) = entity.get("file_path") {
                if let Some(path_str) = path.as_str() {
                    return path_str.to_lowercase().contains(&path_filter_lower);
                }
            }
            false
        });
    }

    // Apply limit
    if entities.len() > options.limit {
        entities.truncate(options.limit);
    }

    Value::Array(entities)
}

/// Handle direct queries that can be mapped directly to KnowledgeGraph methods
fn try_direct_query(
    kg: &KnowledgeGraph,
    instruction: &str,
    _options: &QueryOptions,
) -> Result<Option<Value>> {
    // Lowercase instruction for case-insensitive matching
    let instruction_lower = instruction.to_lowercase();

    // Direct commands mapping
    match instruction_lower.as_str() {
        "list all entities" | "show all entities" => {
            let entities = kg.get_all_entities();
            let result = entities_to_json(entities);
            return Ok(Some(result));
        }
        "list all functions" | "show all functions" | "show functions" | "show funcs" => {
            let functions = kg.get_entities_by_type(&EntityType::Function);
            let result = entities_to_json(functions);
            return Ok(Some(result));
        }
        "list all methods" | "show all methods" | "show methods" => {
            let methods = kg.get_entities_by_type(&EntityType::Method);
            let result = entities_to_json(methods);
            return Ok(Some(result));
        }
        "list all classes" | "show all classes" | "show classes" => {
            let classes = kg.get_entities_by_type(&EntityType::Class);
            let result = entities_to_json(classes);
            return Ok(Some(result));
        }
        "list all modules" | "show all modules" | "show modules" => {
            let modules = kg.get_entities_by_type(&EntityType::Module);
            let result = entities_to_json(modules);
            return Ok(Some(result));
        }
        "list domain concepts" | "show domain concepts" => {
            let concepts = kg.get_domain_concepts();
            let result = json!(concepts
                .iter()
                .map(|c| {
                    json!({
                        "name": c.name(),
                        "id": c.id().as_str(),
                        "attributes": &c.attributes,
                        "description": &c.description
                    })
                })
                .collect::<Vec<_>>());
            return Ok(Some(result));
        }
        _ => {}
    }

    Ok(None)
}

/// Handle pattern-based queries that can be resolved with regex or simple parsing
fn try_pattern_query(
    kg: &KnowledgeGraph,
    instruction: &str,
    options: &QueryOptions,
) -> Result<Option<Value>> {
    // Search for entities pattern
    let search_pattern = Regex::new(r"(?i)^(search|find|lookup)(?:\s+for)?\s+(.+)$")?;
    if let Some(cap) = search_pattern.captures(instruction) {
        if let Some(query) = cap.get(2) {
            let search_query = query.as_str();
            let entities = kg.search(search_query)?;

            // If exact match is required, filter for exact name or ID matches
            let entities = if options.exact_match {
                entities
                    .into_iter()
                    .filter(|e| {
                        e.name().to_lowercase() == search_query.to_lowercase()
                            || e.id().as_str().to_lowercase() == search_query.to_lowercase()
                    })
                    .collect()
            } else {
                entities
            };

            let result = entities_to_json(entities);
            return Ok(Some(result));
        }
    }

    // Get entity details pattern
    let entity_pattern =
        Regex::new(r"(?i)^(show|get|describe)(?:\s+details(?:\s+for|of))?\s+entity\s+(.+)$")?;
    if let Some(cap) = entity_pattern.captures(instruction) {
        if let Some(entity_id_match) = cap.get(2) {
            let entity_id_str = entity_id_match.as_str();

            // Check if it's an exact ID or a name
            if entity_id_str.contains("::") {
                // Likely an ID
                let entities = kg.search(entity_id_str)?;
                if !entities.is_empty() {
                    return Ok(Some(entity_with_relationships_to_json(kg, entities[0])?));
                }
            } else {
                // Search by name
                let entities = kg.search(entity_id_str)?;
                if !entities.is_empty() {
                    return Ok(Some(entity_with_relationships_to_json(kg, entities[0])?));
                }
            }
        }
    }

    // Show paths between entities
    let path_pattern =
        Regex::new(r"(?i)^(show|find)(?:\s+paths?(?:\s+from|between))?\s+(.+?)(?:\s+to\s+)(.+)$")?;
    if let Some(cap) = path_pattern.captures(instruction) {
        if let (Some(from_match), Some(to_match)) = (cap.get(2), cap.get(3)) {
            let from_str = from_match.as_str();
            let to_str = to_match.as_str();

            // Find the entities
            let from_entities = kg.search(from_str)?;
            let to_entities = kg.search(to_str)?;

            if !from_entities.is_empty() && !to_entities.is_empty() {
                let from_id = from_entities[0].id();
                let to_id = to_entities[0].id();

                // Find paths between entities
                let paths = kg.find_paths(from_id, to_id, 5);
                let result = json!(paths
                    .iter()
                    .map(|path| {
                        path.iter()
                            .map(|e| {
                                json!({
                                    "id": e.id().as_str(),
                                    "name": e.name(),
                                    "type": format!("{:?}", e.entity_type())
                                })
                            })
                            .collect::<Vec<_>>()
                    })
                    .collect::<Vec<_>>());
                return Ok(Some(result));
            }
        }
    }

    // Related entities pattern
    let related_pattern = Regex::new(r"(?i)^(show|get|find)(?:\s+related(?:\s+to))?\s+(.+)$")?;
    if let Some(cap) = related_pattern.captures(instruction) {
        if let Some(entity_match) = cap.get(2) {
            let entity_query = entity_match.as_str();
            let entities = kg.search(entity_query)?;

            if !entities.is_empty() {
                let entity = entities[0];
                let related = kg.get_related_entities(entity.id(), None);
                let result = entities_to_json(related);
                return Ok(Some(result));
            }
        }
    }

    Ok(None)
}

/// Convert a list of entities to JSON
fn entities_to_json(entities: Vec<&dyn Entity>) -> Value {
    json!(entities
        .iter()
        .map(|e| {
            let mut entity_map = Map::new();
            entity_map.insert("id".to_string(), Value::String(e.id().as_str().to_string()));
            entity_map.insert("name".to_string(), Value::String(e.name().to_string()));
            entity_map.insert(
                "type".to_string(),
                Value::String(format!("{:?}", e.entity_type())),
            );

            if let Some(path) = e.file_path() {
                entity_map.insert("file_path".to_string(), Value::String(path.to_string()));
            }

            Value::Object(entity_map)
        })
        .collect::<Vec<_>>())
}

/// Convert an entity with its relationships to JSON
fn entity_with_relationships_to_json(kg: &KnowledgeGraph, entity: &dyn Entity) -> Result<Value> {
    let entity_id = entity.id();

    // Get relationship information
    let outgoing = kg.get_outgoing_relationships(entity_id);
    let incoming = kg.get_incoming_relationships(entity_id);

    // Build the entity details
    let mut entity_map = Map::new();
    entity_map.insert(
        "id".to_string(),
        Value::String(entity_id.as_str().to_string()),
    );
    entity_map.insert("name".to_string(), Value::String(entity.name().to_string()));
    entity_map.insert(
        "type".to_string(),
        Value::String(format!("{:?}", entity.entity_type())),
    );

    if let Some(path) = entity.file_path() {
        entity_map.insert("file_path".to_string(), Value::String(path.to_string()));
    }

    // Add metadata
    let metadata = entity.metadata();
    if !metadata.is_empty() {
        entity_map.insert("metadata".to_string(), json!(metadata));
    }

    // Add relationships
    entity_map.insert(
        "outgoing_relationships".to_string(),
        json!(outgoing
            .iter()
            .map(|r| {
                json!({
                    "type": format!("{:?}", r.relationship_type),
                    "target_id": r.target_id.as_str(),
                    "target_name": kg.get_entity(&r.target_id)
                        .map(|e| e.name().to_string())
                        .unwrap_or_else(|| "Unknown".to_string())
                })
            })
            .collect::<Vec<_>>()),
    );

    entity_map.insert(
        "incoming_relationships".to_string(),
        json!(incoming
            .iter()
            .map(|r| {
                json!({
                    "type": format!("{:?}", r.relationship_type),
                    "source_id": r.source_id.as_str(),
                    "source_name": kg.get_entity(&r.source_id)
                        .map(|e| e.name().to_string())
                        .unwrap_or_else(|| "Unknown".to_string())
                })
            })
            .collect::<Vec<_>>()),
    );

    Ok(Value::Object(entity_map))
}
