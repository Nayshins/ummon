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

## Pre-Commit Checklist
- **ALWAYS** run `cargo fmt` before committing code
- Run `cargo clippy` to check for code improvements
- Ensure all tests pass with `cargo test`

## Test Structure
Tests have been set up as inline modules within each source file using Rust's `#[cfg(test)]` attribute.


## Code Style Guidelines
- **Imports**: Group standard library imports first, followed by external crates, then local modules
- **Error Handling**: Use `anyhow::Result` for functions that can fail, with `?` operator for propagation
- **Naming**: Use snake_case for variables and functions, CamelCase for types
- **Types**: Prefer strong typing with descriptive type names
- **Documentation**: Include doc comments for public API functions
- **Formatting**: Follow Rust style guidelines with 4-space indentation
- **Modules**: Organize code in modules by functionality (parser, graph, commands)
- **Warnings**: Do not check in new warnings
- **Dead Code**: Eliminate unused functions, variables, and imports. Dead code should only be retained when necessary for compilation purposes.

## Comment Structure
- **Public API**: Use triple-slash `///` doc comments for all public functions, structs, and methods
- **Implementation**: Use double-slash `//` for inline comments explaining complex logic
- **TODOs**: Format as `// TODO: description` with a clear action item
- **Module Headers**: Add a brief description at the top of each file explaining its purpose
- **Sections**: Use comment blocks `// ---- SECTION NAME ----` to separate logical sections within large files
- **Reasoning**: Include rationale for non-obvious implementation choices
