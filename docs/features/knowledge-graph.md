---
title: Knowledge Graph
description: Understanding Ummon's codebase representation
---

# Knowledge Graph

The knowledge graph is the core of Ummon's code understanding capabilities. It creates a structured representation of your codebase, capturing entities, relationships, and metadata that power all of Ummon's features.

## Overview

Ummon's knowledge graph is a semantic network that models your codebase as:

- **Entities**: Code elements like functions, classes, methods, and modules
- **Relationships**: Connections between entities (calls, contains, inherits, etc.)
- **Attributes**: Properties of entities (name, file path, documentation, etc.)

This structure enables advanced querying, reasoning about code, and context-aware assistance.

## Entity Types

The knowledge graph captures various entity types from your code:

| Entity Type | Description | Example |
|-------------|-------------|--------|
| Function | Standalone functions | `calculate_tax()` |
| Method | Class/object methods | `User.validate()` |
| Class | Classes, structs, interfaces | `UserAccount` |
| Module | Files or namespaces | `authentication.rs` |
| Variable | Variables and fields | `max_retries` |
| Constant | Constant values | `MAX_CONNECTIONS` |
| Domain Concept | Business/domain concepts | `PaymentProcessing` |

## Relationship Types

Relationships capture how entities connect to each other:

| Relationship | Description | Example |
|--------------|-------------|--------|
| Calls | Function/method invocations | `validate()` calls `check_password()` |
| Contains | Hierarchical containment | Class contains methods |
| Imports | Module or file imports | File imports a library |
| Inherits | Class inheritance | `AdminUser` inherits from `User` |
| Implements | Interface implementation | `FileStorage` implements `Storage` |
| References | Entity references another | Function references a variable |
| Represented By | Domain concept in code | `Authentication` represented by auth functions |

## Graph Construction

Ummon builds the knowledge graph through several steps:

1. **Parsing**: Language-specific parsers analyze code files
2. **Entity Extraction**: Identify code elements and their attributes
3. **Relationship Analysis**: Determine how entities relate to each other
4. **Domain Mapping**: (Optional) Connect domain concepts to implementation
5. **Metadata Tracking**: Record file timestamps for incremental updates

## Graph Storage

The knowledge graph is stored using a combination of:

- **SQLite Database**: Efficient storage and retrieval of entities and relationships
- **Metadata Tables**: Tracking file modifications for incremental updates

## Update Mechanisms

Ummon provides two approaches for updating the knowledge graph:

### Incremental Updates (Default)

When run without the `--full` flag, Ummon performs incremental updates:
- Detects modified files since the last index using file modification times
- Removes entities and relationships from modified files only
- Reindexes only the modified files
- Preserves the rest of the graph

This approach is significantly faster for large codebases when only a few files have changed.

### Full Rebuilds

When run with the `--full` flag, Ummon performs a complete rebuild:
- Purges all entities and relationships
- Reindexes the entire codebase from scratch

This is useful after major changes or when you want to ensure graph consistency.

## Technical Implementation

The knowledge graph implementation consists of several components:

- **Entity Module** (`src/graph/entity.rs`): Defines entity types and their properties
- **Relationship Module** (`src/graph/relationship.rs`): Defines relationship types
- **Knowledge Graph** (`src/graph/knowledge_graph.rs`): Core graph operations
- **Database** (`src/db.rs`): Storage and retrieval operations

## Knowledge Graph Applications

The knowledge graph powers all of Ummon's features:

- **Query System**: Structured and natural language queries
- **Relevance Agent**: Finding relevant files for a task
- **Domain Extraction**: Mapping business concepts to code
- **Assistance**: Context-aware recommendations

## Best Practices

- **Regular Indexing**: Keep your knowledge graph up-to-date by reindexing after significant code changes
- **Incremental Updates**: Use incremental updates for day-to-day development
- **Full Rebuilds**: Periodically perform full rebuilds to ensure graph consistency
- **Domain Extraction**: Enable domain extraction for projects with complex business logic
