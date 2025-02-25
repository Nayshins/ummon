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
pub struct LlmDomainModelBuilder {}

impl DomainModelBuilder for LlmDomainModelBuilder {
    fn extract_domain_model(&self, _content: &str, _file_path: &str) -> Result<Vec<DomainEntity>> {
        // This implementation will be async in the actual code
        // Here we're just defining the interface
        Ok(Vec::new())
    }
}
