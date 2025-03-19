use anyhow::Result;
use std::collections::HashMap;
use std::f64;

use crate::db::Database;
use crate::graph::entity::{Entity, EntityId, EntityType};
use crate::graph::relationship::{Relationship, RelationshipId, RelationshipType};

use super::parser::{
    ConditionNode, EntityTypeSelector, Operator, QueryType, SelectQuery, TraversalQuery, Value,
};

/// Executes a parsed query against the knowledge database using SQLite
pub struct QueryExecutor<'a> {
    db: &'a Database,
}

impl<'a> QueryExecutor<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    /// Execute a parsed query and return the results as formatted string
    pub fn execute(&self, query: QueryType) -> Result<String> {
        match query {
            QueryType::Select(select) => self.execute_select(&select),
            QueryType::Traversal(traversal) => self.execute_traversal(&traversal),
        }
    }

    /// Execute a select query using SQL directly
    fn execute_select(&self, query: &SelectQuery) -> Result<String> {
        // Convert query conditions to SQL parameter pairs
        let conditions = self.build_conditions(&query.conditions);

        // Execute the query against the database
        let entities = self
            .db
            .execute_entity_select(&query.entity_type.entity_type, &conditions)?;

        // Format the results
        let formatter = ResultFormatter::new(&OutputFormat::Text);
        formatter.format_entities(&entities)
    }

    /// Execute a traversal query using SQL's recursive CTE
    fn execute_traversal(&self, query: &TraversalQuery) -> Result<String> {
        // Get all entities matching the source type
        let source_conditions = self.build_conditions(&query.conditions);
        let source_entities = self
            .db
            .execute_entity_select(&query.source_type.entity_type, &source_conditions)?;

        let mut all_paths = Vec::new();

        // For each source entity, run a traversal query
        for entity in &source_entities {
            let direction = match query.relationship.direction {
                TraversalDirection::Outbound => "outbound",
                TraversalDirection::Inbound => "inbound",
                TraversalDirection::Both => "both",
            };

            let paths = self.db.execute_traversal(
                entity.id(),
                Some(&query.relationship.relationship_type),
                direction,
                query.max_depth,
            )?;

            if !paths.is_empty() {
                // Add all paths to our result
                all_paths.extend(paths);
            }
        }

        // Format the traversal results
        let formatter = ResultFormatter::new(&OutputFormat::Text);
        formatter.format_paths(&all_paths)
    }

    /// Convert ConditionNode tree to flat SQL-compatible conditions
    fn build_conditions(&self, condition: &Option<ConditionNode>) -> Vec<(String, String)> {
        let mut result = Vec::new();

        if let Some(condition) = condition {
            match condition {
                ConditionNode::Condition {
                    attribute,
                    operator,
                    value,
                } => {
                    // Only handle simple equals conditions for now
                    if let (Operator::Equal, Value::String(s)) = (operator, value) {
                        if attribute == "name" {
                            result.push(("name".to_string(), s.clone()));
                        } else if attribute == "file_path" {
                            result.push(("file_path".to_string(), s.clone()));
                        }
                        // Add other attributes as needed
                    }
                }
                // For now, we only support simple conditions directly in SQL
                // More complex conditions would require post-filtering
                _ => {}
            }
        }

        result
    }
}

/// Direction for traversal queries
#[derive(Debug, Clone)]
pub enum TraversalDirection {
    Outbound,
    Inbound,
    Both,
}

/// Output format for query results
#[derive(Debug, Clone)]
pub enum OutputFormat {
    Text,
    Json,
    Csv,
}

/// Formatter for query results
pub struct ResultFormatter {
    format: OutputFormat,
}

impl ResultFormatter {
    pub fn new(format: &OutputFormat) -> Self {
        Self {
            format: match format {
                OutputFormat::Text => OutputFormat::Text,
                OutputFormat::Json => OutputFormat::Json,
                OutputFormat::Csv => OutputFormat::Csv,
            },
        }
    }

    /// Format a collection of entities as a string
    pub fn format_entities(&self, entities: &[Box<dyn Entity>]) -> Result<String> {
        match self.format {
            OutputFormat::Text => {
                let mut result = String::new();
                result.push_str(&format!("Found {} entities:\n\n", entities.len()));

                for (i, entity) in entities.iter().enumerate() {
                    result.push_str(&format!("{}. {}\n", i + 1, entity.name()));
                    result.push_str(&format!("   Type: {:?}\n", entity.entity_type()));
                    if let Some(path) = entity.file_path() {
                        result.push_str(&format!("   Path: {}\n", path));
                    }
                    result.push_str("\n");
                }

                Ok(result)
            }
            // Implement other formats as needed
            _ => Ok(format!("Found {} entities", entities.len())),
        }
    }

    /// Format traversal paths as a string
    pub fn format_paths(&self, paths: &[(EntityId, usize)]) -> Result<String> {
        match self.format {
            OutputFormat::Text => {
                let mut result = String::new();
                result.push_str(&format!("Found {} path nodes:\n\n", paths.len()));

                for (i, (entity_id, depth)) in paths.iter().enumerate() {
                    result.push_str(&format!(
                        "{}. {} (depth: {})\n",
                        i + 1,
                        entity_id.as_str(),
                        depth
                    ));
                }

                Ok(result)
            }
            // Implement other formats as needed
            _ => Ok(format!("Found {} paths", paths.len())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::get_database;
    use crate::graph::entity::{BaseEntity, EntityId, EntityType, FunctionEntity, Visibility};
    use crate::query::parser::parse_query;
    use tempfile::tempdir;

    // Helper function to create a test database
    fn create_test_db() -> Database {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let db = get_database(db_path.to_str().unwrap()).unwrap();

        // Add function entities
        let func1_id = EntityId::new("func1");
        let base_func1 = BaseEntity::new(
            func1_id.clone(),
            "auth_login".to_string(),
            EntityType::Function,
            Some("src/auth.rs".to_string()),
        );
        let func1 = FunctionEntity {
            base: base_func1,
            parameters: vec![],
            return_type: None,
            visibility: Visibility::Public,
            is_async: false,
            is_static: false,
            is_constructor: false,
            is_abstract: false,
        };

        let func2_id = EntityId::new("func2");
        let base_func2 = BaseEntity::new(
            func2_id.clone(),
            "validate_token".to_string(),
            EntityType::Function,
            Some("src/auth.rs".to_string()),
        );
        let func2 = FunctionEntity {
            base: base_func2,
            parameters: vec![],
            return_type: None,
            visibility: Visibility::Public,
            is_async: false,
            is_static: false,
            is_constructor: false,
            is_abstract: false,
        };

        let func3_id = EntityId::new("func3");
        let base_func3 = BaseEntity::new(
            func3_id.clone(),
            "get_user".to_string(),
            EntityType::Function,
            Some("src/user.rs".to_string()),
        );
        let func3 = FunctionEntity {
            base: base_func3,
            parameters: vec![],
            return_type: None,
            visibility: Visibility::Public,
            is_async: false,
            is_static: false,
            is_constructor: false,
            is_abstract: false,
        };

        // Save entities to database
        db.save_entity(&func1).unwrap();
        db.save_entity(&func2).unwrap();
        db.save_entity(&func3).unwrap();

        // Create relationships
        let rel1 = Relationship::new(
            RelationshipId::new("rel1"),
            func1_id.clone(),
            func2_id.clone(),
            RelationshipType::Calls,
        );

        let rel2 = Relationship::new(
            RelationshipId::new("rel2"),
            func3_id.clone(),
            func1_id.clone(),
            RelationshipType::Calls,
        );

        db.save_relationship(&rel1).unwrap();
        db.save_relationship(&rel2).unwrap();

        db
    }

    #[test]
    fn test_execute_select_basic() {
        let db = create_test_db();
        let executor = QueryExecutor::new(&db);

        let query = parse_query("select functions").unwrap();
        let result = executor.execute(query).unwrap();

        // Simple assertions to check that results contain expected data
        assert!(result.contains("Found 3 entities"));
        assert!(result.contains("auth_login"));
        assert!(result.contains("validate_token"));
        assert!(result.contains("get_user"));
    }

    #[test]
    fn test_execute_select_with_condition() {
        let db = create_test_db();
        let executor = QueryExecutor::new(&db);

        let query = parse_query("select functions where name = 'auth_login'").unwrap();
        let result = executor.execute(query).unwrap();

        assert!(result.contains("Found 1 entities"));
        assert!(result.contains("auth_login"));
        assert!(!result.contains("validate_token"));
    }

    #[test]
    fn test_traversal_basic() {
        let db = create_test_db();
        let executor = QueryExecutor::new(&db);

        let query = parse_query("functions calling functions").unwrap();
        let result = executor.execute(query).unwrap();

        assert!(result.contains("path nodes"));
        // Additional assertions can be made based on expected output format
    }
}
