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
   - Works with multiple languages (Rust, Python, JavaScript, Java)

2. **Advanced Querying System**
   - Query your codebase using a powerful structured query language or natural language
   - Two main query types:
     - Select queries: `select [entity_type] where [conditions]`
     - Traversal queries: `[source_type] [relationship] [target_type] where [conditions]`
   - Natural language translation for user-friendly interaction
   - Rich filtering capabilities with attribute conditions and logical operators
   - Multiple output formats (text, JSON, CSV, tree)
   - Examples: "select functions where name like 'auth%'", "show me all authentication functions"
   - See [Query System Documentation](docs/query_system.md) for more details

3. **Domain Model Extraction**
   - Uses LLMs to identify business entities and concepts
   - Maps domain concepts to implementation details
   - Creates a bridge between technical and business understanding

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

# Query using natural language
ummon query "show all authentication functions"

# Query using structured query language
ummon query "select functions where name like 'auth%'" --no-llm

# Find relationships between entities (traversal query)
ummon query "functions calling functions where name like 'validate%'" --no-llm

# Query with different output formats
ummon query "select functions" --format json
ummon query "select functions" --format csv
ummon query "select functions" --format tree

# Filter query results by type
ummon query "find api" --type-filter function

# Filter by file path pattern
ummon query "show all entities" --path src/auth

# Limit the number of results
ummon query "select functions" --limit 10

# Skip LLM processing for structured queries
ummon query "select functions where file_path like 'src/auth/%'" --no-llm

# Generate AI-assisted recommendations
ummon assist "implement a user registration function"
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

### Language Support

Ummon supports parsing and analysis of multiple programming languages:

- **Rust**: Class/structs, traits, implementations, functions, modules
- **Python**: Classes, functions, decorators, imports
- **JavaScript**: Classes, functions, arrow functions, imports
- **Java**: Classes, interfaces, methods, constructors, fields

The Java parser supports parsing of:
- Class and interface definitions with modifiers
- Constructor declarations
- Method declarations with parameter types
- Field declarations with types
- Package declarations and imports (including wildcard and static imports)
- Documentation comments extraction
- Method calls and relationships

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

### Test Resources

- `test/java/`: Java test files for parser testing
  - `Test.java`: Simple Java class for basic parsing
  - `ComplexExample.java`: Advanced Java features (generics, annotations, etc.)
- `test/javascript/`: JavaScript test files for testing

## License

[APACHE License](LICENSE)
