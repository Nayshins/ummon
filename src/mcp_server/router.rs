use serde_json::Value;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use crate::graph::entity::EntityType;
use crate::graph::knowledge_graph::KnowledgeGraph;
use crate::mcp_core::{
    CapabilitiesBuilder, Content, Resource, ResourceError, Router, ServerCapabilities, Tool,
    ToolError,
};

/// UmmonRouter implements the Router trait and handles Ummon-specific functionality
pub struct UmmonRouter {
    knowledge_graph: Arc<KnowledgeGraph>,
}

impl UmmonRouter {
    pub fn new(knowledge_graph: Arc<KnowledgeGraph>) -> Self {
        Self { knowledge_graph }
    }

    fn debug_graph_tool(&self) -> Result<Vec<Content>, ToolError> {
        // Get entity count
        let entity_count = self.knowledge_graph.get_all_entities().len();
        let relationship_count = self.knowledge_graph.get_relationship_count();

        // Get a sample of 5 entities to verify content
        let sample_entities = self
            .knowledge_graph
            .get_all_entities()
            .into_iter()
            .take(5)
            .map(|e| format!("- {} ({}): {}", e.id(), e.entity_type(), e.name()))
            .collect::<Vec<_>>()
            .join("\n");

        let content = Content::text(format!(
            "Knowledge Graph Status:\n\n\
            Total entities: {}\n\
            Total relationships: {}\n\n\
            Sample entities:\n{}",
            entity_count, relationship_count, sample_entities
        ));

        Ok(vec![content])
    }

    fn search_code_tool(&self, query: &str) -> Result<Vec<Content>, ToolError> {
        // Log some debug info first
        let entity_count = self.knowledge_graph.get_all_entities().len();
        let debug_info = format!(
            "Searching among {} entities for query: '{}'",
            entity_count, query
        );

        // Perform the search against the knowledge graph
        let results = self
            .knowledge_graph
            .search(query)
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to search code: {}", e)))?;

        // Format the results as text
        let content = if results.is_empty() {
            Content::text(format!("{}\n\nNo results found. Try a different search query?\n\nExample queries:\n- \"function\"\n- \"main\"\n- \"router\"", debug_info))
        } else {
            let mut sections = Vec::new();
            let mut functions = Vec::new();
            let mut classes = Vec::new();
            let mut modules = Vec::new();
            let mut others = Vec::new();

            // Categorize results by type
            for entity in results.iter() {
                let entity_info = format!(
                    "- {}: {} ({})",
                    entity.id(),
                    entity.name(),
                    entity.entity_type()
                );

                match entity.entity_type() {
                    EntityType::Function | EntityType::Method => functions.push(entity_info),
                    EntityType::Class | EntityType::Struct | EntityType::Type => {
                        classes.push(entity_info)
                    }
                    EntityType::Module | EntityType::File => modules.push(entity_info),
                    _ => others.push(entity_info),
                }
            }

            // Build formatted sections
            if !functions.is_empty() {
                sections.push(format!("Functions:\n{}", functions.join("\n")));
            }

            if !classes.is_empty() {
                sections.push(format!("Types:\n{}", classes.join("\n")));
            }

            if !modules.is_empty() {
                sections.push(format!("Modules:\n{}", modules.join("\n")));
            }

            if !others.is_empty() {
                sections.push(format!("Other entities:\n{}", others.join("\n")));
            }

            // Join everything together
            Content::text(format!(
                "{}\n\nFound {} results:\n\n{}",
                debug_info,
                results.len(),
                sections.join("\n\n")
            ))
        };

        Ok(vec![content])
    }

    fn get_entity_tool(&self, entity_id: &str) -> Result<Vec<Content>, ToolError> {
        // Get detailed information about a specific entity
        let entity_id_obj = crate::graph::entity::EntityId::new(entity_id);
        let entity = self
            .knowledge_graph
            .get_entity(&entity_id_obj)
            .ok_or_else(|| {
                ToolError::ExecutionFailed(format!("Entity not found: {}", entity_id))
            })?;

        // Get relationships for the entity
        let relationships = self
            .knowledge_graph
            .get_relationships_for_entity(entity_id)
            .map_err(|e| {
                ToolError::ExecutionFailed(format!("Failed to get relationships: {}", e))
            })?;

        // Format the entity details
        let entity_details = format!(
            "Entity: {}\nType: {}\nPath: {}\n",
            entity.name(),
            entity.entity_type(),
            entity.path().unwrap_or("N/A"),
        );

        // Format the relationships
        let relationships_details = if relationships.is_empty() {
            "No relationships found.".to_string()
        } else {
            let formatted = relationships
                .iter()
                .map(|rel| {
                    format!(
                        "- {} {} {}",
                        rel.source_id, rel.relationship_type, rel.target_id
                    )
                })
                .collect::<Vec<_>>()
                .join("\n");

            format!("Relationships:\n{}", formatted)
        };

        let content = Content::text(format!("{}\n{}", entity_details, relationships_details));
        Ok(vec![content])
    }

    fn find_relevant_files_tool(
        &self,
        description: &str,
        limit: usize,
    ) -> Result<Vec<Content>, ToolError> {
        // This tool helps AI agents find the most relevant files for a specific task or feature
        // First, search the knowledge graph for relevant entities based on the description
        let search_results = self
            .knowledge_graph
            .search(description)
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to search code: {}", e)))?;

        if search_results.is_empty() {
            return Ok(vec![Content::text(format!(
                "No relevant files found for task description: '{}'\n\nTry using a different description with more specific keywords related to the codebase.",
                description
            ))]);
        }

        // Group results by file path and count references to identify the most important files
        use std::collections::HashMap;
        let mut file_relevance: HashMap<String, (i32, Vec<String>)> = HashMap::new();

        for entity in search_results.iter() {
            if let Some(path) = entity.path() {
                // Identify the file path (just keeping the path itself)

                // Increase relevance score and add the entity that references this file
                let entry = file_relevance
                    .entry(path.to_string())
                    .or_insert((0, Vec::new()));
                entry.0 += 1;
                entry
                    .1
                    .push(format!("- {} ({})", entity.name(), entity.entity_type()));
            }
        }

        // Sort files by relevance score (descending)
        let mut sorted_files: Vec<(String, i32, Vec<String>)> = file_relevance
            .into_iter()
            .map(|(path, (score, entities))| (path, score, entities))
            .collect();
        sorted_files.sort_by(|a, b| b.1.cmp(&a.1));

        // Limit the number of results
        let sorted_files = sorted_files.into_iter().take(limit).collect::<Vec<_>>();

        // Format the results with explanations
        let mut formatted_results = format!(
            "Found {} relevant files for task: '{}'\n\n",
            sorted_files.len(),
            description
        );

        for (idx, (path, score, entities)) in sorted_files.iter().enumerate() {
            let file_name = path.rsplit('/').next().unwrap_or(path);

            formatted_results.push_str(&format!(
                "{}. {} (relevance score: {})\n   Path: {}\n   Contains:\n      {}\n\n",
                idx + 1,
                file_name,
                score,
                path,
                entities
                    .iter()
                    .take(3)
                    .cloned()
                    .collect::<Vec<_>>()
                    .join("\n      ")
            ));
        }

        formatted_results.push_str("\nRecommendation: Start by examining these files to understand the relevant components for your task.");

        Ok(vec![Content::text(formatted_results)])
    }

    fn explore_relationships_tool(
        &self,
        entity_id: &str,
        relationship_type_filter: Option<String>,
        depth: usize,
    ) -> Result<Vec<Content>, ToolError> {
        // Verify that the entity exists
        let entity_id_obj = crate::graph::entity::EntityId::new(entity_id);
        let entity = self
            .knowledge_graph
            .get_entity(&entity_id_obj)
            .ok_or_else(|| {
                ToolError::ExecutionFailed(format!("Entity not found: {}", entity_id))
            })?;

        // Get direct relationships for the entity
        let relationships = self
            .knowledge_graph
            .get_relationships_for_entity(entity_id)
            .map_err(|e| {
                ToolError::ExecutionFailed(format!("Failed to get relationships: {}", e))
            })?;

        if relationships.is_empty() {
            return Ok(vec![Content::text(format!(
                "Entity '{}' ({}) has no relationships.",
                entity.name(),
                entity.entity_type()
            ))]);
        }

        // Filter relationships by type if specified
        let filtered_relationships = if let Some(ref filter) = relationship_type_filter {
            relationships
                .into_iter()
                .filter(|rel| rel.relationship_type.to_string() == *filter)
                .collect::<Vec<_>>()
        } else {
            relationships
        };

        if filtered_relationships.is_empty() {
            return Ok(vec![Content::text(format!(
                "Entity '{}' ({}) has no relationships of type '{}'.",
                entity.name(),
                entity.entity_type(),
                relationship_type_filter.unwrap_or_default()
            ))]);
        }

        // Build a graph representation of related entities
        use std::collections::{HashMap, HashSet, VecDeque};

        let mut visited: HashSet<String> = HashSet::new();
        let mut relationship_map: HashMap<String, Vec<(String, String, String)>> = HashMap::new();
        let mut queue: VecDeque<(String, usize)> = VecDeque::new();

        // Start with the initial entity
        queue.push_back((entity_id.to_string(), 0));
        visited.insert(entity_id.to_string());

        // BFS to explore relationships up to the specified depth
        while let Some((current_id, current_depth)) = queue.pop_front() {
            // Stop if we've reached the maximum depth
            if current_depth >= depth {
                continue;
            }

            // Get relationships for the current entity
            let current_rels = self
                .knowledge_graph
                .get_relationships_for_entity(&current_id)
                .unwrap_or_default();

            // Filter by relationship type if needed
            let current_rels = if let Some(ref filter) = relationship_type_filter {
                current_rels
                    .into_iter()
                    .filter(|rel| rel.relationship_type.to_string() == *filter)
                    .collect::<Vec<_>>()
            } else {
                current_rels
            };

            // Process each relationship
            for rel in current_rels {
                // Compare using string representation
                let current_id_str = current_id.clone();
                let (other_id, _direction) = if rel.source_id.to_string() == current_id_str {
                    (rel.target_id.to_string(), "outgoing")
                } else {
                    (rel.source_id.to_string(), "incoming")
                };

                // Get the entity names for better readability
                let source_entity =
                    self.knowledge_graph
                        .get_entity(&crate::graph::entity::EntityId::new(
                            &rel.source_id.to_string(),
                        ));
                let target_entity =
                    self.knowledge_graph
                        .get_entity(&crate::graph::entity::EntityId::new(
                            &rel.target_id.to_string(),
                        ));

                let source_id_str = rel.source_id.to_string();
                let target_id_str = rel.target_id.to_string();
                let source_name = source_entity
                    .map(|e| e.name())
                    .unwrap_or_else(|| &source_id_str);
                let target_name = target_entity
                    .map(|e| e.name())
                    .unwrap_or_else(|| &target_id_str);

                // Store the relationship
                let entry = relationship_map
                    .entry(current_id.clone())
                    .or_insert_with(Vec::new);
                entry.push((
                    rel.relationship_type.to_string(),
                    other_id.clone(),
                    format!("{} → {}", source_name, target_name),
                ));

                // Enqueue the other entity if not visited
                if !visited.contains(&other_id) {
                    visited.insert(other_id.clone());
                    queue.push_back((other_id.clone(), current_depth + 1));
                }
            }
        }

        // Format the results as a dependency graph
        let mut result = format!(
            "Relationship exploration for entity '{}' ({}):\n\n",
            entity.name(),
            entity.entity_type()
        );

        // Helper function to format relationships
        fn format_relationships(
            id: &str,
            rel_map: &HashMap<String, Vec<(String, String, String)>>,
            depth: usize,
            entity_names: &HashMap<String, String>,
            visited: &mut HashSet<String>,
        ) -> String {
            if visited.contains(id) || depth == 0 {
                return String::new();
            }

            visited.insert(id.to_string());

            let indent = "  ".repeat(depth);
            let mut result = String::new();

            if let Some(rels) = rel_map.get(id) {
                for (rel_type, other_id, names) in rels {
                    result.push_str(&format!(
                        "{}→ {} [{}] {}\n",
                        indent, rel_type, other_id, names
                    ));

                    // Recursively add related entities
                    result.push_str(&format_relationships(
                        other_id,
                        rel_map,
                        depth + 1,
                        entity_names,
                        visited,
                    ));
                }
            }

            result
        }

        // Build a map of entity IDs to names for better formatting
        let mut entity_names: HashMap<String, String> = HashMap::new();
        for id in visited.iter() {
            if let Some(e) = self
                .knowledge_graph
                .get_entity(&crate::graph::entity::EntityId::new(id))
            {
                entity_names.insert(id.clone(), e.name().to_string());
            }
        }

        // Format the relationship graph
        let mut visited_during_formatting = HashSet::new();
        result.push_str(&format!("● {} ({})\n", entity.name(), entity_id));
        result.push_str(&format_relationships(
            entity_id,
            &relationship_map,
            1,
            &entity_names,
            &mut visited_during_formatting,
        ));

        // Add explanation and summary
        let relationship_count = filtered_relationships.len();
        let relationship_types = filtered_relationships
            .iter()
            .map(|r| r.relationship_type.to_string())
            .collect::<HashSet<_>>();

        result.push_str(&format!(
            "\nSummary: Entity '{}' has {} direct relationship(s) of {} type(s): {}.\n",
            entity.name(),
            relationship_count,
            relationship_types.len(),
            relationship_types
                .into_iter()
                .collect::<Vec<_>>()
                .join(", ")
        ));

        if depth > 1 {
            result.push_str(&format!(
                "Graph shows relationships up to depth {}.\n",
                depth
            ));
        }

        // Add recommendations
        result.push_str("\nRecommendations for understanding these relationships:\n");
        result.push_str("1. Focus on direct dependencies first\n");
        result.push_str("2. Look for clusters of related entities\n");
        result.push_str("3. Use get_entity tool to examine specific entities in detail\n");

        Ok(vec![Content::text(result)])
    }

    fn explain_architecture_tool(
        &self,
        focus_area: Option<String>,
        detail_level: &str,
    ) -> Result<Vec<Content>, ToolError> {
        // This tool helps AI agents understand the architecture of the codebase

        // Analyze the knowledge graph to identify core components
        let all_entities = self.knowledge_graph.get_all_entities();

        // Count entity types to understand the composition of the codebase
        use std::collections::HashMap;
        let mut entity_type_counts: HashMap<String, usize> = HashMap::new();
        let mut module_entities = Vec::new();

        for entity in all_entities.iter() {
            // Count entity types
            let type_name = entity.entity_type().to_string();
            *entity_type_counts.entry(type_name).or_insert(0) += 1;

            // Collect module and file entities for structure analysis
            if matches!(entity.entity_type(), EntityType::Module | EntityType::File) {
                // Use to_owned to get owned entity instead of clone on reference
                if let Some(owned_entity) = self.knowledge_graph.get_entity(&entity.id()) {
                    module_entities.push(owned_entity);
                }
            }
        }

        // Get relationships to understand component connections
        let all_relationships = self.knowledge_graph.get_all_relationships().map_err(|e| {
            ToolError::ExecutionFailed(format!("Failed to get relationships: {}", e))
        })?;

        // Count relationship types
        let mut relationship_type_counts: HashMap<String, usize> = HashMap::new();
        for rel in all_relationships.iter() {
            let type_name = rel.relationship_type.to_string();
            *relationship_type_counts.entry(type_name).or_insert(0) += 1;
        }

        // Build directory/module structure
        let mut module_structure: HashMap<String, Vec<String>> = HashMap::new();
        for entity in module_entities.iter() {
            if let Some(path) = entity.path() {
                // Extract directory components
                let parts: Vec<&str> = path.split('/').collect();

                if parts.len() > 1 {
                    let parent = parts[parts.len() - 2].to_string();
                    let child = parts[parts.len() - 1].to_string();

                    module_structure
                        .entry(parent)
                        .or_insert_with(Vec::new)
                        .push(child);
                }
            }
        }

        // If focus area is specified, filter the analysis
        let focused_results = if let Some(ref focus) = focus_area {
            // Search for entities related to the focus area
            let search_results = self
                .knowledge_graph
                .search(focus)
                .map_err(|e| ToolError::ExecutionFailed(format!("Failed to search code: {}", e)))?;

            if search_results.is_empty() {
                return Ok(vec![Content::text(format!(
                    "No components found for focus area: '{}'\n\nTry using a different focus area with more specific keywords related to the codebase.",
                    focus
                ))]);
            }

            // Collect key entities in the focus area
            let mut key_components = Vec::new();
            for entity in search_results.iter().take(10) {
                key_components.push(format!(
                    "- {} ({}): {}",
                    entity.name(),
                    entity.entity_type(),
                    entity.path().unwrap_or("N/A")
                ));
            }

            Some((search_results.len(), key_components))
        } else {
            None
        };

        // Generate architecture description based on detail level
        let architecture_overview = match detail_level {
            "high" => self.generate_detailed_architecture(
                &entity_type_counts,
                &relationship_type_counts,
                &module_structure,
                focus_area.as_deref(),
                &focused_results,
            ),
            "low" => self.generate_simple_architecture(
                &entity_type_counts,
                &relationship_type_counts,
                &module_structure,
                focus_area.as_deref(),
                &focused_results,
            ),
            _ => self.generate_medium_architecture(
                &entity_type_counts,
                &relationship_type_counts,
                &module_structure,
                focus_area.as_deref(),
                &focused_results,
            ),
        };

        Ok(vec![Content::text(architecture_overview)])
    }

    // Helper methods for architecture explanation

    fn generate_simple_architecture(
        &self,
        _entity_counts: &HashMap<String, usize>,
        _relationship_counts: &HashMap<String, usize>,
        module_structure: &HashMap<String, Vec<String>>,
        focus_area: Option<&str>,
        focused_results: &Option<(usize, Vec<String>)>,
    ) -> String {
        let mut result = if let Some(focus) = focus_area {
            format!("# Architecture Overview: {}\n\n", focus)
        } else {
            "# Architecture Overview\n\n".to_string()
        };

        // Add focus area summary if applicable
        if let Some((count, components)) = focused_results {
            result.push_str(&format!(
                "Found {} components related to '{}'.\n\n",
                count,
                focus_area.unwrap_or("the specified focus area")
            ));

            result.push_str("## Key Components\n\n");
            for component in components.iter().take(5) {
                result.push_str(&format!("{}\n", component));
            }
            result.push('\n');
        }

        // Simplified module structure
        result.push_str("## Main Modules\n\n");

        // Find the top-level modules (assuming src is a top-level directory)
        if let Some(src_modules) = module_structure.get("src") {
            for module in src_modules.iter() {
                result.push_str(&format!("- {}\n", module));
            }
        } else {
            // If no src folder, list a few key directories
            for (dir, _) in module_structure.iter().take(5) {
                result.push_str(&format!("- {}\n", dir));
            }
        }

        result.push_str("\n## Next Steps\n\n");
        result.push_str("1. Use `search_code` to find specific components\n");
        result.push_str("2. Examine key files with `find_relevant_files`\n");
        result.push_str("3. Look at relationships with `explore_relationships`\n");

        result
    }

    fn generate_medium_architecture(
        &self,
        entity_counts: &HashMap<String, usize>,
        relationship_counts: &HashMap<String, usize>,
        module_structure: &HashMap<String, Vec<String>>,
        focus_area: Option<&str>,
        focused_results: &Option<(usize, Vec<String>)>,
    ) -> String {
        let mut result = if let Some(focus) = focus_area {
            format!("# Architecture Overview: {}\n\n", focus)
        } else {
            "# Architecture Overview\n\n".to_string()
        };

        // Add focus area summary if applicable
        if let Some((count, components)) = focused_results {
            result.push_str(&format!(
                "Found {} components related to '{}'.\n\n",
                count,
                focus_area.unwrap_or("the specified focus area")
            ));

            result.push_str("## Key Components\n\n");
            for component in components.iter() {
                result.push_str(&format!("{}\n", component));
            }
            result.push('\n');
        }

        // Codebase statistics
        result.push_str("## Codebase Statistics\n\n");

        let total_entities: usize = entity_counts.values().sum();
        result.push_str(&format!("- Total components: {}\n", total_entities));

        for (entity_type, count) in entity_counts
            .iter()
            .filter(|(_, count)| **count > total_entities / 20) // Only show significant entity types
            .collect::<Vec<_>>()
        {
            result.push_str(&format!("- {}: {}\n", entity_type, count));
        }

        // Module structure
        result.push_str("\n## Module Structure\n\n");

        // Find the src modules (typical for most projects)
        if let Some(src_modules) = module_structure.get("src") {
            for module in src_modules.iter() {
                result.push_str(&format!("- {}/\n", module));

                // Show contents of this module if available
                if let Some(submodules) = module_structure.get(module) {
                    for submodule in submodules.iter().take(5) {
                        result.push_str(&format!("  - {}\n", submodule));
                    }

                    // Indicate if there are more submodules
                    if submodules.len() > 5 {
                        result.push_str(&format!("  - ... and {} more\n", submodules.len() - 5));
                    }
                }
            }
        } else {
            // If no src folder, list other key directories
            for (dir, submodules) in module_structure.iter().take(8) {
                result.push_str(&format!("- {}/\n", dir));

                if !submodules.is_empty() {
                    for submodule in submodules.iter().take(3) {
                        result.push_str(&format!("  - {}\n", submodule));
                    }

                    if submodules.len() > 3 {
                        result.push_str(&format!("  - ... and {} more\n", submodules.len() - 3));
                    }
                }
            }
        }

        // Key relationships
        result.push_str("\n## Key Relationships\n\n");

        for (rel_type, count) in relationship_counts.iter().take(5) {
            result.push_str(&format!("- {}: {} instances\n", rel_type, count));
        }

        // Recommendations
        result.push_str("\n## Working With This Codebase\n\n");
        result.push_str("1. Start by exploring key modules and their relationships\n");
        result.push_str("2. Use `search_code` to find relevant components\n");
        result.push_str("3. Examine dependencies with `explore_relationships`\n");
        result.push_str(
            "4. For specific tasks, use `find_relevant_files` to identify starting points\n",
        );

        result
    }

    fn generate_detailed_architecture(
        &self,
        entity_counts: &HashMap<String, usize>,
        relationship_counts: &HashMap<String, usize>,
        module_structure: &HashMap<String, Vec<String>>,
        focus_area: Option<&str>,
        focused_results: &Option<(usize, Vec<String>)>,
    ) -> String {
        let mut result = if let Some(focus) = focus_area {
            format!("# Detailed Architecture: {}\n\n", focus)
        } else {
            "# Detailed Architecture Analysis\n\n".to_string()
        };

        // Add focus area analysis if applicable
        if let Some((count, components)) = focused_results {
            result.push_str(&format!(
                "## Focus Area: {}\n\n",
                focus_area.unwrap_or("Specified Focus")
            ));

            result.push_str(&format!(
                "Found {} components related to this area.\n\n",
                count
            ));

            result.push_str("### Key Components\n\n");
            for component in components.iter() {
                result.push_str(&format!("{}\n", component));
            }
            result.push('\n');

            // If there's a focus area, try to find relationships between these components
            if let Some(focus) = focus_area {
                result.push_str("### Component Relationships\n\n");

                // Get entities that match the focus area
                let search_results = match self.knowledge_graph.search(focus) {
                    Ok(results) => results,
                    Err(_) => Vec::new(),
                };

                // Analyze relationships between these entities
                if !search_results.is_empty() {
                    let mut relationship_pairs = HashMap::new();

                    // Collect relationships between focus area entities
                    for entity in search_results.iter().take(10) {
                        if let Ok(rels) = self
                            .knowledge_graph
                            .get_relationships_for_entity(&entity.id().to_string())
                        {
                            for rel in rels {
                                let rel_string = format!(
                                    "{} {} {}",
                                    rel.source_id, rel.relationship_type, rel.target_id
                                );
                                relationship_pairs.insert(rel_string, rel);
                            }
                        }
                    }

                    // Output key relationships
                    if !relationship_pairs.is_empty() {
                        let key_rels = relationship_pairs.values().take(10).collect::<Vec<_>>();

                        for rel in key_rels {
                            // Get readable names
                            let source_entity = self.knowledge_graph.get_entity(
                                &crate::graph::entity::EntityId::new(&rel.source_id.to_string()),
                            );
                            let target_entity = self.knowledge_graph.get_entity(
                                &crate::graph::entity::EntityId::new(&rel.target_id.to_string()),
                            );

                            let source_name = source_entity
                                .map(|e| format!("{} ({})", e.name(), e.entity_type()))
                                .unwrap_or_else(|| format!("{}", rel.source_id));
                            let target_name = target_entity
                                .map(|e| format!("{} ({})", e.name(), e.entity_type()))
                                .unwrap_or_else(|| format!("{}", rel.target_id));

                            result.push_str(&format!(
                                "- {} {} {}\n",
                                source_name, rel.relationship_type, target_name
                            ));
                        }
                    } else {
                        result.push_str("No direct relationships found between components in this focus area.\n");
                    }
                }
            }

            result.push('\n');
        }

        // Codebase statistics
        result.push_str("## Codebase Composition\n\n");

        let total_entities: usize = entity_counts.values().sum();
        result.push_str(&format!("Total components: {}\n\n", total_entities));

        result.push_str("### Component Types\n\n");
        for (entity_type, count) in entity_counts.iter() {
            let percentage = (*count as f64 / total_entities as f64) * 100.0;
            result.push_str(&format!(
                "- {}: {} ({:.1}%)\n",
                entity_type, count, percentage
            ));
        }

        // Module structure
        result.push_str("\n## Module Structure\n\n");

        // Process module structure to create a hierarchical view
        let mut module_tree = HashMap::new();

        // Helper function to add a path to the tree
        fn add_to_tree(
            tree: &mut HashMap<String, HashMap<String, HashMap<String, Vec<String>>>>,
            path: &str,
        ) {
            let parts: Vec<&str> = path.split('/').collect();
            if parts.len() >= 4 {
                // At least 4 levels deep (e.g., project/src/module/file)
                let l1 = parts[0].to_string();
                let l2 = parts[1].to_string();
                let l3 = parts[2].to_string();
                let l4 = parts[3].to_string();

                let level1 = tree.entry(l1).or_insert_with(HashMap::new);
                let level2 = level1.entry(l2).or_insert_with(HashMap::new);
                let level3 = level2.entry(l3).or_insert_with(Vec::new);

                if !level3.contains(&l4) {
                    level3.push(l4);
                }
            }
        }

        // Collect file paths
        for entity in self.knowledge_graph.get_all_entities() {
            if let Some(path) = entity.path() {
                add_to_tree(&mut module_tree, path);
            }
        }

        // Display module tree (up to 3 levels)
        for (l1_name, l1) in module_tree.iter() {
            result.push_str(&format!("- {}/\n", l1_name));

            for (l2_name, l2) in l1.iter() {
                result.push_str(&format!("  - {}/\n", l2_name));

                for (l3_name, l3_files) in l2.iter() {
                    result.push_str(&format!("    - {}/\n", l3_name));

                    // Show up to 5 files per module
                    for (_idx, file) in l3_files.iter().take(5).enumerate() {
                        result.push_str(&format!("      - {}\n", file));
                    }

                    // Indicate if there are more files
                    if l3_files.len() > 5 {
                        result.push_str(&format!(
                            "      - ... and {} more files\n",
                            l3_files.len() - 5
                        ));
                    }
                }
            }
        }

        // Key relationships
        result.push_str("\n## Relationships Analysis\n\n");

        let total_relationships: usize = relationship_counts.values().sum();
        result.push_str(&format!("Total relationships: {}\n\n", total_relationships));

        result.push_str("### Relationship Types\n\n");
        for (rel_type, count) in relationship_counts.iter() {
            let percentage = (*count as f64 / total_relationships as f64) * 100.0;
            result.push_str(&format!("- {}: {} ({:.1}%)\n", rel_type, count, percentage));
        }

        // Find centrality - which entities have the most relationships
        if !focus_area.is_some() {
            // Only do this for whole-codebase analysis
            result.push_str("\n### Central Components\n\n");

            // Count relationships per entity
            let mut entity_relationship_counts = HashMap::new();

            if let Ok(all_rels) = self.knowledge_graph.get_all_relationships() {
                for rel in all_rels {
                    *entity_relationship_counts
                        .entry(rel.source_id.clone())
                        .or_insert(0) += 1;
                    *entity_relationship_counts
                        .entry(rel.target_id.clone())
                        .or_insert(0) += 1;
                }

                // Sort by relationship count
                let mut entity_counts = entity_relationship_counts.into_iter().collect::<Vec<_>>();
                entity_counts.sort_by(|a, b| b.1.cmp(&a.1));

                // Display top 10 central entities
                for (entity_id, count) in entity_counts.iter().take(10) {
                    let entity = self
                        .knowledge_graph
                        .get_entity(&crate::graph::entity::EntityId::new(&entity_id.to_string()));

                    if let Some(e) = entity {
                        result.push_str(&format!(
                            "- {} ({}) with {} relationships: {}\n",
                            e.name(),
                            e.entity_type(),
                            count,
                            entity_id
                        ));
                    }
                }
            }
        }

        // Architectural patterns
        result.push_str("\n## Architectural Patterns\n\n");

        // Look for layered architecture
        let mut layers = Vec::new();
        if module_structure.contains_key("src") {
            if let Some(src_modules) = module_structure.get("src") {
                // Common layer names
                for layer in [
                    "api",
                    "controllers",
                    "models",
                    "services",
                    "repositories",
                    "utils",
                    "core",
                    "data",
                    "ui",
                    "views",
                ]
                .iter()
                {
                    if src_modules.contains(&layer.to_string()) {
                        layers.push(*layer);
                    }
                }
            }
        }

        if !layers.is_empty() {
            result.push_str("### Layered Architecture\n\n");
            result.push_str("This codebase appears to follow a layered architecture pattern with these layers:\n\n");

            for layer in layers {
                result.push_str(&format!("- {}\n", layer));
            }
        }

        // Working with this architecture
        result.push_str("\n## Working With This Codebase\n\n");
        result.push_str("### Key Entry Points\n\n");

        // Find potential entry points (main functions, exported functions, etc.)
        let entry_points = self
            .knowledge_graph
            .search("main entry")
            .unwrap_or_default();
        if !entry_points.is_empty() {
            for entity in entry_points.iter().take(5) {
                result.push_str(&format!(
                    "- {} ({}): {}\n",
                    entity.name(),
                    entity.entity_type(),
                    entity.path().unwrap_or("N/A")
                ));
            }
        } else {
            result.push_str("Could not determine specific entry points. Consider searching for 'main' or 'init' functions.\n");
        }

        // Recommendations
        result.push_str("\n### Recommendations for Working with this Codebase\n\n");
        result.push_str("1. Start with the key entry points identified above\n");
        result.push_str("2. Follow the relationships to understand dependencies\n");
        result.push_str("3. Use `find_relevant_files` to identify components for specific tasks\n");
        result.push_str("4. For understanding complex dependencies, use `explore_relationships` with depth parameter\n");
        result.push_str("5. Refer to central components as they provide core functionality\n");

        result
    }
}

impl Router for UmmonRouter {
    fn name(&self) -> String {
        "ummon-router".to_string()
    }

    fn instructions(&self) -> String {
        r#"
        This MCP server provides tools to query and analyze the Ummon code knowledge graph.
        
        Available tools:
        
        Basic Knowledge Graph Tools:
        - search_code: Search for code entities using a natural language query
        - get_entity: Get detailed information about a specific entity
        - debug_graph: Get information about the loaded knowledge graph
        
        Enhanced Agent Tools:
        - find_relevant_files: Find the most relevant files for a specific task or feature
        - explore_relationships: Explore and explain relationships between entities
        - explain_architecture: Provide an architectural overview of the codebase or a specific area
        
        Recommended workflow:
        1. Start with explain_architecture to understand the overall structure
        2. Use find_relevant_files to identify components related to your task
        3. Explore specific entities with get_entity and explore_relationships
        "#
        .to_string()
    }

    fn capabilities(&self) -> ServerCapabilities {
        CapabilitiesBuilder::new()
            .with_tools(true)
            .with_resources(true, false)
            .build()
    }

    fn list_tools(&self) -> Vec<Tool> {
        vec![
            Tool::new(
                "search_code".to_string(),
                "Search for code entities using a natural language query".to_string(),
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        "query": {
                            "type": "string",
                            "description": "Natural language query to search the code knowledge graph"
                        }
                    },
                    "required": ["query"]
                }),
            ),
            Tool::new(
                "get_entity".to_string(),
                "Get detailed information about a specific entity".to_string(),
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        "entity_id": {
                            "type": "string",
                            "description": "ID of the entity to retrieve information for"
                        }
                    },
                    "required": ["entity_id"]
                }),
            ),
            Tool::new(
                "debug_graph".to_string(),
                "Get information about the loaded knowledge graph".to_string(),
                serde_json::json!({
                    "type": "object",
                    "properties": {},
                    "required": []
                }),
            ),
            Tool::new(
                "find_relevant_files".to_string(),
                "Find the most relevant files for a specific task or feature".to_string(),
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        "description": {
                            "type": "string",
                            "description": "Description of the task or feature to find relevant files for"
                        },
                        "limit": {
                            "type": "integer",
                            "description": "Maximum number of files to return (default: 5)"
                        }
                    },
                    "required": ["description"]
                }),
            ),
            Tool::new(
                "explore_relationships".to_string(),
                "Explore and explain relationships between entities".to_string(),
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        "entity_id": {
                            "type": "string",
                            "description": "ID of the entity to explore relationships for"
                        },
                        "relationship_type": {
                            "type": "string",
                            "description": "Optional filter for relationship types (e.g., 'Calls', 'Contains', etc.)"
                        },
                        "depth": {
                            "type": "integer",
                            "description": "Maximum relationship depth to explore (default: 1)"
                        }
                    },
                    "required": ["entity_id"]
                }),
            ),
            Tool::new(
                "explain_architecture".to_string(),
                "Provide an architectural overview of the codebase or a specific area".to_string(),
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        "focus_area": {
                            "type": "string",
                            "description": "Optional focus area to limit the architecture explanation (e.g., 'parsing', 'graph')"
                        },
                        "detail_level": {
                            "type": "string",
                            "enum": ["high", "medium", "low"],
                            "description": "Level of detail in the explanation (default: 'medium')"
                        }
                    },
                    "required": []
                }),
            ),
        ]
    }

    fn call_tool(
        &self,
        tool_name: &str,
        arguments: Value,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<Content>, ToolError>> + Send + 'static>> {
        let name = tool_name.to_string();
        let router = self.clone();

        Box::pin(async move {
            match name.as_str() {
                "search_code" => {
                    let query =
                        arguments
                            .get("query")
                            .and_then(|v| v.as_str())
                            .ok_or_else(|| {
                                ToolError::InvalidParams("Missing 'query' parameter".to_string())
                            })?;

                    router.search_code_tool(query)
                }
                "get_entity" => {
                    let entity_id = arguments
                        .get("entity_id")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| {
                            ToolError::InvalidParams("Missing 'entity_id' parameter".to_string())
                        })?;

                    router.get_entity_tool(entity_id)
                }
                "debug_graph" => router.debug_graph_tool(),
                "find_relevant_files" => {
                    let description = arguments
                        .get("description")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| {
                            ToolError::InvalidParams("Missing 'description' parameter".to_string())
                        })?;

                    let limit =
                        arguments.get("limit").and_then(|v| v.as_u64()).unwrap_or(5) as usize;

                    router.find_relevant_files_tool(description, limit)
                }
                "explore_relationships" => {
                    let entity_id = arguments
                        .get("entity_id")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| {
                            ToolError::InvalidParams("Missing 'entity_id' parameter".to_string())
                        })?;

                    let relationship_type = arguments
                        .get("relationship_type")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());

                    let depth =
                        arguments.get("depth").and_then(|v| v.as_u64()).unwrap_or(1) as usize;

                    router.explore_relationships_tool(entity_id, relationship_type, depth)
                }
                "explain_architecture" => {
                    let focus_area = arguments
                        .get("focus_area")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());

                    let detail_level = arguments
                        .get("detail_level")
                        .and_then(|v| v.as_str())
                        .unwrap_or("medium");

                    router.explain_architecture_tool(focus_area, detail_level)
                }
                _ => Err(ToolError::NotFound(name)),
            }
        })
    }

    fn list_resources(&self) -> Vec<Resource> {
        vec![Resource::new(
            "knowledge_graph.json".to_string(),
            "Knowledge Graph".to_string(),
            "The full knowledge graph in JSON format".to_string(),
            Some(false),
        )]
    }

    fn read_resource(
        &self,
        uri: &str,
    ) -> Pin<Box<dyn Future<Output = Result<String, ResourceError>> + Send + 'static>> {
        let uri = uri.to_string();
        let router = self.clone();

        Box::pin(async move {
            match uri.as_str() {
                "knowledge_graph.json" => {
                    let json =
                        serde_json::to_string_pretty(&*router.knowledge_graph).map_err(|e| {
                            ResourceError::Internal(format!(
                                "Failed to serialize knowledge graph: {}",
                                e
                            ))
                        })?;

                    Ok(json)
                }
                _ => Err(ResourceError::NotFound(uri)),
            }
        })
    }
}

impl Clone for UmmonRouter {
    fn clone(&self) -> Self {
        Self {
            knowledge_graph: Arc::clone(&self.knowledge_graph),
        }
    }
}
