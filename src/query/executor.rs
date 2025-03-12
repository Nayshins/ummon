use anyhow::Result;
use std::f64;

use crate::graph::{
    entity::{Entity, EntityType},
    knowledge_graph::KnowledgeGraph,
};

#[cfg(test)]
use crate::graph::relationship::RelationshipType;

use super::parser::{
    ConditionNode, EntityTypeSelector, Operator, QueryType, SelectQuery, TraversalQuery, Value,
};

/// Executes a parsed query against the knowledge graph
pub struct QueryExecutor<'a> {
    kg: &'a KnowledgeGraph,
}

impl<'a> QueryExecutor<'a> {
    pub fn new(kg: &'a KnowledgeGraph) -> Self {
        Self { kg }
    }

    /// Execute a parsed query and return matching entities
    pub fn execute(&self, query: QueryType) -> Result<Vec<&'a dyn Entity>> {
        match query {
            QueryType::Select(select) => self.execute_select(select),
            QueryType::Traversal(traversal) => self.execute_traversal(traversal),
        }
    }

    /// Execute a select query
    fn execute_select(&self, query: SelectQuery) -> Result<Vec<&'a dyn Entity>> {
        // Get entities of the requested type
        let entities = self.get_entities_by_type(&query.entity_type);

        // Apply conditions if any
        if let Some(condition) = query.conditions {
            Ok(entities
                .into_iter()
                .filter(|e| self.evaluate_condition(e, &condition))
                .collect())
        } else {
            Ok(entities)
        }
    }

    /// Execute a traversal query
    fn execute_traversal(&self, query: TraversalQuery) -> Result<Vec<&'a dyn Entity>> {
        // Get all entities matching the source type
        let source_entities = self.get_entities_by_type(&query.source_type);

        // For each source entity, find related entities of the target type
        // through the specified relationship
        let mut result = Vec::new();

        for source in source_entities {
            // Get related entities
            let related = self
                .kg
                .get_related_entities(source.id(), Some(&query.relationship.relationship_type));

            // Filter by target entity type
            let target_type_related: Vec<&dyn Entity> = related
                .into_iter()
                .filter(|e| self.entity_matches_type(e, &query.target_type))
                .collect();

            // If conditions are specified, apply them
            let filtered_related = if let Some(ref condition) = query.conditions {
                target_type_related
                    .into_iter()
                    .filter(|e| self.evaluate_condition(e, condition))
                    .collect::<Vec<_>>()
            } else {
                target_type_related
            };

            // If there are any matches, include the source entity in results
            if !filtered_related.is_empty() {
                result.push(source);
            }
        }

        Ok(result)
    }

    /// Get entities of a specific type
    fn get_entities_by_type(&self, type_selector: &EntityTypeSelector) -> Vec<&'a dyn Entity> {
        self.kg.get_entities_by_type(&type_selector.entity_type)
    }

    /// Check if an entity matches the specified type
    fn entity_matches_type(
        &self,
        entity: &&dyn Entity,
        type_selector: &EntityTypeSelector,
    ) -> bool {
        entity.entity_type() == type_selector.entity_type
    }

    /// Evaluate a condition node against an entity
    fn evaluate_condition(&self, entity: &&dyn Entity, condition: &ConditionNode) -> bool {
        match condition {
            ConditionNode::And(left, right) => {
                self.evaluate_condition(entity, left) && self.evaluate_condition(entity, right)
            }
            ConditionNode::Or(left, right) => {
                self.evaluate_condition(entity, left) || self.evaluate_condition(entity, right)
            }
            ConditionNode::Not(inner) => !self.evaluate_condition(entity, inner),
            ConditionNode::HasAttribute(attr) => self.entity_has_attribute(entity, attr),
            ConditionNode::Condition {
                attribute,
                operator,
                value,
            } => self.evaluate_simple_condition(entity, attribute, operator, value),
        }
    }

    /// Check if an entity has a non-empty attribute
    fn entity_has_attribute(&self, entity: &&dyn Entity, attribute: &str) -> bool {
        match attribute {
            "name" => !entity.name().is_empty(),
            "file_path" | "path" => entity.file_path().is_some(),
            "documentation" => {
                // Get documentation from metadata
                if let Some(docs) = entity.metadata().get("documentation") {
                    !docs.is_empty()
                } else {
                    false
                }
            }
            // Check other attributes in metadata
            _ => entity.metadata().contains_key(attribute),
        }
    }

    /// Evaluate a simple condition (attribute operator value)
    fn evaluate_simple_condition(
        &self,
        entity: &&dyn Entity,
        attribute: &str,
        operator: &Operator,
        value: &Value,
    ) -> bool {
        // Get the attribute value
        let attr_value = match attribute {
            "name" => Some(entity.name().to_string()),
            "file_path" | "path" => entity.file_path().map(|p| p.to_string()),
            "confidence" => {
                // Special handling for domain concepts
                if let EntityType::DomainConcept = entity.entity_type() {
                    entity.metadata().get("confidence").map(|s| s.to_string())
                } else {
                    None
                }
            }
            // Check other attributes in metadata
            _ => entity.metadata().get(attribute).map(|s| s.to_string()),
        };

        // If the attribute doesn't exist, the condition is false
        let attr_value = match attr_value {
            Some(val) => val,
            None => return false,
        };

        // Compare the attribute value with the condition value
        self.compare_values(&attr_value, operator, value)
    }

    /// Compare a string attribute value with a condition value using the specified operator
    fn compare_values(&self, attr_value: &str, operator: &Operator, value: &Value) -> bool {
        match (operator, value) {
            // String comparisons
            (Operator::Equal, Value::String(s)) => attr_value == s,
            (Operator::NotEqual, Value::String(s)) => attr_value != s,
            (Operator::Like, Value::String(pattern)) => self.pattern_match(attr_value, pattern),

            // Numeric comparisons
            (Operator::Equal, Value::Number(n)) => {
                if let Ok(attr_num) = attr_value.parse::<f64>() {
                    (attr_num - n).abs() < f64::EPSILON
                } else {
                    false
                }
            }
            (Operator::NotEqual, Value::Number(n)) => {
                if let Ok(attr_num) = attr_value.parse::<f64>() {
                    (attr_num - n).abs() > f64::EPSILON
                } else {
                    true // If we can't parse as a number, they're not equal
                }
            }
            (Operator::GreaterThan, Value::Number(n)) => {
                if let Ok(attr_num) = attr_value.parse::<f64>() {
                    attr_num > *n
                } else {
                    false
                }
            }
            (Operator::LessThan, Value::Number(n)) => {
                if let Ok(attr_num) = attr_value.parse::<f64>() {
                    attr_num < *n
                } else {
                    false
                }
            }
            (Operator::GreaterThanOrEqual, Value::Number(n)) => {
                if let Ok(attr_num) = attr_value.parse::<f64>() {
                    attr_num >= *n
                } else {
                    false
                }
            }
            (Operator::LessThanOrEqual, Value::Number(n)) => {
                if let Ok(attr_num) = attr_value.parse::<f64>() {
                    attr_num <= *n
                } else {
                    false
                }
            }

            // Invalid combinations
            _ => false,
        }
    }

    /// Perform SQL LIKE-style pattern matching
    /// Supports % as a wildcard (e.g., "auth%" matches strings starting with "auth")
    fn pattern_match(&self, value: &str, pattern: &str) -> bool {
        // Convert SQL LIKE pattern to regex
        let regex_pattern = pattern
            .replace('%', ".*")
            .replace('_', ".")
            .replace('\\', "\\\\");

        // Anchor the pattern to the start and end
        let full_pattern = format!("^{}$", regex_pattern);

        // Compile and match
        match regex::Regex::new(&full_pattern) {
            Ok(re) => re.is_match(value),
            Err(_) => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::entity::{BaseEntity, EntityId, EntityType, FunctionEntity, Visibility};
    use crate::query::parser::parse_query;

    // Helper function to create a test knowledge graph
    fn create_test_kg() -> KnowledgeGraph {
        let mut kg = KnowledgeGraph::new();

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

        // Add entities to graph
        kg.add_entity(func1).unwrap();
        kg.add_entity(func2).unwrap();
        kg.add_entity(func3).unwrap();

        // Create relationships
        kg.create_relationship(func1_id.clone(), func2_id, RelationshipType::Calls)
            .unwrap();
        kg.create_relationship(func3_id, func1_id, RelationshipType::Calls)
            .unwrap();

        kg
    }

    #[test]
    fn test_execute_select_all_functions() {
        let kg = create_test_kg();
        let executor = QueryExecutor::new(&kg);

        let query = parse_query("select functions").unwrap();
        let results = executor.execute(query).unwrap();

        assert_eq!(results.len(), 3);
        let names: Vec<&str> = results.iter().map(|e| e.name()).collect();
        assert!(names.contains(&"auth_login"));
        assert!(names.contains(&"validate_token"));
        assert!(names.contains(&"get_user"));
    }

    #[test]
    fn test_execute_select_with_condition() {
        let kg = create_test_kg();
        let executor = QueryExecutor::new(&kg);

        let query = parse_query("select functions where name like 'auth%'").unwrap();
        let results = executor.execute(query).unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name(), "auth_login");
    }

    #[test]
    fn test_execute_traversal_query() {
        let kg = create_test_kg();
        let executor = QueryExecutor::new(&kg);

        let query = parse_query("functions calling functions").unwrap();
        let results = executor.execute(query).unwrap();

        assert_eq!(results.len(), 2);
        let names: Vec<&str> = results.iter().map(|e| e.name()).collect();
        assert!(names.contains(&"auth_login"));
        assert!(names.contains(&"get_user"));
    }

    #[test]
    fn test_execute_traversal_with_condition() {
        let kg = create_test_kg();
        let executor = QueryExecutor::new(&kg);

        let query =
            parse_query("functions calling functions where name = 'validate_token'").unwrap();
        let results = executor.execute(query).unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name(), "auth_login");
    }

    #[test]
    fn test_execute_complex_condition() {
        let kg = create_test_kg();
        let executor = QueryExecutor::new(&kg);

        // Use a simpler condition that works with our grammar
        let query =
            parse_query("select functions where name like 'auth%' and file_path like 'src/%'")
                .unwrap();
        let results = executor.execute(query).unwrap();

        assert_eq!(results.len(), 1);
        let names: Vec<&str> = results.iter().map(|e| e.name()).collect();
        assert!(names.contains(&"auth_login"));
    }
}
