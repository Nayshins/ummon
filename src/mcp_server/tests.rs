#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use serde_json::json;

    use crate::graph::entity::{BaseEntity, EntityId, EntityType};
    use crate::graph::knowledge_graph::KnowledgeGraph;
    use crate::graph::relationship::{Relationship, RelationshipType};
    use crate::mcp_core::{Content, ToolError};
    use crate::mcp_server::router::UmmonRouter;

    // Helper function to create a test knowledge graph with some entities and relationships
    fn create_test_knowledge_graph() -> Arc<KnowledgeGraph> {
        let mut kg = KnowledgeGraph::new();
        
        // Add some entities
        let entity1 = BaseEntity::new(
            EntityId::new("entity_1"),
            "TestFunction",
            EntityType::Function,
            Some("src/test.rs".to_string()),
            Some("A test function"),
        );
        
        let entity2 = BaseEntity::new(
            EntityId::new("entity_2"),
            "TestClass",
            EntityType::Class,
            Some("src/test.rs".to_string()),
            Some("A test class"),
        );
        
        let entity3 = BaseEntity::new(
            EntityId::new("entity_3"),
            "TestModule",
            EntityType::Module,
            Some("src/test/mod.rs".to_string()),
            Some("A test module"),
        );
        
        kg.add_entity(Box::new(entity1));
        kg.add_entity(Box::new(entity2));
        kg.add_entity(Box::new(entity3));
        
        // Add some relationships
        let relationship1 = Relationship {
            source_id: EntityId::new("entity_1"),
            target_id: EntityId::new("entity_2"),
            relationship_type: RelationshipType::Calls,
        };
        
        let relationship2 = Relationship {
            source_id: EntityId::new("entity_3"),
            target_id: EntityId::new("entity_1"),
            relationship_type: RelationshipType::Contains,
        };
        
        kg.add_relationship(relationship1.clone());
        kg.add_relationship(relationship2.clone());
        
        Arc::new(kg)
    }

    #[tokio::test]
    async fn test_search_code_tool() {
        let kg = create_test_knowledge_graph();
        let router = UmmonRouter::new(kg);
        
        // Test search for "test"
        let args = json!({
            "query": "test"
        });
        
        let result = router.invoke_tool("search_code", &args).await;
        assert!(result.is_ok(), "Search tool should return Ok result");
        
        let content = result.unwrap();
        assert!(!content.is_empty(), "Search result should not be empty");
        
        if let Content::Text(text) = &content[0] {
            assert!(text.contains("Found"), "Search result should contain 'Found'");
            assert!(text.contains("TestFunction"), "Search result should contain 'TestFunction'");
            assert!(text.contains("TestClass"), "Search result should contain 'TestClass'");
        } else {
            panic!("Search result should be text content");
        }
    }
    
    #[tokio::test]
    async fn test_get_entity_tool() {
        let kg = create_test_knowledge_graph();
        let router = UmmonRouter::new(kg);
        
        // Test get entity
        let args = json!({
            "entity_id": "entity_1"
        });
        
        let result = router.invoke_tool("get_entity", &args).await;
        assert!(result.is_ok(), "Get entity tool should return Ok result");
        
        let content = result.unwrap();
        assert!(!content.is_empty(), "Get entity result should not be empty");
        
        if let Content::Text(text) = &content[0] {
            assert!(text.contains("TestFunction"), "Entity result should contain entity name");
            assert!(text.contains("entity_1"), "Entity result should contain entity ID");
            assert!(text.contains("Function"), "Entity result should contain entity type");
        } else {
            panic!("Entity result should be text content");
        }
    }
    
    #[tokio::test]
    async fn test_debug_graph_tool() {
        let kg = create_test_knowledge_graph();
        let router = UmmonRouter::new(kg);
        
        let result = router.invoke_tool("debug_graph", &json!({})).await;
        assert!(result.is_ok(), "Debug graph tool should return Ok result");
        
        let content = result.unwrap();
        assert!(!content.is_empty(), "Debug graph result should not be empty");
        
        if let Content::Text(text) = &content[0] {
            assert!(text.contains("Total entities: 3"), "Debug result should show correct entity count");
            assert!(text.contains("Total relationships: 2"), "Debug result should show correct relationship count");
        } else {
            panic!("Debug result should be text content");
        }
    }
    
    #[tokio::test]
    async fn test_find_relevant_files_tool() {
        let kg = create_test_knowledge_graph();
        let router = UmmonRouter::new(kg);
        
        // Test find relevant files
        let args = json!({
            "description": "test function",
            "limit": 2
        });
        
        let result = router.invoke_tool("find_relevant_files", &args).await;
        assert!(result.is_ok(), "Find relevant files tool should return Ok result");
        
        let content = result.unwrap();
        assert!(!content.is_empty(), "Find relevant files result should not be empty");
        
        if let Content::Text(text) = &content[0] {
            assert!(text.contains("Found"), "Result should contain 'Found'");
            assert!(text.contains("src/test.rs"), "Result should contain 'src/test.rs'");
        } else {
            panic!("Result should be text content");
        }
    }
    
    #[tokio::test]
    async fn test_explore_relationships_tool() {
        let kg = create_test_knowledge_graph();
        let router = UmmonRouter::new(kg);
        
        // Test explore relationships
        let args = json!({
            "entity_id": "entity_1",
            "depth": 1
        });
        
        let result = router.invoke_tool("explore_relationships", &args).await;
        assert!(result.is_ok(), "Explore relationships tool should return Ok result");
        
        let content = result.unwrap();
        assert!(!content.is_empty(), "Explore relationships result should not be empty");
        
        if let Content::Text(text) = &content[0] {
            assert!(text.contains("TestFunction"), "Result should contain entity name");
            assert!(text.contains("Calls"), "Result should contain relationship type");
            assert!(text.contains("TestClass"), "Result should contain related entity");
        } else {
            panic!("Result should be text content");
        }
    }
    
    #[tokio::test]
    async fn test_explain_architecture_tool() {
        let kg = create_test_knowledge_graph();
        let router = UmmonRouter::new(kg);
        
        // Test explain architecture
        let args = json!({
            "detail_level": "low"
        });
        
        let result = router.invoke_tool("explain_architecture", &args).await;
        assert!(result.is_ok(), "Explain architecture tool should return Ok result");
        
        let content = result.unwrap();
        assert!(!content.is_empty(), "Explain architecture result should not be empty");
        
        if let Content::Text(text) = &content[0] {
            assert!(text.contains("Codebase Architecture"), "Result should contain architecture title");
            assert!(text.contains("Module Structure"), "Result should contain module structure section");
        } else {
            panic!("Result should be text content");
        }
    }
    
    #[tokio::test]
    async fn test_tool_error_handling() {
        let kg = create_test_knowledge_graph();
        let router = UmmonRouter::new(kg);
        
        // Test missing parameter
        let args = json!({});
        let result = router.invoke_tool("search_code", &args).await;
        assert!(matches!(result, Err(ToolError::InvalidParams(_))), "Should return InvalidParams error");
        
        // Test invalid entity ID
        let args = json!({
            "entity_id": "non_existent_entity"
        });
        let result = router.invoke_tool("get_entity", &args).await;
        assert!(matches!(result, Err(ToolError::ExecutionFailed(_))), "Should return ExecutionFailed error");
        
        // Test invalid tool name
        let args = json!({});
        let result = router.invoke_tool("invalid_tool", &args).await;
        assert!(matches!(result, Err(ToolError::NotFound(_))), "Should return NotFound error");
    }
}