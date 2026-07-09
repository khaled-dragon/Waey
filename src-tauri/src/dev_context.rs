use crate::settings::{get_settings, DeveloperAccessLevel};
use crate::ui_context::UiContextSnapshot;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::fs;
use std::path::{Component, Path, PathBuf};
use tauri::AppHandle;

const MAX_SEARCH_ENTRIES: usize = 8_000;
const MAX_ATTACHED_LINES: usize = 220;
const MAX_ATTACHED_BYTES: usize = 28_000;
const CONTEXT_RADIUS_LINES: usize = 110;

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeveloperContextRequest {
    pub prompt: String,
    pub ui_contexts: Vec<UiContextSnapshot>,
    pub approved: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeveloperContextResponse {
    pub content: String,
    pub file_path: Option<String>,
    pub status: DeveloperContextStatus,
    pub warnings: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeveloperContextStatus {
    pub label: String,
    pub detail: String,
    pub kind: DeveloperContextStatusKind,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum DeveloperContextStatusKind {
    Attached,
    Warning,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeveloperFileWriteRequest {
    pub path: String,
    pub content: String,
    pub approved: bool,
}

#[tauri::command]
pub fn build_developer_context(
    app: AppHandle,
    request: DeveloperContextRequest,
) -> Result<Option<DeveloperContextResponse>, String> {
    let settings = get_settings(&app)?;

    if !settings.developer_mode_enabled || settings.developer_workspaces.is_empty() {
        return Ok(None);
    }

    if matches!(settings.developer_access_level, DeveloperAccessLevel::Ask) && !request.approved {
        return Err("Developer mode needs approval before reading workspace files.".to_string());
    }

    let workspaces = normalized_workspaces(&settings.developer_workspaces)?;

    if workspaces.is_empty() {
        return Ok(None);
    }

    let mut warnings = Vec::new();
    let active_title = active_window_title(&request.ui_contexts);
    let selected_text = selected_text_from_contexts(&request.ui_contexts);
    let requested_line = requested_line_number(&request.prompt);
    let candidate_name = active_file_candidate(
        &request.prompt,
        active_title.as_deref(),
        &request.ui_contexts,
    );

    let Some(candidate_name) = candidate_name else {
        return Ok(Some(DeveloperContextResponse {
            content: developer_context_header(
                active_title.as_deref(),
                None,
                selected_text.as_deref(),
                &warnings,
            ),
            file_path: None,
            status: DeveloperContextStatus {
                label: "No file matched".to_string(),
                detail: "Waey could not detect an active code file from this screen.".to_string(),
                kind: DeveloperContextStatusKind::Warning,
            },
            warnings,
        }));
    };

    let matched_path = find_file_in_workspaces(&candidate_name, &workspaces, &mut warnings)?;
    let Some(matched_path) = matched_path else {
        warnings.push(format!(
            "Waey could not find `{candidate_name}` inside the allowed workspaces."
        ));

        return Ok(Some(DeveloperContextResponse {
            content: developer_context_header(
                active_title.as_deref(),
                None,
                selected_text.as_deref(),
                &warnings,
            ),
            file_path: None,
            status: DeveloperContextStatus {
                label: "No workspace file matched".to_string(),
                detail: format!("Could not find `{candidate_name}` inside allowed workspaces."),
                kind: DeveloperContextStatusKind::Warning,
            },
            warnings,
        }));
    };

    let attachment = read_file_attachment(&matched_path, requested_line, selected_text.as_deref())?;
    let content = format!(
        "{}\n\nMatched file: {}\nAttached lines: {}-{} of {}\n\n```{}\n{}\n```",
        developer_context_header(
            active_title.as_deref(),
            Some(&matched_path),
            selected_text.as_deref(),
            &warnings,
        ),
        matched_path.display(),
        attachment.start_line,
        attachment.end_line,
        attachment.total_lines,
        language_from_path(&matched_path),
        attachment.content
    );

    Ok(Some(DeveloperContextResponse {
        content,
        file_path: Some(matched_path.to_string_lossy().to_string()),
        status: DeveloperContextStatus {
            label: "Code context attached".to_string(),
            detail: format!(
                "{} lines {}-{}",
                matched_path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("workspace file"),
                attachment.start_line,
                attachment.end_line
            ),
            kind: DeveloperContextStatusKind::Attached,
        },
        warnings,
    }))
}

#[tauri::command]
pub fn write_developer_file(
    app: AppHandle,
    request: DeveloperFileWriteRequest,
) -> Result<(), String> {
    let settings = get_settings(&app)?;

    if !settings.developer_mode_enabled {
        return Err("Developer Mode is not enabled.".to_string());
    }

    if !matches!(settings.developer_access_level, DeveloperAccessLevel::Auto) && !request.approved {
        return Err("This file edit needs approval.".to_string());
    }

    if request.content.len() > 512_000 {
        return Err("Waey will not write files larger than 512 KB.".to_string());
    }

    let workspaces = normalized_workspaces(&settings.developer_workspaces)?;
    let target_path = fs::canonicalize(clean_path_input(&request.path))
        .map_err(|error| format!("Failed to resolve target file: {error}"))?;

    if !target_path.is_file() {
        return Err("Waey can only edit existing files.".to_string());
    }

    if is_secret_path(&target_path) {
        return Err("Waey will not edit secret-like files.".to_string());
    }

    if !workspaces
        .iter()
        .any(|workspace| target_path.starts_with(workspace))
    {
        return Err("Target file is outside the allowed workspaces.".to_string());
    }

    fs::write(&target_path, request.content).map_err(|error| error.to_string())
}

fn normalized_workspaces(values: &[String]) -> Result<Vec<PathBuf>, String> {
    let mut workspaces = Vec::new();

    for value in values {
        let trimmed = clean_path_input(value);

        if trimmed.is_empty() {
            continue;
        }

        let path = fs::canonicalize(&trimmed)
            .map_err(|error| format!("Failed to read workspace `{trimmed}`: {error}"))?;

        if path.is_dir() && !workspaces.iter().any(|workspace| workspace == &path) {
            workspaces.push(path);
        }
    }

    Ok(workspaces)
}

fn clean_path_input(value: &str) -> String {
    value
        .trim()
        .trim_matches('"')
        .trim_matches('\'')
        .trim()
        .to_string()
}

fn active_window_title(contexts: &[UiContextSnapshot]) -> Option<String> {
    contexts.iter().find_map(|context| {
        non_empty(context.active_window_title.as_deref()).map(ToString::to_string)
    })
}

fn selected_text_from_contexts(contexts: &[UiContextSnapshot]) -> Option<String> {
    contexts
        .iter()
        .flat_map(|context| context.elements.iter())
        .find(|element| element.focused || element.under_cursor)
        .and_then(|element| non_empty(element.selected_text.as_deref()))
        .map(ToString::to_string)
        .or_else(|| {
            contexts
                .iter()
                .flat_map(|context| context.elements.iter())
                .find_map(|element| non_empty(element.selected_text.as_deref()))
                .map(ToString::to_string)
        })
}

fn active_file_candidate(
    prompt: &str,
    title: Option<&str>,
    contexts: &[UiContextSnapshot],
) -> Option<String> {
    title
        .and_then(candidate_from_text)
        .or_else(|| {
            contexts
                .iter()
                .flat_map(|context| context.elements.iter())
                .filter_map(|element| {
                    candidate_from_text(&element.name)
                        .or_else(|| candidate_from_text(element.value.as_deref().unwrap_or("")))
                        .or_else(|| {
                            candidate_from_text(element.selected_text.as_deref().unwrap_or(""))
                        })
                })
                .next()
        })
        .or_else(|| candidate_from_text(prompt))
}

fn candidate_from_text(text: &str) -> Option<String> {
    let separators = [
        " - Visual Studio Code",
        " - Cursor",
        " - Code",
        " - Windsurf",
    ];
    let mut cleaned = text.trim().to_string();

    for separator in separators {
        if let Some(index) = cleaned.rfind(separator) {
            cleaned.truncate(index);
        }
    }

    cleaned
        .split([' ', '\n', '\t', '"', '\'', '`', ':', '|', '/', '\\'])
        .map(|part| {
            part.trim_matches(|character: char| {
                character == ',' || character == ';' || character == ')' || character == '('
            })
        })
        .find(|part| looks_like_code_file(part))
        .map(ToString::to_string)
        .or_else(|| {
            cleaned
                .split(" - ")
                .map(str::trim)
                .find(|part| looks_like_code_file(part))
                .map(ToString::to_string)
        })
}

fn looks_like_code_file(value: &str) -> bool {
    let Some(extension) = Path::new(value)
        .extension()
        .and_then(|extension| extension.to_str())
    else {
        return false;
    };

    matches!(
        extension.to_ascii_lowercase().as_str(),
        "rs" | "ts"
            | "tsx"
            | "js"
            | "jsx"
            | "json"
            | "css"
            | "html"
            | "md"
            | "py"
            | "go"
            | "java"
            | "cs"
            | "cpp"
            | "c"
            | "h"
            | "hpp"
            | "toml"
            | "yaml"
            | "yml"
            | "sql"
    )
}

fn find_file_in_workspaces(
    file_name: &str,
    workspaces: &[PathBuf],
    warnings: &mut Vec<String>,
) -> Result<Option<PathBuf>, String> {
    let target_name = Path::new(file_name)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(file_name)
        .to_ascii_lowercase();
    let mut matches = Vec::new();
    let mut scanned = 0usize;

    for workspace in workspaces {
        let mut queue = VecDeque::from([workspace.clone()]);

        while let Some(path) = queue.pop_front() {
            if scanned >= MAX_SEARCH_ENTRIES {
                warnings
                    .push("Workspace search stopped early to keep Waey responsive.".to_string());
                return Ok(matches.into_iter().next());
            }

            scanned += 1;

            if should_skip_path(&path) {
                continue;
            }

            let metadata = match fs::metadata(&path) {
                Ok(metadata) => metadata,
                Err(_) => continue,
            };

            if metadata.is_dir() {
                if let Ok(entries) = fs::read_dir(&path) {
                    for entry in entries.flatten() {
                        queue.push_back(entry.path());
                    }
                }
                continue;
            }

            let Some(name) = path.file_name().and_then(|value| value.to_str()) else {
                continue;
            };

            if name.eq_ignore_ascii_case(&target_name) && !is_secret_path(&path) {
                matches.push(path);
            }
        }
    }

    if matches.len() > 1 {
        warnings.push(format!(
            "Waey found {} files named `{file_name}` and used the shortest path.",
            matches.len()
        ));
    }

    matches.sort_by_key(|path| path.components().count());
    Ok(matches.into_iter().next())
}

struct FileAttachment {
    content: String,
    start_line: usize,
    end_line: usize,
    total_lines: usize,
}

fn read_file_attachment(
    path: &Path,
    requested_line: Option<usize>,
    selected_text: Option<&str>,
) -> Result<FileAttachment, String> {
    if is_secret_path(path) {
        return Err("Waey will not read secret-like files.".to_string());
    }

    let metadata = fs::metadata(path).map_err(|error| error.to_string())?;

    if metadata.len() > 512_000 {
        return Err("This file is too large for quick Dev Mode context.".to_string());
    }

    let text = fs::read_to_string(path).map_err(|error| error.to_string())?;
    let lines = text.lines().collect::<Vec<_>>();
    let total_lines = lines.len().max(1);
    let focus_line = requested_line
        .filter(|line| *line > 0)
        .or_else(|| selected_text.and_then(|selected| find_selected_text_line(&lines, selected)));
    let start_index = focus_line
        .map(|line| line.saturating_sub(CONTEXT_RADIUS_LINES + 1))
        .unwrap_or(0);
    let mut content = String::new();
    let mut end_line = start_index;

    for (index, line) in lines
        .iter()
        .enumerate()
        .skip(start_index)
        .take(MAX_ATTACHED_LINES)
    {
        if content.len() + line.len() + 1 > MAX_ATTACHED_BYTES {
            break;
        }

        content.push_str(line);
        content.push('\n');
        end_line = index + 1;
    }

    Ok(FileAttachment {
        content: content.trim_end().to_string(),
        start_line: start_index + 1,
        end_line: end_line.max(start_index + 1),
        total_lines,
    })
}

fn developer_context_header(
    active_title: Option<&str>,
    file_path: Option<&Path>,
    selected_text: Option<&str>,
    warnings: &[String],
) -> String {
    let mut lines = vec![
        "Developer workspace context is attached below. You can read this file context even when no screenshot is attached.".to_string(),
        "Do not say you cannot see the file if a matched file and code block are included in this context.".to_string(),
        "Use this context as the source of truth for code questions. If the user asks for a code edit, return a concise answer plus a fenced `waey-edit` block.".to_string(),
        "A `waey-edit` block must contain a full replacement for the target file. The first line must be `path: ABSOLUTE_FILE_PATH`.".to_string(),
        "When returning `waey-edit`, do not put partial snippets, explanations, or markdown inside the block. Only the path line followed by the complete file content.".to_string(),
        "If the user asks to fix the selected line or a specific line number, use the focused text or attached line range to infer the exact change.".to_string(),
        "Waey may apply valid `waey-edit` blocks after checking workspace safety. Do not include destructive changes unless the user explicitly asked for them.".to_string(),
    ];

    if let Some(title) = non_empty(active_title) {
        lines.push(format!("Active window: {title}"));
    }

    if let Some(path) = file_path {
        lines.push(format!("Workspace file: {}", path.display()));
    }

    if let Some(selected_text) = non_empty(selected_text) {
        lines.push(format!("Focused or selected text: {selected_text}"));
    }

    if !warnings.is_empty() {
        lines.push("Warnings:".to_string());
        lines.extend(warnings.iter().map(|warning| format!("- {warning}")));
    }

    lines.join("\n")
}

fn requested_line_number(prompt: &str) -> Option<usize> {
    let lower_prompt = prompt.to_ascii_lowercase();
    let markers = ["line", "السطر", "سطر", "l"];

    for marker in markers {
        let Some(index) = lower_prompt.find(marker) else {
            continue;
        };
        let after_marker = &lower_prompt[index + marker.len()..];
        let digits = after_marker
            .chars()
            .skip_while(|character| !character.is_ascii_digit())
            .take_while(|character| character.is_ascii_digit())
            .collect::<String>();

        if let Ok(line) = digits.parse::<usize>() {
            return Some(line);
        }
    }

    None
}

fn find_selected_text_line(lines: &[&str], selected_text: &str) -> Option<usize> {
    let needle = selected_text.trim();

    if needle.is_empty() {
        return None;
    }

    lines
        .iter()
        .position(|line| line.contains(needle))
        .map(|index| index + 1)
}

fn should_skip_path(path: &Path) -> bool {
    path.components().any(|component| match component {
        Component::Normal(value) => {
            let value = value.to_string_lossy().to_ascii_lowercase();
            matches!(
                value.as_str(),
                ".git"
                    | "node_modules"
                    | "dist"
                    | "build"
                    | "target"
                    | ".next"
                    | ".turbo"
                    | "coverage"
                    | ".venv"
                    | "__pycache__"
            )
        }
        _ => false,
    })
}

fn is_secret_path(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|value| value.to_str()) else {
        return false;
    };

    let lower_name = name.to_ascii_lowercase();

    lower_name == ".env"
        || lower_name.starts_with(".env.")
        || lower_name.ends_with(".pem")
        || lower_name.ends_with(".key")
        || lower_name.ends_with(".pfx")
        || lower_name.ends_with(".p12")
        || lower_name.contains("secret")
        || lower_name.contains("credential")
}

fn language_from_path(path: &Path) -> &'static str {
    match path
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or("")
    {
        "rs" => "rust",
        "ts" => "ts",
        "tsx" => "tsx",
        "js" => "js",
        "jsx" => "jsx",
        "json" => "json",
        "css" => "css",
        "html" => "html",
        "py" => "python",
        "md" => "markdown",
        "toml" => "toml",
        "yml" | "yaml" => "yaml",
        "sql" => "sql",
        _ => "",
    }
}

fn non_empty(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}
