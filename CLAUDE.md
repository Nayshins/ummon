# Ummon Development Guide

*IMPORTANT* Make sure to follow these rules when writing code in this repository

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
- **ALWAYS** fix any Clippy warnings before committing
- Ensure all tests pass with `cargo test`

## Test Structure
Tests have been set up as inline modules within each source file using Rust's `#[cfg(test)]` attribute.

## Test Principles
- **CRITICAL**: When a test fails, NEVER change the test assertions to match incorrect implementation. Always:
  1. Verify that the test assertions represent the correct expected behavior
  2. If the test is correct, fix the implementation to match the expected behavior
  3. Only modify test assertions if the test itself is incorrect (not to match buggy code)
- Tests should be independent and not rely on external state or environment variables
- Mock external dependencies rather than making real API calls
- Each test should focus on a specific behavior or feature


## Code Style Guidelines
- **Imports**: Group standard library imports first, followed by external crates, then local modules
- **Error Handling**: Use `anyhow::Result` for functions that can fail, with `?` operator for propagation
- **Naming**: Use snake_case for variables and functions, CamelCase for types
- **Types**: Prefer strong typing with descriptive type names
- **Documentation**: Include doc comments for public API functions
- **Formatting**: Follow Rust style guidelines with 4-space indentation
- **Modules**: Organize code in modules by functionality (parser, graph, commands)
- **Warnings**: Do not check in new warnings
- **Dead Code**: Eliminate unused functions, variables, and imports. There are no backward compatibility requirements at this time.
- **Error Handling Pattern**: Use `Result` types with context for error handling rather than unwrapping. Prefer `map_or` over `unwrap_or` patterns.
- **Functional Style**: Prefer functional chains with methods like `filter`, `map`, and `and_then` over deeply nested if-let statements. Example:
  ```rust
  // Prefer this:
  let result = some_option
      .filter(|x| condition(x))
      .and_then(|x| transform(x))
      .map(|x| x.to_string());
      
  // Over this:
  let mut result = None;
  if let Some(x) = some_option {
      if condition(x) {
          if let Some(y) = transform(x) {
              result = Some(y.to_string());
          }
      }
  }
  ```
- **Code Completion**: Always complete implementation for all code paths. Do not add placeholder code or TODO comments unless specifically instructed to. Complete all functionality according to requirements before committing code.

## Comment Structure
- **Public API**: Use triple-slash `///` doc comments for all public functions, structs, and methods
- **Implementation**: Use double-slash `//` for inline comments ONLY for complex/non-obvious logic
- **TODOs**: Format as `// TODO: description` with a clear action item
- **Module Headers**: Add a brief description at the top of each file explaining its purpose
- **Sections**: Use comment blocks `// ---- SECTION NAME ----` to separate logical sections within large files
- **Reasoning**: Include rationale for non-obvious implementation choices
- **Minimalism**: Avoid adding comments for straightforward operations or self-explanatory code
- **IMPORTANT**: Do NOT add inline comments that merely restate what the code already shows (e.g., "// Iterate through users" for a loop over users)
- **CRITICAL**: Never use redundant or descriptive inline comments that explain the obvious function of the code
- Let's be aware of the code standards set in this guide. Do not add redundant inline comments and focus on a functional style.
