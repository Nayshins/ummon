use anyhow::Result;
use reqwest::Client;
use serde_json::Value;
use std::str::FromStr;
use std::time::Duration;

/// Enum representing different LLM API providers
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LlmProvider {
    OpenRouter,
    OpenAI,
    Anthropic,
    GoogleVertexAI,
    Ollama,
    Mock,
}

impl FromStr for LlmProvider {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "openrouter" => Ok(LlmProvider::OpenRouter),
            "openai" => Ok(LlmProvider::OpenAI),
            "anthropic" => Ok(LlmProvider::Anthropic),
            "google" | "vertexai" | "vertex" => Ok(LlmProvider::GoogleVertexAI),
            "ollama" | "local" => Ok(LlmProvider::Ollama),
            "mock" => Ok(LlmProvider::Mock),
            _ => Err(anyhow::anyhow!("Unknown LLM provider: {}", s)),
        }
    }
}

/// Configuration for an LLM request
#[derive(Debug, Clone)]
pub struct LlmConfig {
    pub provider: LlmProvider,
    pub api_key: String,
    pub model: String,
    pub temperature: f32,
    pub max_tokens: u32,
    pub endpoint_url: Option<String>,
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            provider: LlmProvider::OpenRouter,
            api_key: String::new(),
            model: "anthropic/claude-3-5-haiku-20241022".to_string(),
            temperature: 0.2,
            max_tokens: 1500,
            endpoint_url: None,
        }
    }
}

/// Async function calling the selected LLM API with error handling and retries
pub async fn query_llm(prompt: &str, config: &LlmConfig) -> Result<String> {
    // Use mock response if no API key is provided
    if config.api_key.is_empty() || config.provider == LlmProvider::Mock {
        return Ok(r#"[
            {
                "name": "MockEntity",
                "entity_type": "Class",
                "description": "This is a mock entity because no API key was provided",
                "attributes": {
                    "id": "String",
                    "name": "String"
                },
                "relationships": []
            }
        ]"#
        .to_string());
    }

    tracing::info!(
        "Sending LLM request to {:?} for domain extraction...",
        config.provider
    );

    let client = Client::builder()
        .timeout(Duration::from_secs(120))
        .build()?;

    // Prepare request body based on provider
    let body = prepare_request_body(prompt, config);

    // Maximum retry attempts
    let max_retries = 3;
    let mut attempt = 0;

    loop {
        attempt += 1;
        tracing::info!("LLM API call attempt {}/{}", attempt, max_retries);

        match try_llm_query(&client, config, &body).await {
            Ok(result) => return Ok(result),
            Err(e) => {
                if attempt >= max_retries {
                    return Err(anyhow::anyhow!(
                        "Failed after {} attempts: {}",
                        max_retries,
                        e
                    ));
                }
                // Exponential backoff
                let backoff = Duration::from_millis(500 * 2u64.pow(attempt as u32 - 1));
                tracing::warn!("LLM API call failed: {}. Retrying in {:?}...", e, backoff);
                tokio::time::sleep(backoff).await;
            }
        }
    }
}

fn prepare_request_body(prompt: &str, config: &LlmConfig) -> Value {
    match config.provider {
        LlmProvider::OpenRouter => {
            serde_json::json!({
                "model": config.model,
                "messages": [
                    {"role": "system", "content": "You are a domain modeling expert."},
                    {"role": "user", "content": prompt}
                ],
                "temperature": config.temperature,
                "max_tokens": config.max_tokens
            })
        }
        LlmProvider::OpenAI => {
            serde_json::json!({
                "model": config.model,
                "messages": [
                    {"role": "system", "content": "You are a domain modeling expert."},
                    {"role": "user", "content": prompt}
                ],
                "temperature": config.temperature,
                "max_tokens": config.max_tokens
            })
        }
        LlmProvider::Anthropic => {
            serde_json::json!({
                "model": config.model,
                "messages": [
                    {"role": "system", "content": "You are a domain modeling expert."},
                    {"role": "user", "content": prompt}
                ],
                "temperature": config.temperature,
                "max_tokens": config.max_tokens
            })
        }
        LlmProvider::GoogleVertexAI => {
            serde_json::json!({
                "contents": [
                    {"role": "system", "parts": [{"text": "You are a domain modeling expert."}]},
                    {"role": "user", "parts": [{"text": prompt}]}
                ],
                "generationConfig": {
                    "temperature": config.temperature,
                    "maxOutputTokens": config.max_tokens
                }
            })
        }
        LlmProvider::Ollama => {
            serde_json::json!({
                "model": config.model,
                "messages": [
                    {"role": "system", "content": "You are a domain modeling expert."},
                    {"role": "user", "content": prompt}
                ],
                "temperature": config.temperature,
                "max_tokens": config.max_tokens
            })
        }
        LlmProvider::Mock => {
            serde_json::json!({}) // Not used
        }
    }
}

async fn try_llm_query(client: &Client, config: &LlmConfig, body: &Value) -> Result<String> {
    let endpoint = get_endpoint_url(config);
    let headers = get_request_headers(config);

    let mut request = client.post(endpoint).json(&body);

    // Add headers
    for (key, value) in headers {
        request = request.header(key, value);
    }

    // Add authentication
    request = match config.provider {
        LlmProvider::GoogleVertexAI => request, // Google uses different auth mechanism
        _ => request.bearer_auth(&config.api_key),
    };

    let res = request.send().await?;

    // Check for HTTP errors
    if !res.status().is_success() {
        let status = res.status();
        let error_text = res
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        return Err(anyhow::anyhow!("HTTP error {}: {}", status, error_text));
    }

    let json: Value = res.json().await?;
    parse_response(json, config.provider)
}

fn get_endpoint_url(config: &LlmConfig) -> String {
    // Use custom endpoint if provided
    if let Some(url) = &config.endpoint_url {
        return url.clone();
    }

    // Default endpoints
    match config.provider {
        LlmProvider::OpenRouter => "https://openrouter.ai/api/v1/chat/completions".to_string(),
        LlmProvider::OpenAI => "https://api.openai.com/v1/chat/completions".to_string(),
        LlmProvider::Anthropic => "https://api.anthropic.com/v1/messages".to_string(),
        LlmProvider::GoogleVertexAI => format!(
            "https://us-central1-aiplatform.googleapis.com/v1/projects/{}/locations/us-central1/publishers/google/models/{}:predict",
            "your-project-id", // This should be configured properly
            config.model
        ),
        LlmProvider::Ollama => "http://localhost:11434/api/chat".to_string(),
        LlmProvider::Mock => "".to_string(),
    }
}

fn get_request_headers(config: &LlmConfig) -> Vec<(&'static str, String)> {
    let mut headers = vec![("Content-Type", "application/json".to_string())];

    match config.provider {
        LlmProvider::OpenRouter => {
            headers.push((
                "HTTP-Referer",
                "https://github.com/yourusername/ummon".to_string(),
            ));
        }
        LlmProvider::Anthropic => {
            headers.push(("Anthropic-Version", "2023-06-01".to_string()));
            headers.push(("X-API-Key", config.api_key.clone()));
        }
        _ => {}
    }

    headers
}

fn parse_response(json: Value, provider: LlmProvider) -> Result<String> {
    match provider {
        LlmProvider::OpenRouter | LlmProvider::OpenAI => {
            if let Some(choice) = json["choices"].as_array().and_then(|arr| arr.get(0)) {
                if let Some(msg) = choice["message"]["content"].as_str() {
                    return Ok(msg.to_string());
                }
            }
        }
        LlmProvider::Anthropic => {
            if let Some(content) = json["content"].as_array().and_then(|arr| arr.get(0)) {
                if let Some(text) = content["text"].as_str() {
                    return Ok(text.to_string());
                }
            }
        }
        LlmProvider::GoogleVertexAI => {
            if let Some(predictions) = json["predictions"].as_array().and_then(|arr| arr.get(0)) {
                if let Some(text) = predictions["candidates"]
                    .as_array()
                    .and_then(|arr| arr.get(0))
                    .and_then(|candidate| candidate["content"].as_str())
                {
                    return Ok(text.to_string());
                }
            }
        }
        LlmProvider::Ollama => {
            if let Some(msg) = json["message"]["content"].as_str() {
                return Ok(msg.to_string());
            }
        }
        LlmProvider::Mock => {
            // Mock response handling not needed here
        }
    }

    // Check for error message in the response
    if let Some(error) = json["error"].as_object() {
        if let Some(message) = error["message"].as_str() {
            return Err(anyhow::anyhow!("API error: {}", message));
        }
    }

    Err(anyhow::anyhow!(
        "Invalid response format from LLM API: {:?}",
        json
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_llm_provider_parsing() {
        // Test all valid provider strings
        assert_eq!(
            LlmProvider::from_str("openrouter").unwrap(),
            LlmProvider::OpenRouter
        );
        assert_eq!(
            LlmProvider::from_str("openai").unwrap(),
            LlmProvider::OpenAI
        );
        assert_eq!(
            LlmProvider::from_str("anthropic").unwrap(),
            LlmProvider::Anthropic
        );
        assert_eq!(
            LlmProvider::from_str("google").unwrap(),
            LlmProvider::GoogleVertexAI
        );
        assert_eq!(
            LlmProvider::from_str("vertexai").unwrap(),
            LlmProvider::GoogleVertexAI
        );
        assert_eq!(
            LlmProvider::from_str("vertex").unwrap(),
            LlmProvider::GoogleVertexAI
        );
        assert_eq!(
            LlmProvider::from_str("ollama").unwrap(),
            LlmProvider::Ollama
        );
        assert_eq!(LlmProvider::from_str("local").unwrap(), LlmProvider::Ollama);
        assert_eq!(LlmProvider::from_str("mock").unwrap(), LlmProvider::Mock);

        // Test case insensitivity
        assert_eq!(
            LlmProvider::from_str("OpenRouter").unwrap(),
            LlmProvider::OpenRouter
        );
        assert_eq!(
            LlmProvider::from_str("OPENAI").unwrap(),
            LlmProvider::OpenAI
        );

        // Test invalid provider
        assert!(LlmProvider::from_str("unknown_provider").is_err());
    }

    #[test]
    fn test_llm_config_default() {
        let config = LlmConfig::default();
        assert_eq!(config.provider, LlmProvider::OpenRouter);
        assert_eq!(config.model, "anthropic/claude-3-5-haiku-20241022");
        assert_eq!(config.temperature, 0.2);
        assert_eq!(config.max_tokens, 1500);
        assert!(config.endpoint_url.is_none());
    }

    #[test]
    fn test_prepare_request_body() {
        let test_prompt = "Test prompt";

        // Test OpenRouter request body
        let config = LlmConfig {
            provider: LlmProvider::OpenRouter,
            api_key: "test_key".to_string(),
            model: "test_model".to_string(),
            temperature: 0.5,
            max_tokens: 100,
            endpoint_url: None,
        };

        let body = prepare_request_body(test_prompt, &config);
        assert_eq!(body["model"], "test_model");
        assert_eq!(body["temperature"], 0.5);
        assert_eq!(body["max_tokens"], 100);
        assert_eq!(body["messages"][0]["role"], "system");
        assert_eq!(body["messages"][1]["role"], "user");
        assert_eq!(body["messages"][1]["content"], test_prompt);

        // Test Google Vertex AI request body (different format)
        let config = LlmConfig {
            provider: LlmProvider::GoogleVertexAI,
            api_key: "test_key".to_string(),
            model: "test_model".to_string(),
            temperature: 0.7,
            max_tokens: 200,
            endpoint_url: None,
        };

        let body = prepare_request_body(test_prompt, &config);
        assert_eq!(body["contents"][0]["role"], "system");
        assert_eq!(body["contents"][1]["role"], "user");
        assert_eq!(body["contents"][1]["parts"][0]["text"], test_prompt);
        assert!(
            body["generationConfig"]["temperature"].as_f64().unwrap() > 0.69
                && body["generationConfig"]["temperature"].as_f64().unwrap() < 0.71
        );
        assert_eq!(body["generationConfig"]["maxOutputTokens"], 200);
    }

    #[test]
    fn test_get_endpoint_url() {
        // Test default endpoints
        let config = LlmConfig {
            provider: LlmProvider::OpenRouter,
            api_key: "".to_string(),
            model: "".to_string(),
            temperature: 0.0,
            max_tokens: 0,
            endpoint_url: None,
        };
        assert_eq!(
            get_endpoint_url(&config),
            "https://openrouter.ai/api/v1/chat/completions"
        );

        // Test custom endpoint
        let config = LlmConfig {
            provider: LlmProvider::OpenRouter,
            api_key: "".to_string(),
            model: "".to_string(),
            temperature: 0.0,
            max_tokens: 0,
            endpoint_url: Some("https://custom.endpoint/api".to_string()),
        };
        assert_eq!(get_endpoint_url(&config), "https://custom.endpoint/api");
    }

    #[test]
    fn test_get_request_headers() {
        // Test OpenRouter headers
        let config = LlmConfig {
            provider: LlmProvider::OpenRouter,
            api_key: "test_key".to_string(),
            model: "".to_string(),
            temperature: 0.0,
            max_tokens: 0,
            endpoint_url: None,
        };
        let headers = get_request_headers(&config);
        assert!(headers.contains(&("Content-Type", "application/json".to_string())));
        assert!(headers.contains(&(
            "HTTP-Referer",
            "https://github.com/yourusername/ummon".to_string()
        )));

        // Test Anthropic headers
        let config = LlmConfig {
            provider: LlmProvider::Anthropic,
            api_key: "test_key".to_string(),
            model: "".to_string(),
            temperature: 0.0,
            max_tokens: 0,
            endpoint_url: None,
        };
        let headers = get_request_headers(&config);
        assert!(headers.contains(&("Content-Type", "application/json".to_string())));
        assert!(headers.contains(&("Anthropic-Version", "2023-06-01".to_string())));
        assert!(headers.contains(&("X-API-Key", "test_key".to_string())));
    }
}
