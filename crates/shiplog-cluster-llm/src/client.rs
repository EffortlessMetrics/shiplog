use anyhow::{Context, Result};

/// Abstraction over LLM APIs. Enables testing with mocks.
pub trait LlmBackend {
    fn complete(&self, system: &str, user: &str) -> Result<String>;
}

/// Backend that speaks the OpenAI chat completions protocol.
pub struct OpenAiCompatibleBackend {
    pub endpoint: String,
    pub api_key: String,
    pub model: String,
    pub temperature: f64,
    pub timeout_secs: u64,
}

impl LlmBackend for OpenAiCompatibleBackend {
    fn complete(&self, system: &str, user: &str) -> Result<String> {
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(self.timeout_secs))
            .build()?;

        let body = serde_json::json!({
            "model": self.model,
            "temperature": self.temperature,
            "response_format": { "type": "json_object" },
            "messages": [
                { "role": "system", "content": system },
                { "role": "user", "content": user }
            ]
        });

        let resp = client
            .post(&self.endpoint)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .context("LLM API request failed")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().unwrap_or_default();
            anyhow::bail!("LLM API returned {status}: {text}");
        }

        let json: serde_json::Value = resp.json().context("parse LLM response")?;
        let content = json["choices"][0]["message"]["content"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("no content in LLM response"))?
            .to_string();

        Ok(content)
    }
}

/// Mock backend for testing.
pub struct MockLlmBackend {
    pub response: String,
}

impl LlmBackend for MockLlmBackend {
    fn complete(&self, _system: &str, _user: &str) -> Result<String> {
        Ok(self.response.clone())
    }
}

/// Mock backend that always fails.
pub struct FailingLlmBackend;

impl LlmBackend for FailingLlmBackend {
    fn complete(&self, _system: &str, _user: &str) -> Result<String> {
        anyhow::bail!("LLM backend failed (mock)")
    }
}
