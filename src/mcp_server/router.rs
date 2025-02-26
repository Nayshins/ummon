use std::sync::Arc;
use serde_json::Value;
use std::future::Future;
use std::pin::Pin;

use crate::mcp_core::{Router, Tool, Resource, Content, ToolError, ResourceError, ServerCapabilities, CapabilitiesBuilder};
use crate::graph::knowledge_graph::KnowledgeGraph;
use crate::graph::entity::EntityType;

/// UmmonRouter implements the Router trait and handles Ummon-specific functionality
pub struct UmmonRouter {
    knowledge_graph: Arc<KnowledgeGraph>,
}

impl UmmonRouter {
    pub fn new(knowledge_graph: Arc<KnowledgeGraph>) -> Self {
        Self {
            knowledge_graph,
        }
    }

    fn debug_graph_tool(&self) -> Result<Vec<Content>, ToolError> {
        // Get entity count
        let entity_count = self.knowledge_graph.get_all_entities().len();
        let relationship_count = self.knowledge_graph.get_relationship_count();
        
        // Get a sample of 5 entities to verify content
        let sample_entities = self.knowledge_graph.get_all_entities()
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
            entity_count,
            relationship_count,
            sample_entities
        ));
        
        Ok(vec![content])
    }

    fn search_code_tool(&self, query: &str) -> Result<Vec<Content>, ToolError> {
        // Log some debug info first
        let entity_count = self.knowledge_graph.get_all_entities().len();
        let debug_info = format!("Searching among {} entities for query: '{}'", entity_count, query);
        
        // Perform the search against the knowledge graph
        let results = self.knowledge_graph.search(query)
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
                let entity_info = format!("- {}: {} ({})", entity.id(), entity.name(), entity.entity_type());
                
                match entity.entity_type() {
                    EntityType::Function | EntityType::Method => functions.push(entity_info),
                    EntityType::Class | EntityType::Struct | EntityType::Type => classes.push(entity_info),
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
            Content::text(format!("{}\n\nFound {} results:\n\n{}", 
                debug_info, 
                results.len(),
                sections.join("\n\n")))
        };

        Ok(vec![content])
    }

    fn get_entity_tool(&self, entity_id: &str) -> Result<Vec<Content>, ToolError> {
        // Get detailed information about a specific entity
        let entity_id_obj = crate::graph::entity::EntityId::new(entity_id);
        let entity = self.knowledge_graph.get_entity(&entity_id_obj)
            .ok_or_else(|| ToolError::ExecutionFailed(format!("Entity not found: {}", entity_id)))?;

        // Get relationships for the entity
        let relationships = self.knowledge_graph.get_relationships_for_entity(entity_id)
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to get relationships: {}", e)))?;

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
            let formatted = relationships.iter()
                .map(|rel| format!(
                    "- {} {} {}",
                    rel.source_id,
                    rel.relationship_type,
                    rel.target_id
                ))
                .collect::<Vec<_>>()
                .join("\n");
            
            format!("Relationships:\n{}", formatted)
        };

        let content = Content::text(format!("{}\n{}", entity_details, relationships_details));
        Ok(vec![content])
    }
}

impl Router for UmmonRouter {
    fn name(&self) -> String {
        "ummon-router".to_string()
    }

    fn instructions(&self) -> String {
        r#"
        This MCP server provides tools to query the Ummon code knowledge graph.
        Available tools:
        - search_code: Search for code entities using a natural language query
        - get_entity: Get detailed information about a specific entity
        - debug_graph: Get information about the loaded knowledge graph
        "#.to_string()
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
                    let query = arguments.get("query")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| ToolError::InvalidParams("Missing 'query' parameter".to_string()))?;
                    
                    router.search_code_tool(query)
                },
                "get_entity" => {
                    let entity_id = arguments.get("entity_id")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| ToolError::InvalidParams("Missing 'entity_id' parameter".to_string()))?;
                    
                    router.get_entity_tool(entity_id)
                },
                "debug_graph" => {
                    router.debug_graph_tool()
                },
                _ => Err(ToolError::NotFound(name)),
            }
        })
    }

    fn list_resources(&self) -> Vec<Resource> {
        vec![
            Resource::new(
                "knowledge_graph.json".to_string(),
                "Knowledge Graph".to_string(),
                "The full knowledge graph in JSON format".to_string(),
                Some(false),
            ),
        ]
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
                    let json = serde_json::to_string_pretty(&*router.knowledge_graph)
                        .map_err(|e| ResourceError::Internal(format!("Failed to serialize knowledge graph: {}", e)))?;
                    
                    Ok(json)
                },
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