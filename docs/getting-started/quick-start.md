---
title: Quick Start
description: Get up and running with Ummon in minutes
---

# Quick Start Guide

This guide will help you get started with Ummon quickly. We'll cover indexing a codebase, running basic queries, and using the relevance agent.

## 1. Installing Ummon

If you haven't installed Ummon yet, you can do so with Cargo:

```bash
cargo install ummon
```

For detailed installation instructions, see the [Installation Guide](/getting-started/installation.md).

## 2. Indexing Your Codebase

Before you can query your codebase, you need to build a knowledge graph by indexing your code:

```bash
# Navigate to your project directory
cd /path/to/your/project

# Index the current directory
ummon index .
```

This will create a knowledge graph of your codebase, indexing functions, classes, modules, and their relationships.

## 3. Running Basic Queries

Once your code is indexed, you can start querying it:

```bash
# Find all functions
ummon query "select functions"

# Find authentication-related functions
ummon query "show me all authentication functions"

# Find function call relationships
ummon query "functions calling functions where name like 'auth%'"
```

## 4. Using Different Output Formats

Ummon supports multiple output formats:

```bash
# JSON output (useful for scripting)
ummon query "select functions" --format json

# CSV output (for spreadsheets)
ummon query "select functions" --format csv

# Tree view (for hierarchical data)
ummon query "select functions" --format tree
```

## 5. Using the Relevance Agent

When working on a specific task, the relevance agent can suggest relevant files:

```bash
# Get files relevant to fixing an authentication bug
ummon assist --suggest-files "fix authentication token validation"
```

## 6. Enabling Domain Model Extraction

To enrich your knowledge graph with business domain concepts:

```bash
# Index with domain model extraction
ummon index . --enable-domain-extraction
```

This uses LLM analysis to identify and connect domain concepts with your codebase.

## 7. Getting Assistance

Ummon can provide AI-assisted recommendations for development tasks:

```bash
# Get assistance with implementing a feature
ummon assist "implement a user registration function"
```

## Next Steps

Now that you're familiar with the basics, you can explore more advanced features:

- Learn about the [Query System](/features/query-system.md) for more complex queries
- Explore the [Relevance Agent](/features/relevance-agent.md) for context-aware assistance
- Understand how the [Knowledge Graph](/features/knowledge-graph.md) works
- Discover [Domain Model Extraction](/features/domain-extraction.md) capabilities
