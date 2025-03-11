use anyhow::{anyhow, Result};

use crate::prompt::llm_integration::{query_llm, LlmConfig};

#[cfg(test)]
use crate::prompt::llm_integration::LlmProvider;
use crate::query::parser::parse_query;

/// Translates natural language queries into Ummon query language
pub struct NaturalLanguageTranslator {
    config: LlmConfig,
}

impl NaturalLanguageTranslator {
    pub fn new(config: LlmConfig) -> Self {
        Self { config }
    }

    /// Translate a natural language query to Ummon query language
    pub async fn translate(&self, natural_query: &str) -> Result<(String, f32)> {
        // Build a prompt that explains the query language and provides examples
        let prompt = self.build_translation_prompt(natural_query);

        // Query the LLM
        let response = query_llm(&prompt, &self.config).await?;

        // Extract the translated query and confidence score
        self.extract_query_and_confidence(&response)
    }

    /// Build a prompt for the LLM to translate a natural language query
    fn build_translation_prompt(&self, natural_query: &str) -> String {
        format!(
            r#"# Ummon Query Language Translation Task

Your task is to translate a natural language query into Ummon's query language. Ummon has a simple, structured query language for accessing a knowledge graph of code entities.

## Ummon Query Language Syntax
Ummon's query language supports two main query types:

1. **Select queries** - to find entities by type with optional conditions:
   `select [entity_type] where [conditions]`

2. **Traversal queries** - to find relationships between entities:
   `[entity_type] [relationship] [entity_type] where [conditions]`

### Entity Types
- `functions` - Functions in code
- `methods` - Methods in classes
- `classes` - Classes or types  
- `modules` - Modules or files
- `variables` - Variables or fields
- `constants` - Constant values
- `domain_concepts` - Business domain concepts

### Relationships
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
- `[attribute] [operator] [value]` - e.g., `name = 'auth'` or `file_path like 'src/%'`
- Attributes include: `name`, `file_path`, `documentation`, `confidence`
- Operators include: `=`, `!=`, `>`, `<`, `>=`, `<=`, `like` (supports % wildcard)
- Logical operators: `and`, `or`, `not`
- Grouping with parentheses: `(name like 'auth%' or name like 'login%')`
- Existence check: `has documentation`

## Examples
- "Show me all functions" → `select functions`
- "Find functions with names starting with auth" → `select functions where name like 'auth%'`
- "Show classes in the src directory" → `select classes where file_path like 'src/%'`
- "Find functions that call authentication functions" → `functions calling functions where name like 'auth%'`
- "List classes containing getter methods" → `classes containing methods where name like 'get%'`
- "What domain concepts have high confidence?" → `select domain_concepts where confidence > 0.8`
- "Show functions related to authentication or login" → `select functions where name like 'auth%' or name like 'login%'`
- "Find functions in auth module that implement validation" → `select functions where file_path like '%auth%' and (name like '%validate%' or name like '%check%')`

## Your Task
Translate the following natural language query into Ummon's query language:

"{}"

Provide your answer in this format:
TRANSLATED_QUERY: <your translated query>
CONFIDENCE: <your confidence score between 0 and 1>
EXPLANATION: <brief explanation>
"#,
            natural_query
        )
    }

    /// Extract the translated query and confidence from the LLM response
    fn extract_query_and_confidence(&self, response: &str) -> Result<(String, f32)> {
        // Extract translated query
        let query_line = response
            .lines()
            .find(|line| line.starts_with("TRANSLATED_QUERY:"))
            .ok_or_else(|| anyhow!("No translated query found in response"))?;

        let translated_query = query_line
            .strip_prefix("TRANSLATED_QUERY:")
            .map(|s| s.trim())
            .ok_or_else(|| anyhow!("Failed to extract translated query"))?;

        // Extract confidence
        let confidence_line = response
            .lines()
            .find(|line| line.starts_with("CONFIDENCE:"))
            .ok_or_else(|| anyhow!("No confidence score found in response"))?;

        let confidence_str = confidence_line
            .strip_prefix("CONFIDENCE:")
            .map(|s| s.trim())
            .ok_or_else(|| anyhow!("Failed to extract confidence score"))?;

        let confidence = confidence_str.parse::<f32>().map_err(|_| {
            anyhow!(
                "Failed to parse confidence score '{}' as a number",
                confidence_str
            )
        })?;

        // Validate the translated query by parsing it
        match parse_query(translated_query) {
            Ok(_) => Ok((translated_query.to_string(), confidence)),
            Err(e) => Err(anyhow!(
                "Translated query '{}' is not valid: {}",
                translated_query,
                e
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    // Skipped in normal test runs since it requires LLM access
    #[tokio::test]
    #[ignore]
    async fn test_translate_simple_query() {
        // Only run this test if environment variables for LLM are set
        let api_key = env::var("OPENAI_API_KEY").ok();
        if api_key.is_none() {
            println!("Skipping test_translate_simple_query as OPENAI_API_KEY is not set");
            return;
        }

        let config = LlmConfig {
            provider: LlmProvider::OpenAI,
            model: "gpt-3.5-turbo".to_string(),
            api_key: api_key.unwrap(),
            temperature: 0.0,
            max_tokens: 1000,
            endpoint_url: None,
        };

        let translator = NaturalLanguageTranslator::new(config);
        let (query, confidence) = translator.translate("show me all functions").await.unwrap();

        // Check that we got a valid query with reasonable confidence
        assert!(query.contains("functions"));
        assert!(confidence > 0.5);
    }

    // Test the prompt structure and extraction logic
    #[test]
    fn test_build_translation_prompt() {
        let config = LlmConfig {
            provider: LlmProvider::Mock,
            model: "dummy".to_string(),
            api_key: "dummy".to_string(),
            temperature: 0.0,
            max_tokens: 1000,
            endpoint_url: None,
        };

        let translator = NaturalLanguageTranslator::new(config);
        let prompt = translator.build_translation_prompt("find auth functions");

        // Check that the prompt contains the necessary elements
        assert!(prompt.contains("Ummon Query Language"));
        assert!(prompt.contains("find auth functions"));
        assert!(prompt.contains("select functions where name like 'auth%'"));
    }

    #[test]
    fn test_extract_query_and_confidence() {
        let config = LlmConfig {
            provider: LlmProvider::Mock,
            model: "dummy".to_string(),
            api_key: "dummy".to_string(),
            temperature: 0.0,
            max_tokens: 1000,
            endpoint_url: None,
        };

        let translator = NaturalLanguageTranslator::new(config);

        // Using an invalid query format to trigger parse_query error
        let response = r#"
TRANSLATED_QUERY: invalidquery functions where name like 'auth%'
CONFIDENCE: 0.95
EXPLANATION: This query finds all functions with names starting with 'auth'.
"#;

        // This uses an invalid query format that will cause parse_query to fail
        let result = translator.extract_query_and_confidence(response);
        assert!(result.is_err());

        // Invalid response (missing translated query)
        let response = r#"
I think the query should be: select functions
CONFIDENCE: 0.7
EXPLANATION: This is a simple query to get all functions.
"#;

        let result = translator.extract_query_and_confidence(response);
        assert!(result.is_err());

        // Invalid response (missing confidence)
        let response = r#"
TRANSLATED_QUERY: select functions
I'm very confident in this translation.
EXPLANATION: This is a simple query to get all functions.
"#;

        let result = translator.extract_query_and_confidence(response);
        assert!(result.is_err());
    }
}
