use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmEndpoint {
    pub id: i64,
    pub name: String,
    pub provider: String,
    pub endpoint_url: String,
    pub api_key: Option<String>,
    pub model: String,
    pub is_default: bool,
}

#[derive(Debug, Serialize)]
struct OpenAiMessage {
    role: String,
    content: String,
}

#[derive(Debug, Serialize)]
struct OpenAiRequest {
    model: String,
    messages: Vec<OpenAiMessage>,
    temperature: f32,
}

#[derive(Debug, Deserialize)]
struct OpenAiResponse {
    choices: Vec<OpenAiChoice>,
}

#[derive(Debug, Deserialize)]
struct OpenAiChoice {
    message: OpenAiResponseMessage,
}

#[derive(Debug, Deserialize)]
struct OpenAiResponseMessage {
    content: String,
}

#[derive(Debug, Serialize)]
struct AnthropicMessage {
    role: String,
    content: String,
}

#[derive(Debug, Serialize)]
struct AnthropicRequest {
    model: String,
    messages: Vec<AnthropicMessage>,
    max_tokens: u32,
}

#[derive(Debug, Deserialize)]
struct AnthropicResponse {
    content: Vec<AnthropicContent>,
}

#[derive(Debug, Deserialize)]
struct AnthropicContent {
    text: String,
}

pub struct LlmClient {
    client: Client,
}

impl LlmClient {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }

    pub fn complete(
        &self,
        endpoint: &LlmEndpoint,
        system_prompt: &str,
        user_prompt: &str,
        temperature: f32,
        _max_tokens: u32,
    ) -> Result<String, String> {
        match endpoint.provider.as_str() {
            "openai" => self.openai_complete(endpoint, system_prompt, user_prompt, temperature),
            "anthropic" => self.anthropic_complete(endpoint, system_prompt, user_prompt),
            "local" => self.local_complete(endpoint, system_prompt, user_prompt, temperature),
            "bedrock" => self.bedrock_complete(endpoint, system_prompt, user_prompt),
            _ => Err(format!("Unknown provider: {}", endpoint.provider)),
        }
    }

    fn openai_complete(
        &self,
        endpoint: &LlmEndpoint,
        system_prompt: &str,
        user_prompt: &str,
        temperature: f32,
    ) -> Result<String, String> {
        let api_key = endpoint
            .api_key
            .as_ref()
            .ok_or("OpenAI requires an API key")?;

        let url = if endpoint.endpoint_url.is_empty() {
            format!("https://api.openai.com/v1/chat/completions")
        } else {
            endpoint.endpoint_url.clone()
        };

        let request = OpenAiRequest {
            model: endpoint.model.clone(),
            messages: vec![
                OpenAiMessage {
                    role: "system".to_string(),
                    content: system_prompt.to_string(),
                },
                OpenAiMessage {
                    role: "user".to_string(),
                    content: user_prompt.to_string(),
                },
            ],
            temperature,
        };

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .map_err(|e| format!("Failed to call OpenAI: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().unwrap_or_default();
            return Err(format!("OpenAI API error ({}): {}", status, body));
        }

        let response_json: OpenAiResponse = response
            .json()
            .map_err(|e| format!("Failed to parse OpenAI response: {}", e))?;

        response_json
            .choices
            .first()
            .map(|c| c.message.content.clone())
            .ok_or_else(|| "No response from OpenAI".to_string())
    }

    fn anthropic_complete(
        &self,
        endpoint: &LlmEndpoint,
        system_prompt: &str,
        user_prompt: &str,
    ) -> Result<String, String> {
        let api_key = endpoint
            .api_key
            .as_ref()
            .ok_or("Anthropic requires an API key")?;

        let url = if endpoint.endpoint_url.is_empty() {
            format!("https://api.anthropic.com/v1/messages")
        } else {
            endpoint.endpoint_url.clone()
        };

        let combined_prompt = format!("{}\n\n{}", system_prompt, user_prompt);

        let request = AnthropicRequest {
            model: endpoint.model.clone(),
            messages: vec![AnthropicMessage {
                role: "user".to_string(),
                content: combined_prompt,
            }],
            max_tokens: 1024,
        };

        let response = self
            .client
            .post(&url)
            .header("x-api-key", api_key)
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .map_err(|e| format!("Failed to call Anthropic: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().unwrap_or_default();
            return Err(format!("Anthropic API error ({}): {}", status, body));
        }

        let response_json: AnthropicResponse = response
            .json()
            .map_err(|e| format!("Failed to parse Anthropic response: {}", e))?;

        response_json
            .content
            .first()
            .map(|c| c.text.clone())
            .ok_or_else(|| "No response from Anthropic".to_string())
    }

    fn local_complete(
        &self,
        endpoint: &LlmEndpoint,
        system_prompt: &str,
        user_prompt: &str,
        temperature: f32,
    ) -> Result<String, String> {
        let url = if endpoint.endpoint_url.is_empty() {
            return Err("Local provider requires an endpoint_url".to_string());
        } else {
            endpoint.endpoint_url.clone()
        };

        let request = OpenAiRequest {
            model: endpoint.model.clone(),
            messages: vec![
                OpenAiMessage {
                    role: "system".to_string(),
                    content: system_prompt.to_string(),
                },
                OpenAiMessage {
                    role: "user".to_string(),
                    content: user_prompt.to_string(),
                },
            ],
            temperature,
        };

        let response = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .map_err(|e| format!("Failed to call local LLM: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().unwrap_or_default();
            return Err(format!("Local LLM error ({}): {}", status, body));
        }

        let response_json: OpenAiResponse = response
            .json()
            .map_err(|e| format!("Failed to parse local LLM response: {}", e))?;

        response_json
            .choices
            .first()
            .map(|c| c.message.content.clone())
            .ok_or_else(|| "No response from local LLM".to_string())
    }

    fn bedrock_complete(
        &self,
        _endpoint: &LlmEndpoint,
        _system_prompt: &str,
        _user_prompt: &str,
    ) -> Result<String, String> {
        Err("Bedrock provider not yet implemented".to_string())
    }
}

impl Default for LlmClient {
    fn default() -> Self {
        Self::new()
    }
}

pub fn parse_classification_response(
    response: &str,
    _cognitive_axis_column: &str,
    _context_axis_column: Option<&str>,
) -> Result<(String, Option<String>), String> {
    let response_lower = response.to_lowercase();

    let cognitive_axis = if response_lower.contains("require") {
        "require"
    } else if response_lower.contains("review") {
        "review"
    } else if response_lower.contains("delegate") {
        "delegate"
    } else if response_lower.contains("delete") {
        "delete"
    } else if response_lower.contains("schedule") {
        "schedule"
    } else if response_lower.contains("call") {
        "call"
    } else if response_lower.contains("meeting") {
        "meeting"
    } else {
        "General"
    };

    let context_axis = if response_lower.contains("work") {
        Some("Work".to_string())
    } else if response_lower.contains("personal") {
        Some("Personal".to_string())
    } else if response_lower.contains("urgent") || response_lower.contains("important") {
        Some("Urgent".to_string())
    } else {
        Some("General".to_string())
    };

    Ok((cognitive_axis.to_string(), context_axis))
}
