use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use std::env;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

pub const MAX_MODEL_INPUT_BYTES: usize = 2_048;

pub struct CloudBudget {
    calls: AtomicUsize,
    cap: usize,
}

impl Default for CloudBudget {
    fn default() -> Self {
        let cap = env::var("LEADFINDER_CLOUD_CALL_CAP")
            .ok()
            .and_then(|value| value.parse().ok())
            .unwrap_or(20)
            .clamp(1, 100);
        Self {
            calls: AtomicUsize::new(0),
            cap,
        }
    }
}

impl CloudBudget {
    fn reserve(&self) -> Result<usize, String> {
        self.calls
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |current| {
                (current < self.cap).then_some(current + 1)
            })
            .map(|previous| previous + 1)
            .map_err(|current| {
                format!(
                    "Cloud call cap reached ({current}/{}) — lead remains unqualified",
                    self.cap
                )
            })
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RouterStatus {
    pub router_ready: bool,
    pub endpoint: String,
    pub fast_model: String,
    pub mid_model: String,
    pub message: String,
}

#[derive(Debug, Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
    #[serde(default)]
    usage: Usage,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: Message,
}

#[derive(Debug, Deserialize)]
struct Message {
    content: String,
}

#[derive(Debug, Default, Deserialize)]
struct Usage {
    #[serde(default)]
    prompt_tokens: u64,
    #[serde(default)]
    completion_tokens: u64,
    #[serde(default)]
    total_tokens: u64,
}

pub fn fast_model() -> String {
    env::var("LEADFINDER_FAST_MODEL").unwrap_or_else(|_| "gc/gemini-2.5-flash-lite".to_string())
}

pub fn mid_model() -> String {
    env::var("LEADFINDER_MID_MODEL").unwrap_or_else(|_| "gc/gemini-2.5-flash".to_string())
}

fn base_url() -> String {
    env::var("LEADFINDER_9ROUTER_URL")
        .unwrap_or_else(|_| "http://127.0.0.1:20128/v1".to_string())
        .trim_end_matches('/')
        .to_string()
}

fn client() -> Result<Client, String> {
    Client::builder()
        .connect_timeout(Duration::from_secs(5))
        .timeout(Duration::from_secs(45))
        .build()
        .map_err(|error| error.to_string())
}

fn with_auth(request: reqwest::blocking::RequestBuilder) -> reqwest::blocking::RequestBuilder {
    match env::var("LEADFINDER_9ROUTER_KEY") {
        Ok(key) if !key.trim().is_empty() => request.bearer_auth(key),
        _ => request,
    }
}

pub fn router_status() -> RouterStatus {
    let endpoint = base_url();
    let request = match client() {
        Ok(client) => with_auth(client.get(format!("{endpoint}/models"))),
        Err(error) => {
            return RouterStatus {
                router_ready: false,
                endpoint,
                fast_model: fast_model(),
                mid_model: mid_model(),
                message: error,
            }
        }
    };
    match request.send() {
        Ok(response) if response.status().is_success() => RouterStatus {
            router_ready: true,
            endpoint,
            fast_model: fast_model(),
            mid_model: mid_model(),
            message: "9router ready".to_string(),
        },
        Ok(response) => RouterStatus {
            router_ready: false,
            endpoint,
            fast_model: fast_model(),
            mid_model: mid_model(),
            message: format!("9router returned {}", response.status()),
        },
        Err(error) => RouterStatus {
            router_ready: false,
            endpoint,
            fast_model: fast_model(),
            mid_model: mid_model(),
            message: format!("9router unavailable: {error}"),
        },
    }
}

pub fn chat_completion(
    budget: &CloudBudget,
    model: &str,
    system: &str,
    input: &str,
    max_tokens: u32,
) -> Result<String, String> {
    validate_model_input(input)?;
    let call_number = budget.reserve()?;
    let endpoint = format!("{}/chat/completions", base_url());
    let request = with_auth(client()?.post(&endpoint)).json(&serde_json::json!({
        "model": model,
        "messages": [
            {"role": "system", "content": system},
            {"role": "user", "content": input}
        ],
        "stream": false,
        "temperature": 0.1,
        "max_tokens": max_tokens
    }));
    let response = request
        .send()
        .map_err(|error| format!("9router unavailable; lead remains unqualified: {error}"))?;
    let status = response.status();
    if !status.is_success() {
        return Err(format!(
            "9router returned {status}; lead remains unqualified and no fallback was used"
        ));
    }
    let parsed: ChatResponse = response
        .json()
        .map_err(|error| format!("9router returned malformed JSON: {error}"))?;
    log::info!(
        "cloud_call={} model={} prompt_tokens={} completion_tokens={} total_tokens={}",
        call_number,
        model,
        parsed.usage.prompt_tokens,
        parsed.usage.completion_tokens,
        parsed.usage.total_tokens
    );
    parsed
        .choices
        .into_iter()
        .next()
        .map(|choice| choice.message.content.trim().to_string())
        .filter(|content| !content.is_empty())
        .ok_or_else(|| "9router returned no message content".to_string())
}

pub fn validate_model_input(input: &str) -> Result<(), String> {
    if input.len() > MAX_MODEL_INPUT_BYTES {
        return Err(format!(
            "Model input is {} bytes; maximum is {MAX_MODEL_INPUT_BYTES}",
            input.len()
        ));
    }
    let lowercase = input.to_ascii_lowercase();
    let html_markers = [
        "<!doctype html",
        "<html",
        "<head",
        "<body",
        "<script",
        "<style",
        "<div",
        "<meta",
        "</",
    ];
    if html_markers.iter().any(|marker| lowercase.contains(marker)) {
        return Err(
            "Raw HTML is forbidden in model input; extract bounded signals first".to_string(),
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{validate_model_input, MAX_MODEL_INPUT_BYTES};

    #[test]
    fn raw_html_and_signals_over_two_kilobytes_never_reach_a_model() {
        assert!(validate_model_input(&"x".repeat(MAX_MODEL_INPUT_BYTES)).is_ok());
        assert!(validate_model_input(&"x".repeat(MAX_MODEL_INPUT_BYTES + 1)).is_err());
        assert!(validate_model_input("<html><body>raw shop page</body></html>").is_err());
        assert!(validate_model_input("<!doctype html><title>raw page</title>").is_err());
    }
}
