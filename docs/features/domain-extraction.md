---
title: Domain Model Extraction
description: Automatically extract business concepts from your codebase
---

# Domain Model Extraction

Ummon includes the ability to use LLMs (Large Language Models) to automatically extract domain models from your codebase. This feature analyzes your code and identifies business concepts, entities, and their relationships.

## Overview

The domain extraction system:

1. Identifies candidate files that may contain domain entities
2. Sends code content to an LLM for analysis
3. Extracts domain entities with attributes and relationships
4. Adds these domain entities to the knowledge graph

## Benefits

Domain model extraction provides several advantages:

- **Business-Technical Alignment**: Creates a bridge between business concepts and implementation details
- **Enhanced Queries**: Allows querying by domain concepts (e.g., "find code related to payment processing")
- **Improved Context**: Provides richer context for the relevance agent and assistance features
- **Documentation**: Helps document the implicit domain model in your code

## Usage

To enable domain extraction when indexing your codebase:

```bash
# Index with domain model extraction enabled
ummon index /path/to/codebase --enable-domain-extraction

# Specify a custom domain directory for extraction
ummon index /path/to/codebase --enable-domain-extraction --domain-dir models/
```

### Environment Variables

You can also configure domain extraction using environment variables:

```bash
# Set your OpenRouter API key (required)
export OPENROUTER_API_KEY=your_api_key_here

# Optional: Specify directory to analyze (defaults to "src")
export DOMAIN_EXTRACTION_DIR=path/to/analyze
```

## Extracted Information

For each identified domain entity, the system extracts:

- **Entity name**: The name of the business concept
- **Entity type**: Class, Struct, Enum, Interface, etc.
- **Attributes**: Properties with their types
- **Relationships**: Connections to other domain entities
- **Description**: A concise explanation of the entity's purpose

## Example Output

After extraction, you can query domain concepts:

```bash
# List all domain concepts
ummon query "select domain_concepts"

# Find domain concepts with high confidence
ummon query "select domain_concepts where confidence > 0.8"

# Find code implementing a domain concept
ummon query "domain_concepts represented_by functions where domain_concepts.name = 'PaymentProcessing'"
```

Sample output for a domain concept:

```
Domain Concept: PaymentProcessing
Type: BusinessProcess
Description: Handles payment authorization, processing, and settlement
Confidence: 0.92
Implemented in:
  - src/services/payment_service.rs
  - src/models/payment.rs
Relationships:
  - Related to: UserAccount, Order, Transaction
```

## File Selection

The system analyzes source files with these constraints:
- Only processes source code files (with extensions like .rs, .py, .js, etc.)
- Skips files specified in `.gitignore`
- Skips very large files (>100,000 bytes)
- Truncates files that are too long (>10,000 chars) to fit within token limits

## Technical Implementation

The domain extraction uses:

- `LlmModelExtractor`: Implements the `DomainModelBuilder` trait
- LLM prompting to analyze code and extract domain entities
- JSON parsing to convert LLM responses to structured entities
- Knowledge graph integration to store domain concepts

## Customization

The domain extraction can be fine-tuned by modifying:

1. File selection heuristics in `src/commands/index.rs`
2. The LLM prompt template in `src/prompt/domain_extraction.rs`
3. The response parsing logic in `src/parser/domain_model.rs`

## Limitations

- Accuracy depends on code quality and naming conventions
- May struggle with highly technical or abstract code
- Large files are automatically truncated to fit within token limits
- Performance depends on the LLM service's response time
