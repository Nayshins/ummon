use anyhow::{anyhow, Result};

use crate::db::Database;
use crate::graph::entity::{Entity, EntityId};
use crate::graph::relationship::RelationshipType;

use super::parser::{ConditionNode, Operator, QueryType, SelectQuery, TraversalQuery, Value};

/// Executes a parsed query against the SQLite database directly
pub struct DbQueryExecutor<'a> {
    db: &'a Database,
}

impl<'a> DbQueryExecutor<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    /// Execute a parsed query and return matching entities
    pub fn execute(&self, query: QueryType) -> Result<Vec<Box<dyn Entity>>> {
        match query {
            QueryType::Select(select) => self.execute_select(&select),
            QueryType::Traversal(traversal) => self.execute_traversal(&traversal),
        }
    }

    /// Execute a select query using direct SQL
    fn execute_select(&self, query: &SelectQuery) -> Result<Vec<Box<dyn Entity>>> {
        let entity_type = &query.entity_type.entity_type;

        let sql_condition = match &query.conditions {
            Some(condition) => Some(self.condition_to_sql(condition)?),
            None => None,
        };

        self.db
            .query_entities_by_type(entity_type, sql_condition.as_deref())
    }

    /// Execute a traversal query using the database's find_paths method
    fn execute_traversal(&self, query: &TraversalQuery) -> Result<Vec<Box<dyn Entity>>> {
        let source_entities = self
            .db
            .query_entities_by_type(&query.source_type.entity_type, None)?;

        let mut result_entities = Vec::new();

        for source_entity in source_entities {
            let direction = match query.relationship.relationship_type {
                RelationshipType::Calls => "outbound",
                RelationshipType::Contains => "outbound",
                RelationshipType::Imports => "outbound",
                RelationshipType::Inherits => "outbound",
                RelationshipType::Implements => "outbound",
                RelationshipType::References => "outbound",
                RelationshipType::Defines => "outbound",
                RelationshipType::Uses => "outbound",
                RelationshipType::Depends => "outbound",
                RelationshipType::RepresentedBy => "outbound",
                RelationshipType::RelatesTo => "both",
                RelationshipType::DependsOn => "outbound",
                RelationshipType::Other(_) => "both",
            };

            let paths = self.db.find_paths(
                source_entity.id(),
                None, // to_id is None because we're looking for any target of the correct type
                Some(&query.target_type.entity_type),
                Some(&query.relationship.relationship_type),
                10, // reasonable max depth
                direction,
            )?;

            if !paths.is_empty() {
                if let Some(ref condition) = query.conditions {
                    let has_valid_target = self.check_traversal_targets(&paths, condition)?;
                    if has_valid_target {
                        result_entities.push(source_entity);
                    }
                } else {
                    result_entities.push(source_entity);
                }
            }
        }

        Ok(result_entities)
    }

    /// Check if any traversal target meets the conditions
    fn check_traversal_targets(
        &self,
        paths: &[(EntityId, usize)],
        condition: &ConditionNode,
    ) -> Result<bool> {
        // Only consider entities that are targets (not the source) - depth > 0
        let target_ids: Vec<&EntityId> = paths
            .iter()
            .filter(|(_, depth)| *depth > 0)
            .map(|(id, _)| id)
            .collect();

        if target_ids.is_empty() {
            return Ok(false);
        }

        for target_id in target_ids {
            if let Some(entity) = self.db.load_entity(target_id)? {
                let sql_condition = self.condition_to_sql(condition)?;
                let entity_type = entity.entity_type();
                
                let id_condition = format!("id = '{}'", target_id.as_str());
                let combined_condition = format!("{} AND {}", id_condition, sql_condition);

                let matching_entities = self
                    .db
                    .query_entities_by_type(&entity_type, Some(&combined_condition))?;

                if !matching_entities.is_empty() {
                    return Ok(true);
                }
            }
        }

        Ok(false)
    }

    /// Convert a condition to SQL where clause
    fn condition_to_sql(&self, condition: &ConditionNode) -> Result<String> {
        match condition {
            ConditionNode::And(left, right) => {
                let left_sql = self.condition_to_sql(left)?;
                let right_sql = self.condition_to_sql(right)?;
                Ok(format!("({}) AND ({})", left_sql, right_sql))
            }
            ConditionNode::Or(left, right) => {
                let left_sql = self.condition_to_sql(left)?;
                let right_sql = self.condition_to_sql(right)?;
                Ok(format!("({}) OR ({})", left_sql, right_sql))
            }
            ConditionNode::Not(inner) => {
                let inner_sql = self.condition_to_sql(inner)?;
                Ok(format!("NOT ({})", inner_sql))
            }
            ConditionNode::HasAttribute(attr) => {
                match attr.as_str() {
                    "name" => Ok("name IS NOT NULL AND name != ''".to_string()),
                    "file_path" | "path" => Ok("file_path IS NOT NULL".to_string()),
                    "documentation" => Ok("documentation IS NOT NULL".to_string()),
                    // For other attributes, we'd need to check metadata in JSON
                    // This is simplified, as proper handling would need JSON extraction
                    _ => Ok(format!("data LIKE '%{}%'", attr)),
                }
            }
            ConditionNode::Condition {
                attribute,
                operator,
                value,
            } => {
                let attr_name = match attribute.as_str() {
                    "name" => "name",
                    "file_path" | "path" => "file_path",
                    "documentation" => "documentation",
                    // For other attributes, we'd need to check metadata or data JSON
                    // This is simplified and might not work for all attributes
                    _ => {
                        return Err(anyhow!(
                            "Attribute {} is not directly supported in SQL conversion",
                            attribute
                        ))
                    }
                };

                let sql_op = match operator {
                    Operator::Equal => "=",
                    Operator::NotEqual => "!=",
                    Operator::GreaterThan => ">",
                    Operator::LessThan => "<",
                    Operator::GreaterThanOrEqual => ">=",
                    Operator::LessThanOrEqual => "<=",
                    Operator::Like => "LIKE",
                };

                let sql_value = match value {
                    Value::String(s) => format!("'{}'", s.replace('\'', "''")), // Escape single quotes for SQL safety
                    Value::Number(n) => n.to_string(),
                };

                Ok(format!("{} {} {}", attr_name, sql_op, sql_value))
            }
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

    // Helper function to create a test database with sample data
    fn create_test_db() -> Database {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let db = get_database(db_path.to_str().unwrap()).unwrap();

        // Create function entities
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

        // Save entities to database
        db.save_entity(&func1).unwrap();
        db.save_entity(&func2).unwrap();

        // Create a relationship
        let rel_id = crate::graph::relationship::RelationshipId::new("calls_rel");
        let rel = crate::graph::relationship::Relationship::new(
            rel_id,
            func1_id,
            func2_id,
            RelationshipType::Calls,
        );
        db.save_relationship(&rel).unwrap();

        db
    }

    #[test]
    fn test_execute_select_query() {
        let db = create_test_db();
        let executor = DbQueryExecutor::new(&db);

        let query = parse_query("select functions").unwrap();
        let results = executor.execute(query).unwrap();

        assert_eq!(results.len(), 2);
        assert!(results.iter().any(|e| e.name() == "auth_login"));
        assert!(results.iter().any(|e| e.name() == "validate_token"));
    }

    #[test]
    fn test_execute_select_with_condition() {
        let db = create_test_db();
        let executor = DbQueryExecutor::new(&db);

        let query = parse_query("select functions where name = 'auth_login'").unwrap();
        let results = executor.execute(query).unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name(), "auth_login");
    }
}
