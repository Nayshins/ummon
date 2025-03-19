mod executor;
mod formatter;
mod nl_translator;
mod parser;

pub use executor::QueryExecutor;
pub use executor::{OutputFormat, TraversalDirection};
pub use nl_translator::translate;
pub use parser::parse_query;

use anyhow::Result;

// Re-export query types
pub use parser::{
    ConditionNode, EntityTypeSelector, Operator, QueryType, Relationship, SelectQuery,
    TraversalQuery, Value,
};

/// A simplified module that directly uses SQLite for queries
/// instead of loading the entire graph into memory

/// For compatibility with existing code
pub struct QueryOptions {
    pub format: String,
    pub natural: bool,
    pub llm_provider: Option<String>,
    pub llm_model: Option<String>,
    pub limit: usize,
}

impl Default for QueryOptions {
    fn default() -> Self {
        Self {
            format: "text".to_string(),
            natural: false,
            llm_provider: None,
            llm_model: None,
            limit: 100,
        }
    }
}
