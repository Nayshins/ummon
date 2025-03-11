// Expose modules as public for use by other crates
pub mod db;
pub mod graph;
pub mod mcp_core;
pub mod mcp_server;
pub mod parser;
pub mod prompt;

// Re-export core types for convenience
pub use graph::entity;
pub use graph::knowledge_graph::KnowledgeGraph;
pub use graph::relationship;
pub use parser::language_support;