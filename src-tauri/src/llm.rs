use crate::providers::{LlmProvider, ProviderKind};
use crate::ui_context::UiContextSnapshot;
use base64::{engine::general_purpose, Engine};
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{collections::HashSet, fs, path::Path, sync::Mutex};
use tauri::{AppHandle, Emitter, Manager};

const STREAM_TOKEN_EVENT: &str = "llm-stream-token";
const STREAM_REASONING_EVENT: &str = "llm-stream-reasoning";
const STREAM_DONE_EVENT: &str = "llm-stream-done";
const STREAM_ERROR_EVENT: &str = "llm-stream-error";

const MANAGED_GROQ_QWEN_MAX_COMPLETION_TOKENS: u32 = 2_048;
const CUSTOM_GROQ_QWEN_MAX_COMPLETION_TOKENS: u32 = 4_096;

#[derive(Default)]
pub struct LlmRequestRegistry {
    cancelled_request_ids: Mutex<HashSet<String>>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LlmChatRequest {
    pub request_id: String,
    pub provider: LlmProvider,
    pub prompt: String,
    pub persona_prompt: Option<String>,
    pub capture_path: Option<String>,
    pub capture_paths: Option<Vec<String>>,
    pub ui_contexts: Option<Vec<UiContextSnapshot>>,
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
    finish_reason: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct StreamError {
    request_id: String,
    message: String,
}

enum StreamCompletion {
    Done(Option<String>),
    Cancelled,
}

pub fn cancel_chat_completion(app: &AppHandle, request_id: String) -> Result<(), String> {
    request_registry(app).cancel(request_id)
}

pub async fn stream_chat_completion(app: AppHandle, request: LlmChatRequest) -> Result<(), String> {
    request_registry(&app).clear_cancelled(&request.request_id)?;
    let result = stream_openai_compatible_response(&app, &request).await;

    match result {
        Ok(StreamCompletion::Done(finish_reason)) => app
            .emit(
                STREAM_DONE_EVENT,
                StreamStatus {
                    request_id: request.request_id,
                    finish_reason,
                },
            )
            .map_err(|error| error.to_string()),
        Ok(StreamCompletion::Cancelled) => Ok(()),
        Err(message) => {
            app.emit(
                STREAM_ERROR_EVENT,
                StreamError {
                    request_id: request.request_id,
                    message: message.clone(),
                },
            )
            .map_err(|error| error.to_string())?;

            Err(message)
        }
    }
}

async fn stream_openai_compatible_response(
    app: &AppHandle,
    request: &LlmChatRequest,
) -> Result<StreamCompletion, String> {
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

    let registry = request_registry(app);
    let mut pending_chunk = String::new();
    let mut stream = response.bytes_stream();

    while let Some(chunk) = stream.next().await {
        if registry.is_cancelled(&request.request_id)? {
            registry.clear_cancelled(&request.request_id)?;
            return Ok(StreamCompletion::Cancelled);
        }

        pending_chunk.push_str(
            std::str::from_utf8(&chunk.map_err(|error| error.to_string())?)
                .map_err(|error| error.to_string())?,
        );

        while let Some(line_break_index) = pending_chunk.find('\n') {
            let line = pending_chunk[..line_break_index].trim().to_string();
            pending_chunk = pending_chunk[line_break_index + 1..].to_string();
            if let Some(finish_reason) = handle_stream_line(app, &request.request_id, &line)? {
                return Ok(StreamCompletion::Done(Some(finish_reason)));
            }
        }
    }

    Ok(StreamCompletion::Done(None))
}

fn request_registry(app: &AppHandle) -> tauri::State<'_, LlmRequestRegistry> {
    app.state::<LlmRequestRegistry>()
}

impl LlmRequestRegistry {
    fn cancel(&self, request_id: String) -> Result<(), String> {
        self.cancelled_request_ids
            .lock()
            .map_err(|error| error.to_string())?
            .insert(request_id);
        Ok(())
    }

    fn is_cancelled(&self, request_id: &str) -> Result<bool, String> {
        Ok(self
            .cancelled_request_ids
            .lock()
            .map_err(|error| error.to_string())?
            .contains(request_id))
    }

    fn clear_cancelled(&self, request_id: &str) -> Result<(), String> {
        self.cancelled_request_ids
            .lock()
            .map_err(|error| error.to_string())?
            .remove(request_id);
        Ok(())
    }
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
        "content": "You are Waey, a concise screen-aware desktop assistant. Answer directly using the user's screen context when an image is attached. If developer workspace context is attached in the user message, treat it as visible local file context that you can read, even when no image is attached. Do not ask the user to upload or resend a file that is already included in developer context. For requested file edits, return a fenced `waey-edit` block with `path: ABSOLUTE_FILE_PATH` on the first line and the full replacement file content after it. The `waey-edit` block must not contain explanations, partial snippets, or markdown around the replacement. Wrap ordinary code, terminal commands, and config snippets in fenced Markdown code blocks."
    })];

    if let Some(persona_prompt) = persona_system_message(request) {
        messages.push(persona_prompt);
    }

    messages.extend(history_messages(request));
    messages.push(json!({
        "role": "user",
        "content": user_message_content(request)?
    }));

    let mut body = json!({
        "model": request.provider.model,
        "stream": true,
        "messages": messages
    });

    if uses_groq_qwen_reasoning(&request.provider) {
        body["max_completion_tokens"] = json!(groq_qwen_max_completion_tokens(&request.provider));
        body["reasoning_format"] = json!("parsed");
        body["reasoning_effort"] = json!(groq_qwen_reasoning_effort(&request.provider));
    }

    Ok(body)
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
    let capture_paths = normalized_capture_paths(request);
    let prompt = prompt_with_ui_context(request);

    if capture_paths.is_empty() {
        return Ok(json!(prompt));
    };

    let mut content = vec![json!({
        "type": "text",
        "text": prompt
    })];

    for capture_path in capture_paths {
        content.push(json!({
            "type": "image_url",
            "image_url": {
                "url": image_data_url(capture_path)?
            }
        }));
    }

    Ok(json!(content))
}

fn prompt_with_ui_context(request: &LlmChatRequest) -> String {
    let Some(contexts) = request.ui_contexts.as_ref() else {
        return request.prompt.clone();
    };

    let formatted_contexts = contexts
        .iter()
        .take(3)
        .enumerate()
        .filter_map(|(index, context)| format_ui_context(index + 1, context))
        .collect::<Vec<_>>();

    if formatted_contexts.is_empty() {
        return request.prompt.clone();
    }

    format!(
        "{}\n\nReadable screen structure captured with the screenshot:\n{}",
        request.prompt,
        formatted_contexts.join("\n\n")
    )
}

fn format_ui_context(index: usize, context: &UiContextSnapshot) -> Option<String> {
    if context.elements.is_empty()
        && context
            .active_window_title
            .as_deref()
            .unwrap_or("")
            .trim()
            .is_empty()
    {
        return None;
    }

    let mut lines = vec![format!("Screenshot {index}:")];

    if let Some(title) = non_empty(context.active_window_title.as_deref()) {
        lines.push(format!("- Active window: {title}"));
    }

    if let Some(app_name) = non_empty(context.active_app_name.as_deref()) {
        lines.push(format!("- App: {app_name}"));
    }

    if let Some(region) = &context.region {
        lines.push(format!(
            "- Selected region: x={}, y={}, width={}, height={}",
            region.x, region.y, region.width, region.height
        ));
    }

    if !context.elements.is_empty() {
        lines.push("- Visible UI elements:".to_string());

        for (element_index, element) in context.elements.iter().take(80).enumerate() {
            let mut flags = Vec::new();

            if element.focused {
                flags.push("focused");
            }

            if element.under_cursor {
                flags.push("under cursor");
            }

            let name = non_empty(Some(&element.name)).unwrap_or("(unnamed)");
            let value = non_empty(element.value.as_deref())
                .map(|value| format!(" value=\"{value}\""))
                .unwrap_or_default();
            let selected_text = non_empty(element.selected_text.as_deref())
                .map(|selected_text| format!(" selected=\"{selected_text}\""))
                .unwrap_or_default();
            let automation_id = non_empty(element.automation_id.as_deref())
                .map(|automation_id| format!(" id=\"{automation_id}\""))
                .unwrap_or_default();
            let flags = if flags.is_empty() {
                String::new()
            } else {
                format!(" [{}]", flags.join(", "))
            };

            lines.push(format!(
                "  {}. {} \"{}\"{}{}{} at x={}, y={}, w={}, h={}{}",
                element_index + 1,
                element.role,
                name,
                value,
                selected_text,
                automation_id,
                element.bounds.x,
                element.bounds.y,
                element.bounds.width,
                element.bounds.height,
                flags
            ));
        }
    }

    Some(lines.join("\n"))
}

fn non_empty(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}

fn normalized_capture_paths(request: &LlmChatRequest) -> Vec<&str> {
    let mut paths = request
        .capture_paths
        .as_deref()
        .unwrap_or_default()
        .iter()
        .map(|path| path.trim())
        .filter(|path| !path.is_empty())
        .collect::<Vec<_>>();

    if paths.is_empty() {
        if let Some(capture_path) = request.capture_path.as_deref() {
            if !capture_path.trim().is_empty() {
                paths.push(capture_path.trim());
            }
        }
    }

    paths.truncate(3);
    paths
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

fn handle_stream_line(
    app: &AppHandle,
    request_id: &str,
    line: &str,
) -> Result<Option<String>, String> {
    if !line.starts_with("data:") {
        return Ok(None);
    }

    let data = line.trim_start_matches("data:").trim();

    if data == "[DONE]" {
        return Ok(None);
    }

    let value: Value = serde_json::from_str(data).map_err(|error| error.to_string())?;
    let choice = &value["choices"][0];
    let delta = &choice["delta"];

    if let Some(reasoning) = reasoning_token(delta) {
        emit_stream_token(app, STREAM_REASONING_EVENT, request_id, reasoning)?;
    }

    if let Some(token) = delta["content"].as_str() {
        emit_stream_token(app, STREAM_TOKEN_EVENT, request_id, token)?;
    }

    Ok(choice["finish_reason"].as_str().map(ToString::to_string))
}

fn emit_stream_token(
    app: &AppHandle,
    event_name: &str,
    request_id: &str,
    token: &str,
) -> Result<(), String> {
    app.emit(
        event_name,
        StreamToken {
            request_id: request_id.to_string(),
            token: token.to_string(),
        },
    )
    .map_err(|error| error.to_string())
}

fn reasoning_token(delta: &Value) -> Option<&str> {
    delta["reasoning"]
        .as_str()
        .or_else(|| delta["reasoning_content"].as_str())
        .or_else(|| delta["reasoningContent"].as_str())
}

fn uses_groq_qwen_reasoning(provider: &LlmProvider) -> bool {
    provider.base_url.contains("api.groq.com") && provider.model.starts_with("qwen/")
}

fn groq_qwen_max_completion_tokens(provider: &LlmProvider) -> u32 {
    if provider.managed {
        MANAGED_GROQ_QWEN_MAX_COMPLETION_TOKENS
    } else {
        CUSTOM_GROQ_QWEN_MAX_COMPLETION_TOKENS
    }
}

fn groq_qwen_reasoning_effort(provider: &LlmProvider) -> &'static str {
    if provider.managed {
        "none"
    } else {
        "default"
    }
}

fn history_role_to_string(role: &LlmHistoryRole) -> &'static str {
    match role {
        LlmHistoryRole::User => "user",
        LlmHistoryRole::Assistant => "assistant",
        LlmHistoryRole::System => "system",
    }
}
