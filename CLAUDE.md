# Ummon Development Guide

## Build & Test Commands
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

## Code Style Guidelines
- **Imports**: Group standard library imports first, followed by external crates, then local modules
- **Error Handling**: Use `anyhow::Result` for functions that can fail, with `?` operator for propagation
- **Naming**: Use snake_case for variables and functions, CamelCase for types
- **Types**: Prefer strong typing with descriptive type names
- **Documentation**: Include doc comments for public API functions
- **Formatting**: Follow Rust style guidelines with 4-space indentation
- **Modules**: Organize code in modules by functionality (parser, graph, commands)