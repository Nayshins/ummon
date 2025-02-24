use clap::{Parser, Subcommand};

/// Model Context Protocol (MCP) Server for codebase analysis
#[derive(Parser)]
#[command(author, version, about = "MCP Server for code analysis and impact assessment")]
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
    },
    
    /// Query the knowledge graph using natural language
    Query {
        /// Natural language query (e.g., "show all functions related to user authentication")
        query: String,
    },
    
    /// Start the MCP server
    Serve {
        /// Port number to listen on
        #[arg(default_value = "3000")]
        port: u16,
        
        /// Host address to bind to
        #[arg(default_value = "127.0.0.1")]
        host: String,
    },
    
    /// Analyze the impact of changing a specific file or component
    Analyze {
        /// Target file or component to analyze (e.g., "src/auth/login.rs")
        target: String,
        
        /// Maximum depth for impact analysis
        #[arg(default_value = "3")]
        depth: usize,
    },
    
    /// Map a domain concept to code implementations
    MapDomain {
        /// Domain concept to map (e.g., "Authentication")
        concept: String,
    },
    
    /// Map code to domain concepts
    MapCode {
        /// Path to the code file to map
        path: String,
    },
}
