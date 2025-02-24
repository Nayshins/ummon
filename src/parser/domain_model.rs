use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainEntity {
    pub name: String,
    pub entity_type: EntityType,
    pub attributes: HashMap<String, AttributeType>,
    pub relationships: Vec<Relationship>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EntityType {
    Class,
    Struct,
    Enum,
    Interface,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AttributeType {
    String,
    Number,
    Boolean,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relationship {
    pub relation_type: RelationType,
    pub target_entity: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RelationType {
    Inherits,
    Contains,
    References,
    Implements,
}

pub trait DomainModelBuilder {
    fn extract_domain_model(&self, content: &str, file_path: &str) -> Result<Vec<DomainEntity>>;
}

/// Builds a domain model using an LLM
pub struct LlmDomainModelBuilder {
    pub api_key: String,
}

impl LlmDomainModelBuilder {
    pub fn new(api_key: String) -> Self {
        Self { api_key }
    }
}

impl DomainModelBuilder for LlmDomainModelBuilder {
    fn extract_domain_model(&self, content: &str, file_path: &str) -> Result<Vec<DomainEntity>> {
        // This implementation will be async in the actual code
        // Here we're just defining the interface
        Ok(Vec::new())
    }
}

/// Extracts domain entities from LLM response text
pub fn parse_domain_entities_from_llm_response(response: &str) -> Result<Vec<DomainEntity>> {
    #[derive(Debug, Deserialize)]
    struct LlmResponseEntity {
        name: String,
        entity_type: String,
        description: Option<String>,
        attributes: HashMap<String, String>,
        relationships: Vec<LlmRelationship>,
    }

    #[derive(Debug, Deserialize)]
    struct LlmRelationship {
        target_entity: String,
        relation_type: String,
    }

    // Try to extract just the JSON part if there's surrounding text
    let json_content = extract_json_from_response(response);
    
    // Parse the JSON
    let llm_entities: Vec<LlmResponseEntity> = serde_json::from_str(&json_content)?;
    
    // Convert to our domain entities
    let domain_entities = llm_entities
        .into_iter()
        .map(|e| {
            let entity_type = match e.entity_type.as_str() {
                "Class" => EntityType::Class,
                "Struct" => EntityType::Struct,
                "Enum" => EntityType::Enum,
                "Interface" => EntityType::Interface,
                _ => EntityType::Class, // Default
            };
            
            let attributes = e.attributes
                .into_iter()
                .map(|(name, type_str)| {
                    let attr_type = match type_str.to_lowercase().as_str() {
                        "string" => AttributeType::String,
                        "number" | "int" | "integer" | "float" => AttributeType::Number,
                        "boolean" | "bool" => AttributeType::Boolean,
                        _ => AttributeType::Custom(type_str),
                    };
                    (name, attr_type)
                })
                .collect();
                
            let relationships = e.relationships
                .into_iter()
                .map(|r| {
                    let relation_type = match r.relation_type.as_str() {
                        "Inherits" => RelationType::Inherits,
                        "Contains" => RelationType::Contains,
                        "References" => RelationType::References,
                        "Implements" => RelationType::Implements,
                        _ => RelationType::References, // Default
                    };
                    
                    Relationship {
                        relation_type,
                        target_entity: r.target_entity,
                    }
                })
                .collect();
                
            DomainEntity {
                name: e.name,
                entity_type,
                attributes,
                relationships,
                description: e.description,
            }
        })
        .collect();
        
    Ok(domain_entities)
}

/// Extract just the JSON part from the LLM response 
/// (in case the LLM added explanatory text)
fn extract_json_from_response(response: &str) -> String {
    // Simple heuristic: look for content between [ and ]
    if let (Some(start), Some(end)) = (response.find('['), response.rfind(']')) {
        if start < end {
            return response[start..=end].to_string();
        }
    }
    
    // Try to find JSON objects if no array is present
    if let (Some(start), Some(end)) = (response.find('{'), response.rfind('}')) {
        if start < end {
            format!("[{}]", &response[start..=end])
        } else {
            response.to_string()
        }
    } else {
        response.to_string()
    }
}
