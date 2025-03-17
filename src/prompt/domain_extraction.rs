use anyhow::Result;
use std::collections::HashMap;
use std::env;
use std::str::FromStr;

use crate::parser::domain_model::{
    AttributeType, DomainEntity, DomainModelBuilder, EntityType, RelationType, Relationship,
};
use crate::prompt::llm_integration::{query_llm, LlmConfig, LlmProvider};

/// Domain model builder that uses an LLM to extract domain entities
pub struct LlmModelExtractor {
    pub config: LlmConfig,
}

impl Default for LlmModelExtractor {
    fn default() -> Self {
        Self::new()
    }
}

impl LlmModelExtractor {
    pub fn new() -> Self {
        // Create default config
        let mut config = LlmConfig::default();

        // Try to determine the provider and API key from environment variables
        if let Ok(provider_str) = env::var("LLM_PROVIDER") {
            if let Ok(provider) = LlmProvider::from_str(&provider_str) {
                config.provider = provider;
                tracing::info!("Using LLM provider: {:?}", provider);
            } else {
                tracing::warn!("Unknown LLM provider '{}', using default", provider_str);
            }
        }

        // Check for API keys based on the provider
        let api_key = match config.provider {
            LlmProvider::OpenRouter => env::var("OPENROUTER_API_KEY").ok(),
            LlmProvider::OpenAI => env::var("OPENAI_API_KEY").ok(),
            LlmProvider::Anthropic => env::var("ANTHROPIC_API_KEY").ok(),
            LlmProvider::GoogleVertexAI => env::var("GOOGLE_API_KEY").ok(),
            LlmProvider::Ollama => Some(String::new()), // Ollama doesn't need an API key
        };

        // Set the API key if found
        if let Some(key) = api_key {
            if !key.is_empty() || config.provider == LlmProvider::Ollama {
                tracing::info!(
                    "Found API key for {:?}, LLM domain extraction enabled",
                    config.provider
                );
                config.api_key = key;
            }
        } else {
            tracing::warn!(
                "{:?} API key not set, LLM domain extraction disabled",
                config.provider
            );
        }

        // Check for custom endpoint
        if let Ok(endpoint) = env::var("LLM_ENDPOINT") {
            if !endpoint.is_empty() {
                config.endpoint_url = Some(endpoint);
                tracing::info!("Using custom LLM endpoint");
            }
        }

        // Check for model override
        if let Ok(model) = env::var("LLM_MODEL") {
            if !model.is_empty() {
                config.model = model;
                tracing::info!("Using custom LLM model: {}", config.model);
            }
        }

        Self { config }
    }
}

fn chunk_file(content: &str, chunk_size: usize, overlap: usize) -> Vec<String> {
    if content.is_empty() || chunk_size == 0 {
        return vec![];
    }

    let mut chunks = Vec::new();
    let mut start = 0;

    while start < content.len() {
        let end = (start + chunk_size).min(content.len());
        chunks.push(content[start..end].to_string());

        if end == content.len() {
            break;
        }

        start = end - overlap;
    }

    chunks
}

fn deduplicate_entities(entities: Vec<DomainEntity>) -> Vec<DomainEntity> {
    let mut unique_entities = HashMap::new();

    for entity in entities {
        unique_entities
            .entry(entity.name.clone())
            .or_insert_with(|| entity);
    }

    unique_entities.into_values().collect()
}

async fn process_llm_prompt(prompt: &str, config: &LlmConfig) -> Result<Vec<DomainEntity>> {
    let llm_response = query_llm(prompt, config).await?;

    let entities_json = serde_json::from_str::<Vec<serde_json::Value>>(&llm_response)
        .map_err(|e| anyhow::anyhow!("Failed to parse LLM response as JSON: {}", e))?;

    let domain_entities = entities_json
        .into_iter()
        .filter_map(parse_entity_from_json)
        .collect::<Vec<_>>();

    if domain_entities.is_empty() {
        return Err(anyhow::anyhow!(
            "No valid domain entities found in LLM response"
        ));
    }

    Ok(domain_entities)
}

impl DomainModelBuilder for LlmModelExtractor {
    async fn extract_domain_model<'a>(
        &'a self,
        content: &'a str,
        file_path: &'a str,
    ) -> Result<Vec<DomainEntity>> {
        let needs_api_key = !matches!(self.config.provider, LlmProvider::Ollama);

        if needs_api_key && self.config.api_key.is_empty() {
            return Err(anyhow::anyhow!(
                "API key not set for provider: {:?}",
                self.config.provider
            ));
        }

        let chunk_size = 10000;
        let overlap = 500;

        if content.len() > chunk_size {
            let chunks = chunk_file(content, chunk_size, overlap);
            let mut all_entities = Vec::new();

            for (i, chunk) in chunks.iter().enumerate() {
                tracing::info!(
                    "Processing chunk {}/{} for {}",
                    i + 1,
                    chunks.len(),
                    file_path
                );
                let prompt = build_domain_extraction_prompt(chunk, file_path);

                match process_llm_prompt(&prompt, &self.config).await {
                    Ok(entities) => all_entities.extend(entities),
                    Err(e) => tracing::warn!("Failed to process chunk {}: {}", i + 1, e),
                }
            }

            if all_entities.is_empty() {
                return Err(anyhow::anyhow!("No entities extracted from file chunks"));
            }

            Ok(deduplicate_entities(all_entities))
        } else {
            let prompt = build_domain_extraction_prompt(content, file_path);
            process_llm_prompt(&prompt, &self.config).await
        }
    }
}

fn parse_entity_from_json(json: serde_json::Value) -> Option<DomainEntity> {
    let name = json["name"].as_str()?.to_string();

    let entity_type =
        json["entity_type"]
            .as_str()
            .map(|entity_type_str| match entity_type_str {
                "Class" => EntityType::Class,
                "Struct" => EntityType::Struct,
                "Enum" => EntityType::Enum,
                "Interface" => EntityType::Interface,
                _ => EntityType::Class,
            })?;

    let description = json["description"].as_str().map(|s| s.to_string());

    let attributes =
        json["attributes"]
            .as_object()
            .map_or_else(std::collections::HashMap::new, |attrs_obj| {
                attrs_obj
                    .iter()
                    .filter_map(|(attr_name, attr_type_value)| {
                        attr_type_value.as_str().map(|attr_type_str| {
                            let attr_type = match attr_type_str {
                                "String" | "string" => AttributeType::String,
                                "Number" | "number" | "int" | "integer" | "float" => {
                                    AttributeType::Number
                                }
                                "Boolean" | "boolean" | "bool" => AttributeType::Boolean,
                                "Date" | "date" => AttributeType::Custom("Date".to_string()),
                                _ => AttributeType::Custom(attr_type_str.to_string()),
                            };
                            (attr_name.clone(), attr_type)
                        })
                    })
                    .collect()
            });

    let relationships = json["relationships"]
        .as_array()
        .map_or_else(Vec::new, |rels_array| {
            rels_array
                .iter()
                .filter_map(|rel| {
                    let target_entity = rel["target_entity"].as_str()?;
                    let relation_type = rel["relation_type"].as_str().map_or(
                        RelationType::References,
                        |rel_type_str| match rel_type_str {
                            "Inherits" => RelationType::Inherits,
                            "Contains" => RelationType::Contains,
                            "References" => RelationType::References,
                            "Implements" => RelationType::Implements,
                            _ => RelationType::References,
                        },
                    );

                    Some(Relationship {
                        target_entity: target_entity.to_string(),
                        relation_type,
                    })
                })
                .collect()
        });

    Some(DomainEntity {
        name,
        entity_type,
        attributes,
        relationships,
        description,
    })
}

/// Build a prompt for domain entity extraction
fn build_domain_extraction_prompt(content: &str, file_path: &str) -> String {
    indoc::formatdoc! {r#"
        Analyze the code from file '{}' and extract business domain entities, their attributes, associated functions, and relationships.

        CODE:
        {}

        Return a JSON array where each item has:
        - "name": Entity name (e.g., "Customer")
        - "entity_type": Entity type (e.g., "Class", "Struct")
        - "attributes": Object mapping attribute names to types (e.g., {{"id": "String", "name": "String"}})
        - "functions": List of [{{"name": "processOrder", "purpose": "Processes an order"}}]
        - "relationships": List of [{{"target_entity": "Order", "relation_type": "Contains"}}]
        - "description": Description of the entity's purpose and role

        Focus on identifying business domain entities that represent real-world concepts.

        For example, given this Python code:

        ```python
        class Customer:
            def __init__(self, id, name, email):
                self.id = id
                self.name = name
                self.email = email
            
            def place_order(self, items):
                return Order(self.id, items)
        
        class Order:
            def __init__(self, customer_id, items):
                self.id = generate_id()
                self.customer_id = customer_id
                self.items = items
                self.total = calculate_total(items)
        ```

        The expected output would be:

        [
            {{
                "name": "Customer",
                "entity_type": "Class",
                "attributes": {{"id": "String", "name": "String", "email": "String"}},
                "functions": [{{"name": "place_order", "purpose": "Creates an order for the customer"}}],
                "relationships": [{{"target_entity": "Order", "relation_type": "Contains"}}],
                "description": "Represents a customer who can place orders"
            }},
            {{
                "name": "Order",
                "entity_type": "Class",
                "attributes": {{"id": "String", "customer_id": "String", "items": "Array", "total": "Number"}},
                "functions": [],
                "relationships": [{{"target_entity": "Customer", "relation_type": "References"}}],
                "description": "Represents an order placed by a customer"
            }}
        ]

        Only provide the JSON array with no other text.
    "#,
    file_path, content
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Unit tests for helper functions
    #[test]
    fn test_chunk_file() {
        // Test with a content of length 20, chunk size 10, and overlap 2
        let content = "12345678901234567890";

        // Manually calculate expected chunks:
        // 1st chunk: indices 0-10  => "1234567890"
        // 2nd chunk: indices 8-18  => "9012345678"
        // 3rd chunk: indices 16-20 => "7890"
        let chunks = chunk_file(content, 10, 2);

        assert_eq!(chunks.len(), 3);
        assert_eq!(chunks[0], "1234567890");
        assert_eq!(chunks[1], "9012345678");
        assert_eq!(chunks[2], "7890");

        // Test with empty content
        let empty_chunks = chunk_file("", 10, 2);
        assert_eq!(empty_chunks.len(), 0);

        // Test with chunk size larger than content
        let large_chunk = chunk_file("12345", 10, 2);
        assert_eq!(large_chunk.len(), 1);
        assert_eq!(large_chunk[0], "12345");
    }

    #[test]
    fn test_deduplicate_entities() {
        let mut entities = Vec::new();

        let entity1 = DomainEntity {
            name: "Customer".to_string(),
            entity_type: EntityType::Class,
            attributes: [("id".to_string(), AttributeType::String)]
                .into_iter()
                .collect(),
            relationships: vec![],
            description: None,
        };

        let entity2 = DomainEntity {
            name: "Customer".to_string(), // Same name as entity1
            entity_type: EntityType::Class,
            attributes: [("name".to_string(), AttributeType::String)]
                .into_iter()
                .collect(),
            relationships: vec![],
            description: Some("Duplicate".to_string()),
        };

        let entity3 = DomainEntity {
            name: "Order".to_string(),
            entity_type: EntityType::Class,
            attributes: [("id".to_string(), AttributeType::String)]
                .into_iter()
                .collect(),
            relationships: vec![],
            description: None,
        };

        entities.push(entity1);
        entities.push(entity2);
        entities.push(entity3);

        let result = deduplicate_entities(entities);
        assert_eq!(result.len(), 2);

        let names: Vec<String> = result.iter().map(|e| e.name.clone()).collect();
        assert!(names.contains(&"Customer".to_string()));
        assert!(names.contains(&"Order".to_string()));
    }

    #[test]
    fn test_build_domain_extraction_prompt() {
        let content = "function test() { return true; }";
        let file_path = "/test/file.js";
        let prompt = build_domain_extraction_prompt(content, file_path);

        assert!(prompt.contains("code from file '/test/file.js'"));
        assert!(prompt.contains("function test() { return true; }"));
        assert!(prompt.contains("Only provide the JSON array"));
    }
}
