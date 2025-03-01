use clap::{Parser, Subcommand, ValueEnum};

/// Ummon: A code analysis tool that builds knowledge graphs from codebases
#[derive(Parser)]
#[command(
    author,
    version,
    about = "A code analysis tool that builds knowledge graphs from codebases"
)]
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

        /// Filter results by type (function, method, class, etc.)
        #[arg(long, short)]
        type_filter: Option<String>,

        /// Filter results by file path pattern
        #[arg(long, short = 'p')]
        path: Option<String>,

        /// Include exact ID matches only
        #[arg(long, short)]
        exact: bool,

        /// Maximum number of results to return
        #[arg(long, short, default_value = "20")]
        limit: usize,

        /// Skip LLM and only use direct knowledge graph queries
        #[arg(long)]
        no_llm: bool,
    },

    /// Generate AI-assisted recommendations
    Assist {
        /// User instruction (e.g., "implement a user registration function")
        instruction: String,
    },

    /// Start an MCP server for AI agent interaction
    Serve {
        /// Transport type to use for the MCP server
        #[arg(long, short, default_value = "stdin-stdout")]
        transport: TransportType,
    },
}

#[derive(Clone, Debug, ValueEnum)]
pub enum TransportType {
    /// Use stdin/stdout for communication (for CLI tool integration)
    StdinStdout,

    /// Use HTTP server (for networked integration)
    Http,
}

impl std::fmt::Display for TransportType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TransportType::StdinStdout => write!(f, "stdin-stdout"),
            TransportType::Http => write!(f, "http"),
        }
    }
}
