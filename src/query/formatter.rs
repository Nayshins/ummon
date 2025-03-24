use anyhow::Result;
use serde_json::{json, Value};
use std::collections::HashSet;

use crate::graph::entity::Entity;
use crate::graph::knowledge_graph::KnowledgeGraph;

/// Supported output formats
pub enum OutputFormat {
    Json,
    Text,
    Tree,
    Csv,
}

impl std::str::FromStr for OutputFormat {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "json" => Ok(OutputFormat::Json),
            "tree" => Ok(OutputFormat::Tree),
            "csv" => Ok(OutputFormat::Csv),
            _ => Ok(OutputFormat::Text), // Default to text format
        }
    }
}

/// Format query results based on the specified output format
pub struct ResultFormatter<'a> {
    kg: Option<&'a KnowledgeGraph>,
    format: OutputFormat,
}

impl<'a> ResultFormatter<'a> {
    /// Create a new formatter with knowledge graph (for traditional query)
    pub fn new(kg: &'a KnowledgeGraph, format: OutputFormat) -> Self {
        Self {
            kg: Some(kg),
            format,
        }
    }

    /// Create a new formatter without knowledge graph (for database-only query)
    pub fn new_for_boxed_entities(format: OutputFormat) -> Self {
        Self { kg: None, format }
    }

    /// Format query results (reference entities)
    pub fn format(&self, entities: Vec<&dyn Entity>) -> Result<String> {
        match self.format {
            OutputFormat::Json => self.format_json(entities),
            OutputFormat::Text => self.format_text(entities),
            OutputFormat::Tree => {
                if let Some(kg) = self.kg {
                    self.format_tree(entities, kg)
                } else {
                    Err(anyhow::anyhow!("Tree format requires knowledge graph"))
                }
            }
            OutputFormat::Csv => self.format_csv(entities),
        }
    }

    /// Format query results (boxed entities)
    pub fn format_boxed_entities(&self, entities: &[Box<dyn Entity>]) -> Result<String> {
        match self.format {
            OutputFormat::Json => self.format_json_boxed(entities),
            OutputFormat::Text => self.format_text_boxed(entities),
            OutputFormat::Tree => Err(anyhow::anyhow!(
                "Tree format not supported for boxed entities"
            )),
            OutputFormat::Csv => self.format_csv_boxed(entities),
        }
    }

    /// Format as JSON (default)
    fn format_json(&self, entities: Vec<&dyn Entity>) -> Result<String> {
        let json_entities: Vec<Value> = entities
            .iter()
            .map(|e| {
                let mut entity_map = serde_json::Map::new();
                entity_map.insert("id".to_string(), json!(e.id().as_str()));
                entity_map.insert("name".to_string(), json!(e.name()));
                entity_map.insert("type".to_string(), json!(format!("{:?}", e.entity_type())));

                if let Some(path) = e.file_path() {
                    entity_map.insert("file_path".to_string(), json!(path));
                }

                // Include metadata
                let metadata = e.metadata();
                if !metadata.is_empty() {
                    entity_map.insert("metadata".to_string(), json!(metadata));
                }

                json!(entity_map)
            })
            .collect();

        Ok(serde_json::to_string_pretty(&json_entities)?)
    }

    /// Format as JSON for boxed entities
    fn format_json_boxed(&self, entities: &[Box<dyn Entity>]) -> Result<String> {
        let json_entities: Vec<Value> = entities
            .iter()
            .map(|e| {
                let mut entity_map = serde_json::Map::new();
                entity_map.insert("id".to_string(), json!(e.id().as_str()));
                entity_map.insert("name".to_string(), json!(e.name()));
                entity_map.insert("type".to_string(), json!(format!("{:?}", e.entity_type())));

                if let Some(path) = e.file_path() {
                    entity_map.insert("file_path".to_string(), json!(path));
                }

                // Include metadata
                let metadata = e.metadata();
                if !metadata.is_empty() {
                    entity_map.insert("metadata".to_string(), json!(metadata));
                }

                json!(entity_map)
            })
            .collect();

        Ok(serde_json::to_string_pretty(&json_entities)?)
    }

    /// Format as plain text
    fn format_text(&self, entities: Vec<&dyn Entity>) -> Result<String> {
        if entities.is_empty() {
            return Ok("No entities found".to_string());
        }

        let mut result = String::new();

        for entity in entities {
            result.push_str(&format!("{} ({})", entity.name(), entity.id().as_str()));

            if let Some(path) = entity.file_path() {
                result.push_str(&format!(" [{}]", path));
            }

            result.push('\n');
        }

        Ok(result)
    }

    /// Format as plain text for boxed entities
    fn format_text_boxed(&self, entities: &[Box<dyn Entity>]) -> Result<String> {
        if entities.is_empty() {
            return Ok("No entities found".to_string());
        }

        let mut result = String::new();

        for entity in entities {
            result.push_str(&format!("{} ({})", entity.name(), entity.id().as_str()));

            if let Some(path) = entity.file_path() {
                result.push_str(&format!(" [{}]", path));
            }

            result.push('\n');
        }

        Ok(result)
    }

    /// Format as hierarchical tree
    fn format_tree(&self, entities: Vec<&dyn Entity>, kg: &KnowledgeGraph) -> Result<String> {
        if entities.is_empty() {
            return Ok("No entities found".to_string());
        }

        let mut result = String::new();
        let mut processed = HashSet::new();

        for entity in entities {
            if processed.contains(entity.id().as_str()) {
                continue;
            }

            processed.insert(entity.id().as_str());

            // Add the entity as a root node
            result.push_str(&format!("{} ({:?})\n", entity.name(), entity.entity_type()));

            // Get outgoing relationships and add them as child nodes
            let outgoing = kg.get_outgoing_relationships(entity.id());

            for (i, rel) in outgoing.iter().enumerate() {
                let is_last = i == outgoing.len() - 1;
                let prefix = if is_last { "└─ " } else { "├─ " };

                if let Some(target) = kg.get_entity(&rel.target_id) {
                    result.push_str(&format!(
                        "  {}{} ({:?}) <- {:?}\n",
                        prefix,
                        target.name(),
                        target.entity_type(),
                        rel.relationship_type
                    ));

                    processed.insert(rel.target_id.as_str());
                }
            }

            // Add a blank line between root nodes
            if !outgoing.is_empty() {
                result.push('\n');
            }
        }

        Ok(result)
    }

    /// Format as CSV
    fn format_csv(&self, entities: Vec<&dyn Entity>) -> Result<String> {
        if entities.is_empty() {
            return Ok("No entities found".to_string());
        }

        // Collect all possible attributes
        let all_attributes = ["id", "name", "type", "file_path"];
        let mut all_metadata_keys = HashSet::new();

        for entity in &entities {
            for key in entity.metadata().keys() {
                all_metadata_keys.insert(key.as_str());
            }
        }

        let mut sorted_metadata_keys: Vec<&str> = all_metadata_keys.into_iter().collect();
        sorted_metadata_keys.sort();

        // Build the header row
        let mut header = all_attributes.to_vec();
        header.extend(sorted_metadata_keys.iter().cloned());

        let mut result = header.join(",");
        result.push('\n');

        // Add entity rows
        for entity in entities {
            let mut row = Vec::new();

            // Add basic attributes
            row.push(entity.id().as_str().to_string());
            row.push(entity.name().to_string());
            row.push(format!("{:?}", entity.entity_type()));
            row.push(entity.file_path().map_or("".to_string(), |p| p.to_string()));

            // Add metadata values
            for &key in &sorted_metadata_keys {
                let value = entity
                    .metadata()
                    .get(key)
                    .map_or("".to_string(), |v| v.to_string());
                row.push(self.escape_csv_value(&value));
            }

            result.push_str(&row.join(","));
            result.push('\n');
        }

        Ok(result)
    }

    /// Format as CSV for boxed entities
    fn format_csv_boxed(&self, entities: &[Box<dyn Entity>]) -> Result<String> {
        if entities.is_empty() {
            return Ok("No entities found".to_string());
        }

        // Collect all possible attributes
        let all_attributes = ["id", "name", "type", "file_path"];
        let mut all_metadata_keys = HashSet::new();

        for entity in entities {
            for key in entity.metadata().keys() {
                all_metadata_keys.insert(key.as_str());
            }
        }

        let mut sorted_metadata_keys: Vec<&str> = all_metadata_keys.into_iter().collect();
        sorted_metadata_keys.sort();

        // Build the header row
        let mut header = all_attributes.to_vec();
        header.extend(sorted_metadata_keys.iter().cloned());

        let mut result = header.join(",");
        result.push('\n');

        // Add entity rows
        for entity in entities {
            let mut row = Vec::new();

            // Add basic attributes
            row.push(entity.id().as_str().to_string());
            row.push(entity.name().to_string());
            row.push(format!("{:?}", entity.entity_type()));
            row.push(entity.file_path().map_or("".to_string(), |p| p.to_string()));

            // Add metadata values
            for &key in &sorted_metadata_keys {
                let value = entity
                    .metadata()
                    .get(key)
                    .map_or("".to_string(), |v| v.to_string());
                row.push(self.escape_csv_value(&value));
            }

            result.push_str(&row.join(","));
            result.push('\n');
        }

        Ok(result)
    }

    /// Escape a CSV value
    fn escape_csv_value(&self, value: &str) -> String {
        if value.contains(',') || value.contains('"') || value.contains('\n') {
            let escaped = value.replace('"', "\"\"");
            format!("\"{}\"", escaped)
        } else {
            value.to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::entity::{BaseEntity, EntityId, EntityType, FunctionEntity, Visibility};
    use crate::graph::relationship::RelationshipType;

    // Helper function to create a test knowledge graph
    fn create_test_kg() -> KnowledgeGraph {
        let mut kg = KnowledgeGraph::new();

        // Create a function entity with metadata
        let id1 = EntityId::new("func1");
        let mut base1 = BaseEntity::new(
            id1.clone(),
            "test_function".to_string(),
            EntityType::Function,
            Some("src/test.rs".to_string()),
        );

        base1
            .metadata
            .insert("author".to_string(), "TestUser".to_string());
        base1
            .metadata
            .insert("description".to_string(), "A test function".to_string());

        let func = FunctionEntity {
            base: base1,
            parameters: vec![],
            return_type: None,
            visibility: Visibility::Public,
            is_async: false,
            is_static: false,
            is_constructor: false,
            is_abstract: false,
        };

        // Create another function with different metadata
        let id2 = EntityId::new("func2");
        let mut base2 = BaseEntity::new(
            id2.clone(),
            "another_function".to_string(),
            EntityType::Function,
            Some("src/test.rs".to_string()),
        );

        base2
            .metadata
            .insert("author".to_string(), "AnotherUser".to_string());

        let func2 = FunctionEntity {
            base: base2,
            parameters: vec![],
            return_type: None,
            visibility: Visibility::Public,
            is_async: false,
            is_static: false,
            is_constructor: false,
            is_abstract: false,
        };

        // Add entities and relationship
        kg.add_entity(func).unwrap();
        kg.add_entity(func2).unwrap();
        kg.create_relationship(id1, id2, RelationshipType::Calls)
            .unwrap();

        kg
    }

    // Helper function to create test boxed entities
    fn create_test_boxed_entities() -> Vec<Box<dyn Entity>> {
        let mut entities = Vec::new();

        // Create a function entity with metadata
        let id1 = EntityId::new("func1");
        let mut base1 = BaseEntity::new(
            id1.clone(),
            "test_function".to_string(),
            EntityType::Function,
            Some("src/test.rs".to_string()),
        );

        base1
            .metadata
            .insert("author".to_string(), "TestUser".to_string());
        base1
            .metadata
            .insert("description".to_string(), "A test function".to_string());

        let func = FunctionEntity {
            base: base1,
            parameters: vec![],
            return_type: None,
            visibility: Visibility::Public,
            is_async: false,
            is_static: false,
            is_constructor: false,
            is_abstract: false,
        };

        entities.push(Box::new(func) as Box<dyn Entity>);

        // Create another function with different metadata
        let id2 = EntityId::new("func2");
        let mut base2 = BaseEntity::new(
            id2.clone(),
            "another_function".to_string(),
            EntityType::Function,
            Some("src/test.rs".to_string()),
        );

        base2
            .metadata
            .insert("author".to_string(), "AnotherUser".to_string());

        let func2 = FunctionEntity {
            base: base2,
            parameters: vec![],
            return_type: None,
            visibility: Visibility::Public,
            is_async: false,
            is_static: false,
            is_constructor: false,
            is_abstract: false,
        };

        entities.push(Box::new(func2) as Box<dyn Entity>);

        entities
    }

    #[test]
    fn test_format_json() {
        let kg = create_test_kg();
        let formatter = ResultFormatter::new(&kg, OutputFormat::Json);

        let entities = kg.get_all_entities();
        let json_result = formatter.format(entities).unwrap();

        assert!(json_result.contains("test_function"));
        assert!(json_result.contains("another_function"));
        assert!(json_result.contains("author"));
        assert!(json_result.contains("TestUser"));
        assert!(json_result.contains("AnotherUser"));
    }

    #[test]
    fn test_format_json_boxed() {
        let formatter = ResultFormatter::new_for_boxed_entities(OutputFormat::Json);
        let boxed_entities = create_test_boxed_entities();

        let json_result = formatter.format_boxed_entities(&boxed_entities).unwrap();

        assert!(json_result.contains("test_function"));
        assert!(json_result.contains("another_function"));
        assert!(json_result.contains("author"));
        assert!(json_result.contains("TestUser"));
        assert!(json_result.contains("AnotherUser"));
    }

    #[test]
    fn test_format_text() {
        let kg = create_test_kg();
        let formatter = ResultFormatter::new(&kg, OutputFormat::Text);

        let entities = kg.get_all_entities();
        let text_result = formatter.format(entities).unwrap();

        assert!(text_result.contains("test_function"));
        assert!(text_result.contains("another_function"));
        assert!(text_result.contains("func1"));
        assert!(text_result.contains("func2"));
        assert!(text_result.contains("src/test.rs"));
    }

    #[test]
    fn test_format_text_boxed() {
        let formatter = ResultFormatter::new_for_boxed_entities(OutputFormat::Text);
        let boxed_entities = create_test_boxed_entities();

        let text_result = formatter.format_boxed_entities(&boxed_entities).unwrap();

        assert!(text_result.contains("test_function"));
        assert!(text_result.contains("another_function"));
        assert!(text_result.contains("func1"));
        assert!(text_result.contains("func2"));
        assert!(text_result.contains("src/test.rs"));
    }

    #[test]
    fn test_format_tree() {
        let kg = create_test_kg();
        let formatter = ResultFormatter::new(&kg, OutputFormat::Tree);

        let entities = kg.get_all_entities();
        let tree_result = formatter.format(entities).unwrap();

        assert!(tree_result.contains("test_function"));
        assert!(tree_result.contains("another_function"));
        assert!(tree_result.contains("Calls"));
    }

    #[test]
    fn test_format_csv() {
        let kg = create_test_kg();
        let formatter = ResultFormatter::new(&kg, OutputFormat::Csv);

        let entities = kg.get_all_entities();
        let csv_result = formatter.format(entities).unwrap();

        assert!(csv_result.contains("id,name,type,file_path,author,description"));
        assert!(csv_result
            .contains("func1,test_function,Function,src/test.rs,TestUser,A test function"));
        assert!(csv_result.contains("func2,another_function,Function,src/test.rs,AnotherUser,"));
    }

    #[test]
    fn test_format_csv_boxed() {
        let formatter = ResultFormatter::new_for_boxed_entities(OutputFormat::Csv);
        let boxed_entities = create_test_boxed_entities();

        let csv_result = formatter.format_boxed_entities(&boxed_entities).unwrap();

        assert!(csv_result.contains("id,name,type,file_path,author,description"));
        assert!(csv_result
            .contains("func1,test_function,Function,src/test.rs,TestUser,A test function"));
        assert!(csv_result.contains("func2,another_function,Function,src/test.rs,AnotherUser,"));
    }

    #[test]
    fn test_empty_results() {
        let kg = create_test_kg();
        let formatter = ResultFormatter::new(&kg, OutputFormat::Text);

        let empty_entities: Vec<&dyn Entity> = vec![];
        let result = formatter.format(empty_entities).unwrap();

        assert_eq!("No entities found", result);
    }

    #[test]
    fn test_empty_results_boxed() {
        let formatter = ResultFormatter::new_for_boxed_entities(OutputFormat::Text);

        let empty_entities: Vec<Box<dyn Entity>> = vec![];
        let result = formatter.format_boxed_entities(&empty_entities).unwrap();

        assert_eq!("No entities found", result);
    }
}
