use anyhow::Result;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct DomainEntity {
    pub name: String,
    pub entity_type: EntityType,
    pub attributes: HashMap<String, AttributeType>,
    pub relationships: Vec<Relationship>,
}

#[derive(Debug, Clone)]
pub enum EntityType {
    Class,
    Struct,
    Enum,
    Interface,
}

#[derive(Debug, Clone)]
pub enum AttributeType {
    String,
    Number,
    Boolean,
    Custom(String),
}

#[derive(Debug, Clone)]
pub struct Relationship {
    pub relation_type: RelationType,
    pub target_entity: String,
}

#[derive(Debug, Clone)]
pub enum RelationType {
    Inherits,
    Contains,
    References,
    Implements,
}

pub trait DomainModelBuilder {
    fn extract_domain_model(&self, content: &str) -> Result<Vec<DomainEntity>>;
}
