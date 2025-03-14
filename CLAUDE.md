# Ummon Development Guide

## Issue #37 Implementation Status
We've implemented the following steps for "Parsing Improvements":
1. ✅ Completed (earlier)
2. ✅ Completed (earlier)
3. ✅ Completed (earlier)
4. ✅ Improve parser robustness
   - Added empty content validation in all parsers
   - Added bounds checking for string operations to prevent panics
   - Improved error messages with better context (filename, file size)
   - Added tracing/logging for better diagnostics
5. ✅ Standardize parser interface
   - Created well-documented consistent interface for all language parsers
   - Enhanced CallReference with more context (location, file path, arguments)
   - Added shared helper methods for common operations (safe_extract_text)
   - Added default implementations for optional interfaces
6. ✅ Test enhanced parsers
   - Fixed Clippy warnings in code (improved functional style)
   - Removed unused code (dead code elimination)
   - Enhanced code style with better patterns (if let vs map for side effects)
   - Argument extraction tested in all parsers

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
