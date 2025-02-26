use clap::{Parser, Subcommand};

/// Ummon: A code analysis tool that builds knowledge graphs from codebases
#[derive(Parser)]
#[command(author, version, about = "A code analysis tool that builds knowledge graphs from codebases")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Build or update the knowledge graph from the codebase
    Index {
        /// Path to the directory to index (defaults to current directory)
        #[arg(default_value = ".")]
        path: String,
        
        /// Enable domain model extraction using LLM
        #[arg(long, short = 'e')]
        enable_domain_extraction: bool,
        
        /// Directory to analyze for domain extraction (defaults to src/)
        #[arg(long, default_value = "src")]
        domain_dir: String,
    },
    
    /// Query the knowledge graph using natural language
    Query {
        /// Natural language query (e.g., "show all functions related to user authentication")
        query: String,
        
        /// Output format (text, json)
        #[arg(long, short, default_value = "text")]
        format: String,
    },
    
    /// Generate AI-assisted recommendations
    Assist {
        /// User instruction (e.g., "implement a user registration function")
        instruction: String,
    },
}
