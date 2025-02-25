# Ummon

Ummon is a code analysis tool that builds knowledge graphs from codebases to enhance understanding, improve AI assistance, and enable sophisticated querying. It creates connections between code entities (functions, classes, modules) and domain concepts, making it easier to reason about complex software systems.

## Core Features

1. **Knowledge Graph Construction**
   - Indexes code to create a semantic representation
   - Maps relationships between code entities (calls, imports, dependencies)
   - Works with multiple languages (Rust, Python, JavaScript)

2. **Natural Language Querying**
   - Query your codebase using plain English
   - Find entities, relationships, and domain concepts
   - Examples: "Show functions that use the database", "What calls the authenticate method?"

3. **Domain Model Extraction**
   - Uses LLMs to identify business entities and concepts
   - Maps domain concepts to implementation details
   - Creates a bridge between technical and business understanding

4. **Impact Analysis**
   - Assess how changes might affect other parts of the codebase
   - Identify ripple effects before making changes
   - Reduce the risk of unexpected regressions

## Installation and Setup

```
cargo install ummon
```

## Usage

```bash
# Index a codebase
ummon index /path/to/codebase

# Query the knowledge graph
ummon query "show all authentication functions"

# Start the server
ummon serve

# Generate AI-assisted recommendations
ummon assist "implement a user registration function"
```

## Configuration

Ummon can be configured through environment variables:

- `OPENROUTER_API_KEY`: API key for LLM services
- `USE_LLM_MODEL_EXTRACTION`: Enable domain model extraction
- `DOMAIN_EXTRACTION_DIR`: Directory to analyze for domain concepts

## Architecture

Ummon is built with a modular architecture:
- Language-specific parsers for code analysis
- Graph-based storage for entities and relationships
- LLM integration for semantic understanding
- Command-line interface for user interaction
- Server component for API access

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
