use anyhow::{anyhow, Result};
use rusqlite::types::ToSql;

use crate::db::Database;
use crate::graph::entity::{Entity, EntityId};
use crate::graph::relationship::RelationshipType;

use super::parser::{ConditionNode, Operator, QueryType, SelectQuery, TraversalQuery, Value};

/// List of allowed column names for safe attribute access
const ALLOWED_COLUMNS: [&str; 4] = ["name", "file_path", "documentation", "id"];

/// SQL query with parameters, used to avoid SQL injection
pub struct SafeQuery {
    pub sql: String,
    pub params: Vec<Box<dyn ToSql>>,
}

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

        let safe_query = match &query.conditions {
            Some(condition) => Some(self.condition_to_sql(condition)?),
            None => None,
        };

        // Unpack the safe query into condition and parameters
        match safe_query {
            Some(sq) => self
                .db
                .query_entities_by_type(entity_type, Some(&sq.sql), sq.params),
            None => self.db.query_entities_by_type(entity_type, None, vec![]),
        }
    }

    /// Execute a traversal query using the database's find_paths method
    fn execute_traversal(&self, query: &TraversalQuery) -> Result<Vec<Box<dyn Entity>>> {
        let source_entities =
            self.db
                .query_entities_by_type(&query.source_type.entity_type, None, vec![])?;

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
                let entity_type = entity.entity_type();

                let mut safe_query = self.condition_to_sql(condition)?;

                let id_condition = "id = ?".to_string();
                let combined_sql = format!("{} AND ({})", id_condition, safe_query.sql);

                let mut combined_params: Vec<Box<dyn ToSql>> =
                    vec![Box::new(target_id.as_str().to_string())];
                combined_params.append(&mut safe_query.params);
                let matching_entities = self.db.query_entities_by_type(
                    &entity_type,
                    Some(&combined_sql),
                    combined_params,
                )?;

                if !matching_entities.is_empty() {
                    return Ok(true);
                }
            }
        }

        Ok(false)
    }

    /// Convert a condition to SQL where clause with parameterized values
    fn condition_to_sql(&self, condition: &ConditionNode) -> Result<SafeQuery> {
        match condition {
            ConditionNode::And(left, right) => {
                let left_query = self.condition_to_sql(left)?;
                let right_query = self.condition_to_sql(right)?;

                // Combine SQL parts
                let sql = format!("({}) AND ({})", left_query.sql, right_query.sql);

                // Combine parameters, preserving order
                let mut params = left_query.params;
                params.extend(right_query.params);

                Ok(SafeQuery { sql, params })
            }
            ConditionNode::Or(left, right) => {
                let left_query = self.condition_to_sql(left)?;
                let right_query = self.condition_to_sql(right)?;

                let sql = format!("({}) OR ({})", left_query.sql, right_query.sql);

                let mut params = left_query.params;
                params.extend(right_query.params);

                Ok(SafeQuery { sql, params })
            }
            ConditionNode::Not(inner) => {
                let inner_query = self.condition_to_sql(inner)?;
                let sql = format!("NOT ({})", inner_query.sql);

                Ok(SafeQuery {
                    sql,
                    params: inner_query.params,
                })
            }
            ConditionNode::HasAttribute(attr) => {
                let attr_name = self.validate_attribute_name(attr)?;

                match attr_name {
                    "name" => Ok(SafeQuery {
                        sql: "name IS NOT NULL AND name != ''".to_string(),
                        params: vec![],
                    }),
                    "file_path" => Ok(SafeQuery {
                        sql: "file_path IS NOT NULL".to_string(),
                        params: vec![],
                    }),
                    "documentation" => Ok(SafeQuery {
                        sql: "documentation IS NOT NULL".to_string(),
                        params: vec![],
                    }),
                    // For other attributes, use a parameterized LIKE query
                    _ => Err(anyhow!(
                        "Attribute '{}' is not supported for 'has' condition",
                        attr
                    )),
                }
            }
            ConditionNode::Condition {
                attribute,
                operator,
                value,
            } => {
                // Validate attribute name against allowed columns
                let attr_name = self.validate_attribute_name(attribute)?;

                let sql_op = match operator {
                    Operator::Equal => "=",
                    Operator::NotEqual => "!=",
                    Operator::GreaterThan => ">",
                    Operator::LessThan => "<",
                    Operator::GreaterThanOrEqual => ">=",
                    Operator::LessThanOrEqual => "<=",
                    Operator::Like => "LIKE",
                };

                // Create parameterized query with placeholder
                let sql = format!("{} {} ?", attr_name, sql_op);

                // Convert value to SQL parameter
                let param: Box<dyn ToSql> = match value {
                    Value::String(s) => Box::new(s.clone()),
                    Value::Number(n) => Box::new(*n),
                };

                Ok(SafeQuery {
                    sql,
                    params: vec![param],
                })
            }
        }
    }

    /// Validate an attribute name against the allowed column whitelist
    fn validate_attribute_name<'b>(&self, name: &'b str) -> Result<&'b str> {
        if ALLOWED_COLUMNS.contains(&name) {
            Ok(name)
        } else {
            Err(anyhow!(
                "Attribute '{}' is not supported or not allowed",
                name
            ))
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

    #[test]
    fn test_condition_to_sql_parameterization() {
        use super::super::parser::{ConditionNode, Operator, Value};

        let db = create_test_db();
        let executor = DbQueryExecutor::new(&db);

        // Test string condition
        let condition = ConditionNode::Condition {
            attribute: "name".to_string(),
            operator: Operator::Equal,
            value: Value::String("test".to_string()),
        };

        let result = executor.condition_to_sql(&condition).unwrap();
        assert_eq!(result.sql, "name = ?");
        assert_eq!(result.params.len(), 1);

        // Test complex condition
        let complex = ConditionNode::And(
            Box::new(ConditionNode::Condition {
                attribute: "name".to_string(),
                operator: Operator::Like,
                value: Value::String("test%".to_string()),
            }),
            Box::new(ConditionNode::Condition {
                attribute: "file_path".to_string(),
                operator: Operator::Like,
                value: Value::String("src/%".to_string()),
            }),
        );

        let result = executor.condition_to_sql(&complex).unwrap();
        assert_eq!(result.sql, "(name LIKE ?) AND (file_path LIKE ?)");
        assert_eq!(result.params.len(), 2);
    }
}
