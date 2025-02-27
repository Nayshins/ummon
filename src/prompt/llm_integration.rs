use anyhow::Result;
use reqwest::Client;
use serde_json::Value;
use std::time::Duration;

/// Async function calling the OpenRouter API with better error handling and retries
pub async fn query_llm(prompt: &str, api_key: &str) -> Result<String> {
    if api_key.is_empty() {
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

    println!("  Sending LLM request for domain extraction...");

    let client = Client::builder()
        .timeout(Duration::from_secs(120))
        .build()?;

    let body = serde_json::json!({
        "model": "anthropic/claude-3-opus-20240229",
        "messages": [
            {"role": "system", "content": "You are a domain modeling expert."},
            {"role": "user", "content": prompt}
        ],
        "temperature": 0.2, // Lower temperature for more deterministic output
        "max_tokens": 1500  // Enough tokens for complex output
    });

    let max_retries = 3;
    let mut attempt = 0;

    loop {
        attempt += 1;
        println!("  LLM API call attempt {}/{}", attempt, max_retries);

        match try_llm_query(&client, api_key, &body).await {
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
                println!("  LLM API call failed: {}. Retrying in {:?}...", e, backoff);
                tokio::time::sleep(backoff).await;
            }
        }
    }
}

async fn try_llm_query(client: &Client, api_key: &str, body: &Value) -> Result<String> {
    let res = client
        .post("https://openrouter.ai/api/v1/chat/completions")
        .header("HTTP-Referer", "https://github.com/yourusername/ummon")
        .header("Content-Type", "application/json")
        .bearer_auth(api_key)
        .json(&body)
        .send()
        .await?;

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

    if let Some(choice) = json["choices"].as_array().and_then(|arr| arr.get(0)) {
        if let Some(msg) = choice["message"]["content"].as_str() {
            return Ok(msg.to_string());
        }
    }

    // Check for error message in the response
    if let Some(error) = json["error"].as_object() {
        if let Some(message) = error["message"].as_str() {
            return Err(anyhow::anyhow!("API error: {}", message));
        }
    }

    Err(anyhow::anyhow!("Invalid response format from LLM API"))
}
