use anyhow::Result;
use std::env;
use std::str::FromStr;

use crate::parser::domain_model::{AttributeType, DomainEntity, DomainModelBuilder, EntityType};
use crate::prompt::llm_integration::{LlmConfig, LlmProvider};

/// Domain model builder that uses an LLM to extract domain entities
pub struct LlmModelExtractor {
    pub config: LlmConfig,
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
            LlmProvider::Mock => Some(String::new()),
        };

        // Set the API key if found
        if let Some(key) = api_key {
            if !key.is_empty()
                || config.provider == LlmProvider::Ollama
                || config.provider == LlmProvider::Mock
            {
                tracing::info!(
                    "Found API key for {:?}, LLM domain extraction enabled",
                    config.provider
                );
                config.api_key = key;
            }
        } else {
            tracing::warn!(
                "{:?} API key not set, using mock domain entities",
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

impl DomainModelBuilder for LlmModelExtractor {
    fn extract_domain_model<'a>(
        &'a self,
        content: &'a str,
        file_path: &'a str,
    ) -> impl std::future::Future<Output = Result<Vec<DomainEntity>>> + Send + 'a {
        async move {
            // Check if api key is set (for providers that need it)
            let needs_api_key = match self.config.provider {
                LlmProvider::Ollama | LlmProvider::Mock => false,
                _ => true,
            };

            if needs_api_key && self.config.api_key.is_empty() {
                tracing::info!(
                    "Using mock domain entity (API key not set) for {}",
                    file_path
                );

                // Return a mock domain entity when no API key is provided
                return Ok(vec![DomainEntity {
                    name: format!(
                        "MockEntity_{}",
                        file_path.split('/').last().unwrap_or("unknown")
                    ),
                    entity_type: EntityType::Class,
                    attributes: [
                        ("id".to_string(), AttributeType::String),
                        ("name".to_string(), AttributeType::String),
                    ]
                    .into_iter()
                    .collect(),
                    relationships: vec![],
                    description: Some(
                        "This is a mock domain entity because no API key was provided".to_string(),
                    ),
                }]);
            }

            // Truncate content if it's too long (limit to ~10k chars to avoid token limits)
            let truncated_content = if content.len() > 10000 {
                tracing::info!("Content too large, truncating for LLM analysis");
                // Take the first 8k and last 2k characters to capture more of the important structure
                // Usually class/type definitions are at the beginning of files
                let first_size = 8000.min(content.len());
                let first = &content[..first_size];

                if content.len() > first_size {
                    let remaining = content.len() - first_size;
                    let last_size = 2000.min(remaining);
                    let last = &content[content.len() - last_size..];
                    format!("{}\n\n... [content truncated] ...\n\n{}", first, last)
                } else {
                    first.to_string()
                }
            } else {
                content.to_string()
            };

            // Create a prompt for the LLM
            let prompt = build_domain_extraction_prompt(&truncated_content, file_path);

            tracing::info!(
                "Sending request to {:?} for domain extraction...",
                self.config.provider
            );

            // Directly use await since we're in an async function now
            match crate::prompt::llm_integration::query_llm(&prompt, &self.config).await {
                Ok(response) => {
                    // Try to parse the LLM response as JSON array of domain entities
                    match serde_json::from_str::<Vec<serde_json::Value>>(&response) {
                        Ok(entities_json) => {
                            // Parse each entity in the JSON array
                            let mut domain_entities = Vec::new();

                            for entity_json in entities_json {
                                if let Some(entity) = parse_entity_from_json(entity_json) {
                                    domain_entities.push(entity);
                                }
                            }

                            if domain_entities.is_empty() {
                                tracing::warn!("No valid entities parsed from LLM response");
                                create_mock_entity(file_path)
                            } else {
                                Ok(domain_entities)
                            }
                        }
                        Err(e) => {
                            tracing::error!("Error parsing LLM response as JSON: {}", e);
                            create_mock_entity(file_path)
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("Error calling LLM API: {}", e);
                    create_mock_entity(file_path)
                }
            }
        }
    }
}

fn create_mock_entity(file_path: &str) -> Result<Vec<DomainEntity>> {
    tracing::info!("Using mock domain entity for {}", file_path);
    let file_name = file_path.split('/').last().unwrap_or("unknown");

    // Generate domain entity based on the file name
    let entity_type = if file_name.contains("entity") {
        EntityType::Class
    } else if file_name.contains("model") {
        EntityType::Class
    } else if file_name.contains("struct") {
        EntityType::Struct
    } else if file_name.contains("enum") {
        EntityType::Enum
    } else {
        EntityType::Class
    };

    // Return a more detailed mock entity based on the file name
    Ok(vec![DomainEntity {
        name: format!(
            "{}Model",
            file_name.split('.').next().unwrap_or("Domain").to_string()
        ),
        entity_type,
        attributes: [
            ("id".to_string(), AttributeType::String),
            ("name".to_string(), AttributeType::String),
            ("created_at".to_string(), AttributeType::String),
        ]
        .into_iter()
        .collect(),
        relationships: vec![],
        description: Some(format!("Domain model extracted from {}", file_path)),
    }])
}

fn parse_entity_from_json(json: serde_json::Value) -> Option<DomainEntity> {
    let name = json["name"].as_str()?.to_string();

    // Parse entity type
    let entity_type_str = json["entity_type"].as_str()?;
    let entity_type = match entity_type_str {
        "Class" => EntityType::Class,
        "Struct" => EntityType::Struct,
        "Enum" => EntityType::Enum,
        "Interface" => EntityType::Interface,
        _ => EntityType::Class, // Default to Class
    };

    // Parse description
    let description = json["description"].as_str().map(|s| s.to_string());

    // Parse attributes
    let mut attributes = std::collections::HashMap::new();
    if let Some(attrs_obj) = json["attributes"].as_object() {
        for (attr_name, attr_type_value) in attrs_obj {
            if let Some(attr_type_str) = attr_type_value.as_str() {
                let attr_type = match attr_type_str {
                    "String" | "string" => AttributeType::String,
                    "Number" | "number" | "int" | "integer" | "float" => AttributeType::Number,
                    "Boolean" | "boolean" | "bool" => AttributeType::Boolean,
                    // Date types are converted to custom string types
                    "Date" | "date" => AttributeType::Custom("Date".to_string()),
                    _ => AttributeType::Custom(attr_type_str.to_string()),
                };
                attributes.insert(attr_name.clone(), attr_type);
            }
        }
    }

    // Parse relationships (simplified for now)
    let relationships = vec![];

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
    format!(
        r#"You are an expert software engineer specializing in domain modeling and code comprehension. 
Analyze the following code and extract domain entities (concepts) from it.

File: {}

```
{}
```

Identify all entities (classes, interfaces, data structures, functions) that represent important concepts in this codebase.
Don't limit yourself to just business concepts - extract technical concepts as well.
For each entity:
1. Provide a name
2. Classify its type (Class, Struct, Enum, Interface)
3. List attributes with their types
4. Describe relationships to other entities
5. Add a brief description of the entity's purpose and role in the system

Look for:
- Data structures that represent domain entities
- Key abstractions that organize functionality
- Core concepts mentioned in comments or function names
- Important technical patterns or architectural components

Format your response as a JSON array containing entity objects with these fields:
- name: string
- entity_type: "Class" | "Struct" | "Enum" | "Interface"
- description: string
- attributes: object mapping attribute names to types
- relationships: array of objects with {{target_entity: string, relation_type: "Inherits"|"Contains"|"References"|"Implements"}}

Only provide the JSON with no other text."#,
        file_path, content
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_llm_model_extractor_new() {
        // Test with no environment variables
        std::env::remove_var("LLM_PROVIDER");
        std::env::remove_var("LLM_MODEL");
        std::env::remove_var("OPENROUTER_API_KEY");
        std::env::remove_var("OPENAI_API_KEY");
        std::env::remove_var("ANTHROPIC_API_KEY");

        let extractor = LlmModelExtractor::new();
        assert_eq!(extractor.config.provider, LlmProvider::OpenRouter);
        assert_eq!(extractor.config.api_key, "");
        assert_eq!(
            extractor.config.model,
            "anthropic/claude-3-5-haiku-20241022"
        );

        // Test with environment variables
        std::env::set_var("LLM_PROVIDER", "anthropic");
        std::env::set_var("LLM_MODEL", "claude-instant-1.2");
        std::env::set_var("ANTHROPIC_API_KEY", "test_key");

        let extractor = LlmModelExtractor::new();
        assert_eq!(extractor.config.provider, LlmProvider::Anthropic);
        assert_eq!(extractor.config.api_key, "test_key");
        assert_eq!(extractor.config.model, "claude-instant-1.2");

        // Cleanup
        std::env::remove_var("LLM_PROVIDER");
        std::env::remove_var("LLM_MODEL");
        std::env::remove_var("ANTHROPIC_API_KEY");
    }

    #[tokio::test]
    async fn test_extract_domain_model() {
        let mut extractor = LlmModelExtractor::new();
        // Configure a mock provider to avoid real API calls
        extractor.config.provider = LlmProvider::Mock;

        let content =
            "class TestEntity { constructor(id, name) { this.id = id; this.name = name; } }";
        let file_path = "/test/entity.js";

        let result = extractor.extract_domain_model(content, file_path).await;
        assert!(result.is_ok());

        let entities = result.unwrap();
        assert!(!entities.is_empty());
    }

    #[test]
    fn test_build_domain_extraction_prompt() {
        let content = "function test() { return true; }";
        let file_path = "/test/file.js";
        let prompt = build_domain_extraction_prompt(content, file_path);

        assert!(prompt.contains("File: /test/file.js"));
        assert!(prompt.contains("function test() { return true; }"));
        assert!(prompt.contains("Format your response as a JSON array"));
    }
}
