use crate::providers::{LlmProvider, ProviderKind};
use base64::{engine::general_purpose, Engine};
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{fs, path::Path};
use tauri::{AppHandle, Emitter};

const STREAM_TOKEN_EVENT: &str = "llm-stream-token";
const STREAM_DONE_EVENT: &str = "llm-stream-done";
const STREAM_ERROR_EVENT: &str = "llm-stream-error";

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LlmChatRequest {
    pub request_id: String,
    pub provider: LlmProvider,
    pub prompt: String,
    pub persona_prompt: Option<String>,
    pub capture_path: Option<String>,
    pub history_messages: Vec<LlmHistoryMessage>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LlmHistoryMessage {
    pub role: LlmHistoryRole,
    pub content: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LlmHistoryRole {
    User,
    Assistant,
    System,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct StreamToken {
    request_id: String,
    token: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct StreamStatus {
    request_id: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct StreamError {
    request_id: String,
    message: String,
}

pub async fn stream_chat_completion(app: AppHandle, request: LlmChatRequest) -> Result<(), String> {
    let result = stream_openai_compatible_response(&app, &request).await;

    if let Err(message) = result {
        app.emit(
            STREAM_ERROR_EVENT,
            StreamError {
                request_id: request.request_id,
                message: message.clone(),
            },
        )
        .map_err(|error| error.to_string())?;

        return Err(message);
    }

    app.emit(
        STREAM_DONE_EVENT,
        StreamStatus {
            request_id: request.request_id,
        },
    )
    .map_err(|error| error.to_string())
}

async fn stream_openai_compatible_response(
    app: &AppHandle,
    request: &LlmChatRequest,
) -> Result<(), String> {
    let client = reqwest::Client::new();
    let endpoint = chat_completions_endpoint(&request.provider.base_url);
    let response = client
        .post(endpoint)
        .headers(request_headers(&request.provider)?)
        .json(&chat_request_body(request)?)
        .send()
        .await
        .map_err(|error| error.to_string())?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();

        return Err(format!("Provider returned {status}: {body}"));
    }

    let mut pending_chunk = String::new();
    let mut stream = response.bytes_stream();

    while let Some(chunk) = stream.next().await {
        pending_chunk.push_str(
            std::str::from_utf8(&chunk.map_err(|error| error.to_string())?)
                .map_err(|error| error.to_string())?,
        );

        while let Some(line_break_index) = pending_chunk.find('\n') {
            let line = pending_chunk[..line_break_index].trim().to_string();
            pending_chunk = pending_chunk[line_break_index + 1..].to_string();
            handle_stream_line(app, &request.request_id, &line)?;
        }
    }

    Ok(())
}

fn request_headers(provider: &LlmProvider) -> Result<reqwest::header::HeaderMap, String> {
    let mut headers = reqwest::header::HeaderMap::new();

    headers.insert(
        reqwest::header::CONTENT_TYPE,
        reqwest::header::HeaderValue::from_static("application/json"),
    );

    if !provider.api_key.trim().is_empty() {
        let value = format!("Bearer {}", provider.api_key.trim());
        headers.insert(
            reqwest::header::AUTHORIZATION,
            reqwest::header::HeaderValue::from_str(&value).map_err(|error| error.to_string())?,
        );
    }

    if matches!(provider.kind, ProviderKind::Openrouter) {
        headers.insert(
            reqwest::header::HeaderName::from_static("x-title"),
            reqwest::header::HeaderValue::from_static("Waey"),
        );
    }

    Ok(headers)
}

fn chat_request_body(request: &LlmChatRequest) -> Result<Value, String> {
    let mut messages = vec![json!({
        "role": "system",
        "content": "You are Waey, a concise screen-aware desktop assistant. Answer directly using the user's screen context when an image is attached."
    })];

    if let Some(persona_prompt) = persona_system_message(request) {
        messages.push(persona_prompt);
    }

    messages.extend(history_messages(request));
    messages.push(json!({
        "role": "user",
        "content": user_message_content(request)?
    }));

    Ok(json!({
        "model": request.provider.model,
        "stream": true,
        "messages": messages
    }))
}

fn persona_system_message(request: &LlmChatRequest) -> Option<Value> {
    let prompt = request.persona_prompt.as_ref()?.trim();

    if prompt.is_empty() {
        return None;
    }

    Some(json!({
        "role": "system",
        "content": prompt
    }))
}

fn history_messages(request: &LlmChatRequest) -> Vec<Value> {
    request
        .history_messages
        .iter()
        .filter(|message| !message.content.trim().is_empty())
        .map(|message| {
            json!({
                "role": history_role_to_string(&message.role),
                "content": message.content
            })
        })
        .collect()
}

fn user_message_content(request: &LlmChatRequest) -> Result<Value, String> {
    let Some(capture_path) = &request.capture_path else {
        return Ok(json!(request.prompt));
    };

    Ok(json!([
        {
            "type": "text",
            "text": request.prompt
        },
        {
            "type": "image_url",
            "image_url": {
                "url": image_data_url(capture_path)?
            }
        }
    ]))
}

fn image_data_url(path: &str) -> Result<String, String> {
    let bytes = fs::read(path).map_err(|error| error.to_string())?;
    let mime_type = match Path::new(path).extension().and_then(|value| value.to_str()) {
        Some("jpg") | Some("jpeg") => "image/jpeg",
        _ => "image/png",
    };

    Ok(format!(
        "data:{mime_type};base64,{}",
        general_purpose::STANDARD.encode(bytes)
    ))
}

fn chat_completions_endpoint(base_url: &str) -> String {
    let trimmed_url = base_url.trim().trim_end_matches('/');

    if trimmed_url.ends_with("/chat/completions") {
        trimmed_url.to_string()
    } else {
        format!("{trimmed_url}/chat/completions")
    }
}

fn handle_stream_line(app: &AppHandle, request_id: &str, line: &str) -> Result<(), String> {
    if !line.starts_with("data:") {
        return Ok(());
    }

    let data = line.trim_start_matches("data:").trim();

    if data == "[DONE]" {
        return Ok(());
    }

    let value: Value = serde_json::from_str(data).map_err(|error| error.to_string())?;
    let Some(token) = value["choices"][0]["delta"]["content"].as_str() else {
        return Ok(());
    };

    app.emit(
        STREAM_TOKEN_EVENT,
        StreamToken {
            request_id: request_id.to_string(),
            token: token.to_string(),
        },
    )
    .map_err(|error| error.to_string())
}

fn history_role_to_string(role: &LlmHistoryRole) -> &'static str {
    match role {
        LlmHistoryRole::User => "user",
        LlmHistoryRole::Assistant => "assistant",
        LlmHistoryRole::System => "system",
    }
}
