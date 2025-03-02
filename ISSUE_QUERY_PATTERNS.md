# Revisit Query Patterns and Interface

## Issue Description

The current query implementation offers a tiered approach with direct KG queries, pattern-based queries, and LLM fallback. While this implementation is a good start, we should revisit the query patterns to make them more comprehensive and consistent.

## Goals

1. **Expand Pattern Recognition**:
   - Add support for more common query patterns
   - Improve regex patterns to better capture user intent
   - Create a more structured way to define and expand patterns

2. **Query Optimization**:
   - Review the filtering mechanism to improve performance
   - Consider caching common query results
   - Evaluate more efficient query execution strategies

3. **Advanced Filtering**:
   - Add support for nested filtering (AND/OR conditions)
   - Support filtering on entity metadata and other attributes
   - Improve full-text search capabilities

4. **Standardize Query Language**:
   - Establish a consistent grammar for complex queries
   - Document the query language for easier adoption
   - Consider a small DSL (Domain-Specific Language) for complex queries

5. **Improved Result Formatting**:
   - Add support for more output formats (e.g., CSV, tree view)
   - Enhance JSON output with more contextual information
   - Add summary statistics for large result sets

## Implementation Ideas

- Create a pattern registry that allows for easy addition of new patterns
- Use a proper parser combinator library for more complex query parsing
- Consider a rule-based system for entity matching and filtering
- Add advanced relevancy ranking for search results
- Implement a more sophisticated query planner for complex queries

## Related Changes

- Tiered query architecture with direct KG queries, pattern-based queries, and LLM fallback
- Added grep-like filtering options (--type-filter, --path, --exact, --limit, --no-llm)
- Improved query performance by avoiding unnecessary LLM calls

## Timeline

This is a long-term task that should be revisited after the core functionality is stable, likely as part of a 1.x release.