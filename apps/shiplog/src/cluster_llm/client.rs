use anyhow::{Context, Result};
use url::Url;

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
    #[mutants::skip]
    fn complete(&self, system: &str, user: &str) -> Result<String> {
        validate_https_endpoint(&self.endpoint)?;

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

fn validate_https_endpoint(endpoint: &str) -> Result<()> {
    let parsed = Url::parse(endpoint).context("parse LLM API endpoint")?;
    if parsed.scheme() != "https" {
        anyhow::bail!("LLM API endpoint must use https, got {}", parsed.scheme());
    }
    if parsed.host_str().is_none() {
        anyhow::bail!("LLM API endpoint must include a host");
    }
    Ok(())
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
    #[mutants::skip]
    fn complete(&self, _system: &str, _user: &str) -> Result<String> {
        anyhow::bail!("LLM backend failed (mock)")
    }
}

#[cfg(test)]
mod tests {
    use super::validate_https_endpoint;
    use anyhow::Result;

    #[test]
    fn accepts_https_endpoint() -> Result<()> {
        validate_https_endpoint("https://api.example.com/v1/chat/completions")?;
        Ok(())
    }

    #[test]
    fn rejects_http_endpoint() -> Result<()> {
        let error = validate_https_endpoint("http://api.example.com/v1/chat/completions")
            .err()
            .ok_or_else(|| anyhow::anyhow!("HTTP endpoint should be rejected"))?;
        if !error.to_string().contains("must use https") {
            anyhow::bail!("unexpected validation error: {error}");
        }
        Ok(())
    }

    #[test]
    fn rejects_invalid_endpoint() -> Result<()> {
        validate_https_endpoint("not a URL")
            .err()
            .ok_or_else(|| anyhow::anyhow!("invalid endpoint should be rejected"))?;
        Ok(())
    }
}
