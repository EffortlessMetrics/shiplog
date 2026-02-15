/// Configuration for LLM-assisted clustering.
pub struct LlmConfig {
    /// API endpoint (e.g., "https://api.openai.com/v1/chat/completions")
    pub api_endpoint: String,
    /// API key for authentication
    pub api_key: String,
    /// Model name (default: "gpt-4o-mini")
    pub model: String,
    /// Max tokens for input context (~4 chars/token heuristic)
    pub max_input_tokens: usize,
    /// Temperature for generation (default: 0.2)
    pub temperature: f64,
    /// Maximum workstreams to create
    pub max_workstreams: Option<usize>,
    /// Request timeout in seconds
    pub timeout_secs: u64,
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            api_endpoint: "https://api.openai.com/v1/chat/completions".to_string(),
            api_key: String::new(),
            model: "gpt-4o-mini".to_string(),
            max_input_tokens: 8000,
            temperature: 0.2,
            max_workstreams: None,
            timeout_secs: 60,
        }
    }
}
