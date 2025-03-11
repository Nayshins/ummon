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

        /// LLM provider to use for domain extraction
        #[arg(long, value_enum, default_value = "openrouter")]
        llm_provider: Option<String>,

        /// LLM model to use
        #[arg(long)]
        llm_model: Option<String>,
    },

    /// Query the knowledge graph using Ummon's query language or natural language
    ///
    /// Supports two query types:
    /// 1. Select queries: "select [entity_type] where [conditions]"
    /// 2. Traversal queries: "[source_type] [relationship] [target_type] where [conditions]"
    ///
    /// Entity types: functions, methods, classes, modules, variables, constants, domain_concepts
    /// Relationships: calls, contains, imports, inherits, implements, references, uses, depends_on
    ///
    /// Examples:
    ///   - "select functions where name like 'auth%'"
    ///   - "functions calling functions where name like 'validate%'"
    ///   - "classes containing methods where name like 'get%'"
    ///   - Or in natural language: "show me authentication functions"
    Query {
        /// Query string in either structured syntax or natural language
        /// For structured syntax, use: "select [entity_type] where [conditions]"
        /// For natural language, use regular English: "show me all authentication functions"
        query: String,

        /// Output format for results
        #[arg(long, short, default_value = "text", value_parser=["text", "json", "csv", "tree"])]
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
        /// Use this when you want to use the structured query syntax directly
        #[arg(long)]
        no_llm: bool,

        /// LLM provider to use for natural language query translation
        #[arg(long, value_enum, default_value = "openrouter")]
        llm_provider: Option<String>,

        /// LLM model to use for natural language query translation
        #[arg(long)]
        llm_model: Option<String>,
    },

    /// Generate AI-assisted recommendations
    Assist {
        /// User instruction (e.g., "implement a user registration function")
        instruction: String,

        /// LLM provider to use for assistance
        #[arg(long, value_enum, default_value = "openrouter")]
        llm_provider: Option<String>,

        /// LLM model to use
        #[arg(long)]
        llm_model: Option<String>,
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
