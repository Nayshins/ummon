```
██╗   ██╗███╗   ███╗███╗   ███╗ ██████╗ ███╗   ██╗
██║   ██║████╗ ████║████╗ ████║██╔═══██╗████╗  ██║
██║   ██║██╔████╔██║██╔████╔██║██║   ██║██╔██╗ ██║
██║   ██║██║╚██╔╝██║██║╚██╔╝██║██║   ██║██║╚██╗██║
╚██████╔╝██║ ╚═╝ ██║██║ ╚═╝ ██║╚██████╔╝██║ ╚████║
 ╚═════╝ ╚═╝     ╚═╝╚═╝     ╚═╝ ╚═════╝ ╚═╝  ╚═══╝

"WHAT ARE THE ACTIVITIES OF A SYSTEM?
 I HAVE NOT THE SLIGHTEST IDEA.
 THE PATH APPEARS"
```

> **⚠️ WARNING: This project is in early development and is not yet stable. APIs and functionality may change significantly between versions.**

Ummon is a code analysis tool that builds knowledge graphs from codebases to enhance understanding, improve AI assistance, and enable sophisticated querying. It creates connections between code entities (functions, classes, modules) and domain concepts, making it easier to reason about complex software systems.

Named after the AI Ummon from Dan Simmons' Hyperion Cantos, this project provides deep insights into codebases that help both humans and AI assistants better understand software systems.

## Core Features

1. **Knowledge Graph Construction**
   - Indexes code to create a semantic representation
   - Maps relationships between code entities (calls, imports, dependencies)
   - Works with multiple languages (Rust, Python, JavaScript)

2. **Advanced Querying System**
   - Query your codebase using natural language or precise grep-like flags
   - Find entities, relationships, and domain concepts with tiered processing:
     - Direct knowledge graph queries for common patterns
     - Pattern-based queries for structured requests
     - LLM-powered analysis for complex semantic questions
   - Efficient filtering with type, path, and exact matching options
   - Examples: "Show functions that use the database", "find api --type-filter function --path src/auth"

3. **Domain Model Extraction**
   - Uses LLMs to identify business entities and concepts
   - Maps domain concepts to implementation details
   - Creates a bridge between technical and business understanding

4. **AI Agent Integration with MCP**
   - Provides Model Context Protocol (MCP) server for AI agent interaction
   - Enables agents to explore code relationships and architecture
   - Advanced visualization tools for navigating complex codebases
   - Find relevant files for specific tasks or features

## Installation and Setup

```
cargo install ummon
```

## Usage

```bash
# Index a codebase
ummon index /path/to/codebase

# Index with domain model extraction enabled
ummon index /path/to/codebase --enable-domain-extraction

# Specify a custom domain directory for extraction
ummon index /path/to/codebase --enable-domain-extraction --domain-dir models/

# Query the knowledge graph
ummon query "show all authentication functions"

# Query with JSON output
ummon query "show all authentication functions" --format json

# Filter query results by entity type
ummon query "find api" --type-filter function

# Filter by file path pattern
ummon query "show all entities" --path src/auth

# Limit the number of results
ummon query "list functions" --limit 10

# Use exact matching only (no partial matches)
ummon query "find user" --exact

# Skip LLM processing for faster results
ummon query "show authentication logic" --no-llm

# Generate AI-assisted recommendations
ummon assist "implement a user registration function"

# Start an MCP server (Model Context Protocol) for AI agent interaction
ummon serve                      # Use stdin/stdout (default)
ummon serve --transport http     # Use HTTP server (requires feature flag)
```

## MCP Server

Ummon includes a Model Context Protocol (MCP) server that allows AI agents to interact with codebase knowledge graphs. The server provides these capabilities:

### Available Tools:

- `search_code`: Search for code entities using a natural language query
- `get_entity`: Get detailed information about a specific entity

### Available Resources:

- `knowledge_graph.json`: The full knowledge graph in JSON format

### Example MCP Usage:

```bash
# Start MCP server with stdin/stdout transport (works with MCP Inspector)
ummon serve

# Connect to the server using MCP Inspector
npx @modelcontextprotocol/inspector ummon serve
```

## Configuration

Ummon uses environment variables only for sensitive information:

- `OPENROUTER_API_KEY`: API key for LLM services (required for queries and domain extraction)

All other configuration is handled through command-line flags.

## Architecture

Ummon is built with a modular architecture:
- Language-specific parsers for code analysis
- Graph-based storage for entities and relationships
- LLM integration for semantic understanding
- Command-line interface for user interaction
- MCP server for AI agent integration

## Development

### Build & Test Commands
```bash
# Build the project
cargo build

# Run the project
cargo run

# Run with specific command
cargo run -- index .      # Index current directory
cargo run -- query "show funcs"  # Query the knowledge graph

# Run tests
cargo test
cargo test -- --nocapture  # Show test output
cargo test <test_name>     # Run specific test

# Format code
cargo fmt
```

## License

[APACHE License](LICENSE)
