---
title: Configuration
description: Configure Ummon for your environment and preferences
---

# Ummon Configuration

This document outlines how to configure Ummon for your specific needs and environment.

## Configuration Methods

Ummon uses a combination of:

1. Command-line options (for run-specific settings)
2. Environment variables (for sensitive information)

There is no configuration file at this time, which simplifies usage and deployment.

## Environment Variables

### Required Variables

- `OPENROUTER_API_KEY`: API key for accessing LLM services
  - Required for: natural language queries, assisted recommendations, domain extraction
  - Example: `export OPENROUTER_API_KEY="your-api-key-here"`

### Optional Variables

- `DOMAIN_EXTRACTION_DIR`: Directory to focus on for domain extraction
  - Default: `src`
  - Example: `export DOMAIN_EXTRACTION_DIR="app/models"`

## Command-Line Configuration

Most configuration is handled through command-line options. Each command has its own set of options. See the [CLI Reference](cli-reference.md) for details.

## LLM Provider Configuration

Ummon uses OpenRouter by default for LLM services. You can configure this with command-line options:

```bash
# Use a specific provider
ummon query "show auth functions" --llm-provider openrouter

# Use a specific model
ummon query "show auth functions" --llm-model anthropic/claude-3-opus-20240229
```

## Performance Tuning

### Database Configuration

Ummon uses an SQLite database to store the knowledge graph. The database is created in a `.ummon` directory in your home directory by default. You can tune SQLite performance by:

- Ensuring sufficient disk space for the database
- Using an SSD for better I/O performance
- Checking that file system permissions allow read/write access

### Memory Usage

Ummon is designed to be memory-efficient, but indexing very large codebases may require significant memory. Key considerations:

- Incremental updates use less memory than full rebuilds
- Domain extraction requires additional memory for LLM processing
- Very large projects (>1M lines of code) may benefit from indexing subsets

## Language Support Configuration

Ummon supports multiple programming languages out of the box:

- Rust
- Python
- JavaScript
- Java

No additional configuration is needed for these languages.

## Security Considerations

Ummon follows these security practices:

- **API Keys**: Stored only in environment variables, never in files
- **No Remote Code**: Does not execute remote code or download external resources
- **Local Processing**: Processes files locally except for LLM queries
- **LLM Data**: Sends only code snippets to LLMs, not environment or security data

## Logging and Debugging

Ummon uses the Rust `tracing` crate for logging. You can enable verbose output with:

```bash
ummon index . -v
```

## Common Configuration Patterns

### Development Environment

```bash
# Store your API key in your shell profile
echo 'export OPENROUTER_API_KEY="your-key-here"' >> ~/.bashrc

# Use incremental updates for faster indexing during development
ummon index .
```

### CI/CD Environment

```bash
# Set API key as a CI secret variable
# Use full rebuilds to ensure consistency
ummon index . --full --enable-domain-extraction
```

## Future Configuration Options

In future releases, Ummon plans to add:

- Configuration file support for persistent settings
- Additional LLM provider options
- Custom language parser configurations
- Team collaboration settings