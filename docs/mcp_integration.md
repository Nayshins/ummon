# Model Context Protocol (MCP) Integration

Ummon provides a Model Context Protocol (MCP) server that enables AI agents to interact with your codebase knowledge graph. This integration allows AI assistants to better understand your code structure, entities, relationships, and architecture.

## Overview

The MCP server exposes a set of tools that AI agents can use to:

1. Search for code entities using natural language
2. Explore specific entities and their relationships
3. Understand the architectural structure of the codebase
4. Find relevant files for implementing features or fixing bugs
5. Visualize relationships between components

## Starting the MCP Server

To start the MCP server, use the `serve` command:

```bash
# Start with default stdin/stdout transport (for CLI tool integration)
ummon serve

# Start with HTTP transport (for networked integration)
ummon serve --transport http
```

## Available Tools

### Basic Knowledge Graph Tools

#### `search_code`
Searches for code entities using natural language queries.

Parameters:
- `query` (string, required): Natural language query (e.g., "knowledge graph implementation")

Example response:
```
Found 12 results:

Functions:
- entity_42: add_to_knowledge_graph (Function)
- entity_87: update_knowledge_graph (Method)

Types:
- entity_21: KnowledgeGraph (Class)
- entity_35: GraphNode (Struct)
```

#### `get_entity`
Gets detailed information about a specific entity.

Parameters:
- `entity_id` (string, required): The ID of the entity to retrieve information about

Example response:
```
Entity: KnowledgeGraph (Class)
ID: entity_21
Path: src/graph/knowledge_graph.rs
Description: Stores and manages the code knowledge graph

Relationships:
- entity_21 Contains entity_35
- entity_42 References entity_21
```

#### `debug_graph`
Gets information about the loaded knowledge graph.

Parameters: None

Example response:
```
Knowledge Graph Status:

Total entities: 342
Total relationships: 528

Sample entities:
- entity_1 (File): main.rs
- entity_21 (Class): KnowledgeGraph
- entity_35 (Struct): GraphNode
```

### Enhanced Agent Tools

#### `find_relevant_files`
Finds the most relevant files for a specific task or feature.

Parameters:
- `description` (string, required): Description of the task or feature
- `limit` (integer, optional): Maximum number of files to return (default: 5)

Example response:
```
Found 3 relevant files for task: 'parsing source code'

1. parser.rs (relevance score: 8)
   Path: src/parser/mod.rs
   Contains:
      - Parser (Struct)
      - parse_file (Function)
      - ParseResult (Type)

2. rust.rs (relevance score: 6)
   Path: src/parser/language_support/rust.rs
   Contains:
      - RustParser (Struct)
      - parse_rust_file (Function)
      - extract_functions (Method)

3. javascript.rs (relevance score: 4)
   Path: src/parser/language_support/javascript.rs
   Contains:
      - JavaScriptParser (Struct)
      - parse_javascript_file (Function)
      - extract_functions (Method)

Recommendation: Start by examining these files to understand the relevant components for your task.
```

#### `explore_relationships`
Explores and explains relationships between entities in the codebase.

Parameters:
- `entity_id` (string, required): ID of the entity to explore relationships for
- `relationship_type` (string, optional): Filter for specific relationship types
- `depth` (integer, optional): Maximum relationship depth to explore (default: 1)

Example response:
```
Relationships for KnowledgeGraph (Class) - entity_21:

Outgoing Relationships:
- Contains → GraphNode (Struct)
- Contains → Relationship (Struct)
- Uses → HashMap (External)

Incoming Relationships:
- Used By → add_to_knowledge_graph (Function)
- Referenced By → update_knowledge_graph (Method)

Related Entities Graph:
KnowledgeGraph
├─ Contains → GraphNode
│  └─ Contains → NodeData
├─ Contains → Relationship
│  └─ Contains → RelationshipType
└─ Uses → HashMap

Key Insights:
- KnowledgeGraph is a central component with 8 relationships
- It primarily manages GraphNode and Relationship entities
- Several utility functions interact with this class
```

#### `explain_architecture`
Provides an architectural overview of the codebase or a specific area.

Parameters:
- `focus_area` (string, optional): Focus area to limit the explanation
- `detail_level` (string, optional): Level of detail (high, medium, low) (default: medium)

Example response:
```
# Codebase Architecture Overview

## Module Structure
- src/
  - commands/: Command line interface implementations
  - graph/: Knowledge graph data structures and operations
  - parser/: Code parsing and analysis for different languages
  - prompt/: LLM integration and prompt templates
  - mcp_server/: Model Context Protocol server implementation

## Component Analysis
- Total Entities: 342
  - Functions: 156 (45.6%)
  - Classes/Structs: 48 (14.0%)
  - Modules/Files: 26 (7.6%)
  - Other: 112 (32.8%)
  
## Relationships Analysis
- Total relationships: 528
  - Contains: 184 (34.8%)
  - Calls: 147 (27.8%)
  - Uses: 98 (18.6%)
  - Imports: 64 (12.1%)
  - Other: 35 (6.6%)

## Central Components
- KnowledgeGraph (Class) with 24 relationships
- Parser (Struct) with 18 relationships
- Entity (Trait) with 15 relationships

## Working With This Codebase
### Key Entry Points
- main (Function): src/main.rs
- run (Function): src/commands/index.rs
- serve (Function): src/commands/serve.rs

### Recommendations
1. Start with the key entry points identified above
2. Follow the relationships to understand dependencies
3. Use find_relevant_files to identify components for specific tasks
4. For understanding complex dependencies, use explore_relationships
5. Refer to central components as they provide core functionality
```

## Testing the MCP Server

A JavaScript test client is included in the repository to test the MCP server functionality:

```bash
# Start the MCP server
cargo run -- serve

# In another terminal, run the test client
node test_mcp_client.js
```

The test client sends requests to each of the MCP tools to demonstrate their functionality.

## Implementation Notes

- Tools are implemented in `src/mcp_server/router.rs`
- The MCP server uses the knowledge graph built by the `index` command
- More detailed relationship and architecture analyses require more compute time