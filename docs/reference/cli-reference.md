---
title: CLI Reference
description: Comprehensive reference for Ummon command-line options
---

# CLI Reference

This document provides a comprehensive reference for all Ummon command-line options and subcommands.

## Global Options

These options apply to all Ummon commands:

```
Ummon - A code analysis tool for building knowledge graphs

Usage: ummon [OPTIONS] <COMMAND>

Commands:
  index    Index a codebase to build or update the knowledge graph
  query    Query the knowledge graph
  assist   Get AI-assisted recommendations or suggestions
  help     Display help for a specific command

Options:
  -h, --help     Print help
  -V, --version  Print version
```

## Index Command

The `index` command builds or updates the knowledge graph from your codebase.

```
ummon index [OPTIONS] <PATH>

Arguments:
  <PATH>  Path to the directory to index

Options:
  -f, --full                   Perform a full rebuild of the knowledge graph
  --enable-domain-extraction   Enable LLM-based domain model extraction
  --domain-dir <DOMAIN_DIR>    Specify a custom directory for domain extraction [default: src]
  --llm-provider <PROVIDER>    LLM provider to use [default: openrouter]
  --llm-model <MODEL>          LLM model to use
  -v, --verbose                Enable verbose output
  -h, --help                   Print help
```

### Examples

```bash
# Incremental update of the knowledge graph
ummon index .

# Full rebuild of the knowledge graph
ummon index . --full

# Index with domain model extraction
ummon index . --enable-domain-extraction

# Specify a custom domain directory
ummon index . --enable-domain-extraction --domain-dir models/
```

## Query Command

The `query` command searches and analyzes your codebase using the knowledge graph.

```
ummon query [OPTIONS] <QUERY>

Arguments:
  <QUERY>  Query string (e.g., "select functions where name like 'auth%'")

Options:
  -f, --format <FORMAT>        Output format: text, json, csv, tree [default: text]
  -l, --limit <LIMIT>          Maximum number of results to return [default: 20]
  --no-llm                     Skip LLM and only use direct knowledge graph queries
  --type-filter <TYPE>         Filter results by entity type
  --path <PATH>                Filter results by file path pattern
  --llm-provider <PROVIDER>    LLM provider to use [default: openrouter]
  --llm-model <MODEL>          LLM model to use
  -h, --help                   Print help
```

### Examples

```bash
# Query using natural language
ummon query "show all authentication functions"

# Query using structured syntax
ummon query "select functions where name like 'auth%'" --no-llm

# Traversal query with relationship
ummon query "functions calling functions where name like 'validate%'" --no-llm

# JSON output format
ummon query "select functions" --format json

# Filter by type
ummon query "find api" --type-filter function

# Filter by path
ummon query "show all entities" --path src/auth

# Limit results
ummon query "select functions" --limit 10
```

## Assist Command

The `assist` command provides AI-assisted recommendations and file suggestions.

```
ummon assist [OPTIONS] <PROMPT>

Arguments:
  <PROMPT>  Description of the task or question

Options:
  --suggest-files              Suggest relevant files for the given prompt
  --top <TOP>                  Number of suggestions to return [default: 5]
  --llm-provider <PROVIDER>    LLM provider to use [default: openrouter]
  --llm-model <MODEL>          LLM model to use
  -h, --help                   Print help
```

### Examples

```bash
# Get assistance with implementing a feature
ummon assist "implement a user registration function"

# Get file suggestions for a proposed change
ummon assist --suggest-files "fix authentication token validation"

# Adjust the number of suggestions
ummon assist --suggest-files "add payment processing" --top 10
```

## Environment Variables

Ummon uses environment variables for sensitive configuration:

- `OPENROUTER_API_KEY`: API key for LLM services (required for natural language queries, assistance, and domain extraction)

## Exit Codes

Ummon returns the following exit codes:

- `0`: Success
- `1`: General error
- `2`: Configuration error
- `3`: File system error
- `4`: Database error
- `5`: LLM service error
