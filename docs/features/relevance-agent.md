---
title: Relevance Agent
description: Context-aware file suggestion system for code changes
---

# Relevance Agent

The Relevance Agent is a context-aware system that suggests files relevant to proposed changes or queries. It leverages the knowledge graph to identify the most likely files that would need modification for a given task.

## Overview

When working with large codebases, identifying which files to modify for a particular change can be challenging. The Relevance Agent addresses this by:

1. Extracting technical keywords from natural language descriptions
2. Identifying related entities in the knowledge graph
3. Scoring files by relevance using both proximity and centrality metrics
4. Providing a ranked list of the most relevant files

## How It Works

The Relevance Agent employs a multi-step process to determine file relevance:

### 1. Keyword Extraction

When provided with a natural language description of a change (e.g., "Fix authentication token validation"), the agent uses an LLM to extract key technical concepts, entity names, and domain terms. This produces a list of keywords like `["authentication", "token", "validation"]`.

### 2. Seed Entity Discovery

The system searches the knowledge graph for entities matching these keywords, looking at:
- Entity names
- File paths
- Documentation

These become "seed entities" with initial relevance scores based on how closely they match the keywords.

### 3. Context Expansion

The agent then expands outward from these seed entities by traversing relationships in the knowledge graph:
- Function calls
- Contains relationships
- Imports
- References
- Domain concept mappings

This builds a broader network of potentially relevant entities with scores that diminish based on distance from seed entities.

### 4. Entity Ranking

Entities are ranked using a hybrid scoring approach that considers:
- Proximity Score: How closely related to seed entities (70% weight)
- Centrality Score: How central/important in the subgraph (30% weight)

### 5. File Aggregation

Finally, entity scores are aggregated to the file level, producing a ranked list of files most relevant to the proposed change.

## Usage

```bash
# Get file suggestions for a proposed change
ummon assist --suggest-files "fix authentication token validation"
```

## Example Output

```
Relevant files for "fix authentication token validation":

1. src/auth/token_validator.rs (relevance: 0.92)
2. src/auth/middleware.rs (relevance: 0.81)
3. src/models/user_session.rs (relevance: 0.75)
4. src/api/auth_routes.rs (relevance: 0.68)
5. src/config/security_settings.rs (relevance: 0.59)
```

## Configuration Options

Currently, the Relevance Agent operates with default settings optimized for most use cases. Future versions will include customization options such as:

- Adjusting the maximum relationship traversal depth
- Tuning the scoring weights between proximity and centrality
- Setting minimum relevance thresholds for inclusion

## Technical Implementation

The Relevance Agent is implemented in the `src/agent/relevance_agent.rs` file. Key components include:

- `suggest_relevant_files()`: Main entry point that orchestrates the process
- `extract_keywords()`: Uses LLM to extract keywords from the query
- `search_seed_entities()`: Identifies initial entities matching keywords
- `expand_context()`: Traverses the graph to find related entities
- `rank_entities()`: Scores entities using the hybrid approach
- `aggregate_and_rank_files()`: Groups by file and calculates final scores

The implementation includes fallback mechanisms for robust operation and limits result sets to prevent overwhelming the user with too many suggestions.