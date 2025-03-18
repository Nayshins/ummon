# Ummon Query System

The Ummon query system allows you to search and analyze your codebase using both a structured query language and natural language queries. This guide explains how to use the query system effectively.

## Query Language Syntax

Ummon's query language supports two main query types:

1. **Select queries** - find entities by type with optional conditions:
   ```
   select [entity_type] where [conditions]
   ```

2. **Traversal queries** - find relationships between entities:
   ```
   [source_entity_type] [relationship] [target_entity_type] where [conditions]
   ```

### Entity Types

The following entity types are supported:

- `functions` - Functions in code
- `methods` - Methods in classes
- `classes` - Classes or types  
- `modules` - Modules or files
- `variables` - Variables or fields
- `constants` - Constant values
- `domain_concepts` - Business domain concepts

### Relationships

Relationships between entities can be expressed using these keywords:

- `calls`/`calling` - Function/method calls another
- `contains`/`containing` - Entity contains another
- `imports`/`importing` - Entity imports another
- `inherits`/`inheriting` - Class inherits from another
- `implements`/`implementing` - Class implements interface
- `references`/`referencing` - Entity references another
- `uses`/`using` - Entity uses another
- `depends_on`/`depending` - Entity depends on another
- `represented_by` - Domain concept is represented by code
- `relates_to` - General relationship between entities

### Conditions

Conditions help filter entities based on attributes:

- `[attribute] [operator] [value]` - e.g., `name = 'auth'` or `file_path like 'src/%'`
- Attributes include: `name`, `file_path`, `documentation`, `confidence`
- Operators include: `=`, `!=`, `>`, `<`, `>=`, `<=`, `like` (supports % wildcard)
- Logical operators: `and`, `or`, `not`
- Existence check: `has documentation`

## Output Formats

Ummon supports multiple output formats:

- `text` - Simple text format (default)
- `json` - JSON format for programmatic use
- `csv` - CSV format for spreadsheet import
- `tree` - Tree view for hierarchical data

## Common Query Examples

Here are some common query patterns:

### Finding Functions

```
# Find all functions
select functions

# Find functions with specific naming pattern
select functions where name like 'auth%'

# Find functions in specific directories
select functions where file_path like 'src/auth/%'

# Find functions with documentation
select functions where has documentation
```

### Finding Related Code

```
# Find functions calling authentication functions
functions calling functions where name like 'auth%'

# Find classes containing getter methods
classes containing methods where name like 'get%'

# Find methods used by a specific class
methods used_by classes where name = 'UserController'
```

### Finding Domain Concepts

```
# Find all domain concepts
select domain_concepts

# Find high-confidence domain concepts
select domain_concepts where confidence > 0.8

# Find code implementing a domain concept
domain_concepts represented_by functions
```

### Complex Queries

```
# Functions in auth module that implement validation
select functions where file_path like '%auth%' and (name like '%validate%' or name like '%check%')

# Classes that both inherit and implement interfaces
classes where inherits and implements
```

## Natural Language Queries

Ummon supports natural language queries, which are automatically translated to structured queries:

```bash
# Using structured syntax
ummon query "select functions where name like 'auth%'"

# Using natural language
ummon query "show me all authentication functions"
```

You can disable natural language processing with the `--no-llm` flag if you prefer to use the structured syntax directly.

## Command Line Options

```
ummon query [OPTIONS] <QUERY>

Arguments:
  <QUERY>  Natural language query (e.g., "show all functions related to user authentication")

Options:
  -f, --format <FORMAT>  Output format (text, json, csv, tree) [default: text]
  -l, --limit <LIMIT>    Maximum number of results to return [default: 20]
  --no-llm               Skip LLM and only use direct knowledge graph queries
  --llm-provider <LLM_PROVIDER>  LLM provider to use for querying [default: openrouter]
  --llm-model <LLM_MODEL>        LLM model to use
  -h, --help             Print help
```

## Advanced Usage

### Performance Tips

- Use more specific entity types to narrow down results
- Add file path conditions to limit search scope
- Combine conditions to create more precise queries
- Use `limit` to control result size when working with large codebases

### Query System Components

The query system consists of:

1. **Parser** - Converts query strings into structured representations
2. **Executor** - Performs the actual search against the knowledge graph
3. **Formatter** - Presents results in different formats
4. **NL Translator** - Converts natural language to structured queries using LLMs

## Further Reading

For more information about Ummon, see:
- [README.md](../README.md) - Project overview