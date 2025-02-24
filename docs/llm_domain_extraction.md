# LLM-Based Domain Model Extraction

Ummon includes the ability to use LLMs (Large Language Models) to automatically extract domain models from your codebase. This feature analyzes your code and identifies business concepts, entities, and their relationships.

## Overview

The domain extraction system:

1. Identifies candidate files that may contain domain entities
2. Sends code content to an LLM for analysis
3. Extracts domain entities with attributes and relationships
4. Adds these domain entities to the knowledge graph

## Usage

To enable LLM-based domain extraction:

```bash
# Set your OpenRouter API key
export OPENROUTER_API_KEY=your_api_key_here

# Enable domain extraction
export USE_LLM_MODEL_EXTRACTION=true

# Optional: Specify directory to analyze (defaults to "src")
export DOMAIN_EXTRACTION_DIR=path/to/analyze

# Run the indexing command
ummon index path/to/your/code
```

## Configuration

### Environment Variables

- `OPENROUTER_API_KEY`: Your API key for OpenRouter
- `USE_LLM_MODEL_EXTRACTION`: Set to "true" to enable domain extraction
- `DOMAIN_EXTRACTION_DIR`: Directory to analyze (defaults to "src" if not specified)

### Fine-Tuning

The domain extraction can be further configured by modifying:

1. File selection heuristics in `src/commands/index.rs`
2. The LLM prompt template in `src/prompt/domain_extraction.rs`
3. The response parsing logic in `src/parser/domain_model.rs`

## Implementation Details

### Architecture

The domain extraction uses:

- `LlmModelExtractor`: Implements the `DomainModelBuilder` trait
- `parse_domain_entities_from_llm_response`: Parses structured JSON from LLM responses
- Heuristics to identify files likely to contain domain entities

### LLM Interaction

The system:

1. Creates a prompt with instructions for domain modeling
2. Sends code to the LLM via OpenRouter API
3. Receives JSON-formatted domain entities
4. Converts to internal domain entity representations

### File Selection

All source files in the codebase are analyzed, with the following constraints:
- Only processes source code files (with extensions like .rs, .py, .js, etc.)
- Skips files specified in `.gitignore`
- Skips very large files (>100,000 bytes)
- Truncates files that are too long (>10,000 chars) to fit within token limits

## Example Output

For each identified domain entity, the system extracts:

- Entity name
- Entity type (Class, Struct, Enum, Interface)
- Attributes with types
- Relationships to other entities
- Description

## Technical Notes

- Large files are automatically truncated to fit within token limits
- The system includes fallback to mock entities when no API key is provided
- The internal implementation is async-aware and handles runtime constraints