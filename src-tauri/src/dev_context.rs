use crate::settings::{get_settings, DeveloperAccessLevel};
use crate::spreadsheet;
use crate::ui_context::UiContextSnapshot;
use crate::workspace::{
    atomic_write_text, is_protected_path, looks_like_text_file, should_skip_workspace_path,
    WorkspaceRegistry,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::VecDeque;
use std::fs;
use std::path::{Path, PathBuf};
use tauri::AppHandle;

const MAX_SEARCH_ENTRIES: usize = 8_000;
const MAX_ATTACHED_LINES: usize = 220;
const MAX_ATTACHED_BYTES: usize = 28_000;
const CONTEXT_RADIUS_LINES: usize = 110;
const MAX_REPOSITORY_MAP_ENTRIES: usize = 72;
const MAX_REPOSITORY_MAP_SCAN_ENTRIES: usize = 900;

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
    pub file_path: Option<String>,
    pub active_window_title: Option<String>,
    pub line_range: Option<DeveloperLineRange>,
    pub warnings: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum DeveloperContextStatusKind {
    Attached,
    Warning,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeveloperLineRange {
    pub start: usize,
    pub end: usize,
    pub total: usize,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeveloperFileWriteRequest {
    pub workspace: Option<String>,
    pub path: String,
    pub content: String,
    pub expected_sha256: Option<String>,
    pub operation: DeveloperFileOperation,
    #[serde(default)]
    pub overwrite: bool,
    pub approved: bool,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DeveloperFileOperation {
    Edit,
    Create,
}

#[tauri::command]
pub async fn build_developer_context(
    app: AppHandle,
    request: DeveloperContextRequest,
) -> Result<Option<DeveloperContextResponse>, String> {
    tauri::async_runtime::spawn_blocking(move || build_developer_context_sync(&app, request))
        .await
        .map_err(|error| format!("Developer context worker failed: {error}"))?
}

fn build_developer_context_sync(
    app: &AppHandle,
    request: DeveloperContextRequest,
) -> Result<Option<DeveloperContextResponse>, String> {
    let settings = get_settings(app)?;

    if !settings.developer_mode_enabled || settings.developer_workspaces.is_empty() {
        return Ok(None);
    }

    if matches!(settings.developer_access_level, DeveloperAccessLevel::Ask) && !request.approved {
        return Err("Developer mode needs approval before reading workspace files.".to_string());
    }

    let workspace_registry = WorkspaceRegistry::from_settings(&settings.developer_workspaces)?;
    let workspaces = workspace_registry.roots();

    if workspace_registry.is_empty() {
        return Ok(None);
    }

    let mut warnings = Vec::new();
    let repository_map = workspace_repository_map(workspaces);
    let active_title = active_window_title(&request.ui_contexts);
    let selected_text = selected_text_from_contexts(&request.ui_contexts);
    let requested_line = requested_line_number(&request.prompt);
    let candidates = file_candidates(
        &request.prompt,
        active_title.as_deref(),
        &request.ui_contexts,
    );

    if candidates.is_empty() {
        return Ok(Some(DeveloperContextResponse {
            content: developer_context_header(
                active_title.as_deref(),
                None,
                None,
                None,
                selected_text.as_deref(),
                &warnings,
                &repository_map,
            ),
            file_path: None,
            status: developer_status(
                "No file matched",
                "Waey could not detect an active code file from this screen.",
                DeveloperContextStatusKind::Warning,
                None,
                active_title.as_deref(),
                None,
                &warnings,
            ),
            warnings,
        }));
    };

    let mut matched_path = None;
    let mut matched_candidate = None;

    for candidate in &candidates {
        if let Some(path) = find_file_in_workspaces(candidate, workspaces, &mut warnings)? {
            matched_candidate = Some(candidate.clone());
            matched_path = Some(path);
            break;
        }
    }

    let Some(matched_path) = matched_path else {
        warnings.push(format!(
            "Waey could not find any of these files inside the allowed workspaces: {}.",
            candidates.join(", ")
        ));

        return Ok(Some(DeveloperContextResponse {
            content: developer_context_header(
                active_title.as_deref(),
                None,
                None,
                None,
                selected_text.as_deref(),
                &warnings,
                &repository_map,
            ),
            file_path: None,
            status: developer_status(
                "No workspace file matched",
                "Could not find a requested code file inside allowed workspaces.",
                DeveloperContextStatusKind::Warning,
                None,
                active_title.as_deref(),
                None,
                &warnings,
            ),
            warnings,
        }));
    };

    if candidates.len() > 1 {
        warnings.push(format!(
            "Waey detected multiple possible files and attached `{}`.",
            matched_candidate.unwrap_or_else(|| {
                matched_path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("workspace file")
                    .to_string()
            })
        ));
    }

    let workspace_root = workspace_registry
        .root_for_path(&matched_path)
        .ok_or_else(|| "Matched file is outside the approved workspace registry.".to_string())?;
    let workspace_relative_path = matched_path
        .strip_prefix(workspace_root)
        .map_err(|error| error.to_string())?;
    let content;
    let mut attachment = None;

    if is_spreadsheet_file(&matched_path) {
        let spreadsheet_summary = spreadsheet::summarize_workbook(&matched_path)?;
        content = format!(
            "{}\n\nMatched spreadsheet: {}\n\n```text\n{}\n```",
            developer_context_header(
                active_title.as_deref(),
                Some(workspace_root),
                Some(workspace_relative_path),
                None,
                selected_text.as_deref(),
                &warnings,
                &repository_map,
            ),
            workspace_relative_path.display(),
            spreadsheet_summary,
        );
    } else {
        let file_attachment =
            read_file_attachment(&matched_path, requested_line, selected_text.as_deref())?;
        content = format!(
            "{}\n\nMatched file: {}\nAttached lines: {}-{} of {}\n\n```{}\n{}\n```",
            developer_context_header(
                active_title.as_deref(),
                Some(workspace_root),
                Some(workspace_relative_path),
                Some(&file_attachment),
                selected_text.as_deref(),
                &warnings,
                &repository_map,
            ),
            workspace_relative_path.display(),
            file_attachment.start_line,
            file_attachment.end_line,
            file_attachment.total_lines,
            language_from_path(&matched_path),
            file_attachment.content
        );
        attachment = Some(file_attachment);
    }

    Ok(Some(DeveloperContextResponse {
        content,
        file_path: Some(matched_path.to_string_lossy().to_string()),
        status: developer_status(
            if is_spreadsheet_file(&matched_path) {
                "Spreadsheet context attached"
            } else {
                "Code context attached"
            },
            &developer_status_detail(&matched_path, attachment.as_ref()),
            DeveloperContextStatusKind::Attached,
            Some(&matched_path),
            active_title.as_deref(),
            attachment.as_ref(),
            &warnings,
        ),
        warnings,
    }))
}

#[tauri::command]
pub async fn write_developer_file(
    app: AppHandle,
    request: DeveloperFileWriteRequest,
) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || write_developer_file_sync(&app, request))
        .await
        .map_err(|error| format!("Developer file worker failed: {error}"))?
}

fn write_developer_file_sync(
    app: &AppHandle,
    request: DeveloperFileWriteRequest,
) -> Result<(), String> {
    let settings = get_settings(app)?;

    if !settings.developer_mode_enabled {
        return Err("Developer Mode is not enabled.".to_string());
    }

    if !matches!(settings.developer_access_level, DeveloperAccessLevel::Auto) && !request.approved {
        return Err("This file edit needs approval.".to_string());
    }

    if request.content.len() > 512_000 {
        return Err("Waey will not write files larger than 512 KB.".to_string());
    }

    let workspaces = WorkspaceRegistry::from_settings(&settings.developer_workspaces)?;
    let target = match &request.operation {
        DeveloperFileOperation::Edit => {
            workspaces.resolve_existing(request.workspace.as_deref(), &request.path)?
        }
        DeveloperFileOperation::Create => workspaces.resolve_write_target(
            request.workspace.as_deref(),
            &request.path,
            request.overwrite,
        )?,
    };

    if is_protected_path(&target.path) {
        return Err("Waey will not edit protected or secret-like files.".to_string());
    }

    if !looks_like_text_file(&target.path) {
        return Err("Waey can only write developer text/code files.".to_string());
    }

    if matches!(&request.operation, DeveloperFileOperation::Edit) {
        let expected_hash = request
            .expected_sha256
            .as_deref()
            .filter(|hash| !hash.trim().is_empty())
            .ok_or_else(|| "Existing file edits must include the current file hash.".to_string())?;
        let current_content = fs::read(&target.path).map_err(|error| error.to_string())?;

        if sha256_hex(&current_content) != expected_hash.trim().to_ascii_lowercase() {
            return Err("This file changed after Waey read it. Refresh the context before applying an edit.".to_string());
        }
    }

    atomic_write_text(&target.path, &request.content)
}

fn active_window_title(contexts: &[UiContextSnapshot]) -> Option<String> {
    contexts.iter().find_map(|context| {
        non_empty(context.active_window_title.as_deref()).map(ToString::to_string)
    })
}

fn selected_text_from_contexts(contexts: &[UiContextSnapshot]) -> Option<String> {
    contexts
        .iter()
        .find_map(|context| non_empty(context.selected_text.as_deref()))
        .map(ToString::to_string)
        .or_else(|| {
            contexts
                .iter()
                .flat_map(|context| context.elements.iter())
                .find(|element| element.focused || element.under_cursor)
                .and_then(|element| non_empty(element.selected_text.as_deref()))
                .map(ToString::to_string)
        })
        .or_else(|| {
            contexts
                .iter()
                .flat_map(|context| context.elements.iter())
                .find_map(|element| non_empty(element.selected_text.as_deref()))
                .map(ToString::to_string)
        })
}

fn file_candidates(
    prompt: &str,
    title: Option<&str>,
    contexts: &[UiContextSnapshot],
) -> Vec<String> {
    let mut candidates = Vec::new();

    push_candidates(&mut candidates, prompt);

    if let Some(title) = title {
        push_candidates(&mut candidates, title);
    }

    for element in contexts.iter().flat_map(|context| context.elements.iter()) {
        if element.focused || element.under_cursor {
            push_candidates(&mut candidates, &element.name);
            push_candidates(&mut candidates, element.value.as_deref().unwrap_or(""));
            push_candidates(
                &mut candidates,
                element.selected_text.as_deref().unwrap_or(""),
            );
        }
    }

    for element in contexts.iter().flat_map(|context| context.elements.iter()) {
        push_candidates(&mut candidates, &element.name);
        push_candidates(&mut candidates, element.value.as_deref().unwrap_or(""));
        push_candidates(
            &mut candidates,
            element.selected_text.as_deref().unwrap_or(""),
        );
    }

    candidates.truncate(8);
    candidates
}

fn push_candidates(candidates: &mut Vec<String>, text: &str) {
    for candidate in candidates_from_text(text) {
        if !candidates
            .iter()
            .any(|existing| existing.eq_ignore_ascii_case(&candidate))
        {
            candidates.push(candidate);
        }
    }
}

fn candidates_from_text(text: &str) -> Vec<String> {
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

    let mut candidates = Vec::new();

    for part in cleaned
        .split([
            '\n', '\t', '"', '\'', '`', '|', '<', '>', '[', ']', '(', ')',
        ])
        .flat_map(|part| part.split_whitespace())
        .flat_map(|part| part.split(" - "))
        .map(clean_candidate_token)
        .filter(|part| looks_like_workspace_file(part))
    {
        if !candidates
            .iter()
            .any(|candidate: &String| candidate.eq_ignore_ascii_case(&part))
        {
            candidates.push(part);
        }
    }

    candidates
}

fn clean_candidate_token(value: &str) -> String {
    value
        .trim()
        .trim_matches(|character: char| {
            matches!(
                character,
                ',' | ';' | ':' | '.' | '!' | '?' | '"' | '\'' | '`'
            )
        })
        .to_string()
}

fn looks_like_workspace_file(value: &str) -> bool {
    looks_like_text_file(Path::new(value))
        || Path::new(value)
            .extension()
            .and_then(|extension| extension.to_str())
            .map(|extension| matches!(extension.to_ascii_lowercase().as_str(), "xlsx"))
            .unwrap_or(false)
}

fn is_spreadsheet_file(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| extension.eq_ignore_ascii_case("xlsx"))
        .unwrap_or(false)
}

fn find_file_in_workspaces(
    file_name: &str,
    workspaces: &[PathBuf],
    warnings: &mut Vec<String>,
) -> Result<Option<PathBuf>, String> {
    let normalized_candidate = file_name.replace('\\', "/").to_ascii_lowercase();
    let target_name = Path::new(file_name)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(file_name)
        .to_ascii_lowercase();
    let mut matches: Vec<(usize, PathBuf)> = Vec::new();
    let mut scanned = 0usize;

    for workspace in workspaces {
        let mut queue = VecDeque::from([workspace.clone()]);

        while let Some(path) = queue.pop_front() {
            if scanned >= MAX_SEARCH_ENTRIES {
                warnings
                    .push("Workspace search stopped early to keep Waey responsive.".to_string());
                matches.sort_by_key(|(score, path)| (*score, path.components().count()));
                return Ok(matches.into_iter().map(|(_, path)| path).next());
            }

            scanned += 1;

            let canonical_path = match fs::canonicalize(&path) {
                Ok(path) if path.starts_with(workspace) && !is_protected_path(&path) => path,
                _ => continue,
            };
            let relative_path = canonical_path
                .strip_prefix(workspace)
                .map_err(|error| error.to_string())?;

            if !relative_path.as_os_str().is_empty() && should_skip_workspace_path(relative_path) {
                continue;
            }

            let metadata = match fs::metadata(&canonical_path) {
                Ok(metadata) => metadata,
                Err(_) => continue,
            };

            if metadata.is_dir() {
                if let Ok(entries) = fs::read_dir(&canonical_path) {
                    for entry in entries.flatten() {
                        queue.push_back(entry.path());
                    }
                }
                continue;
            }

            let Some(name) = canonical_path.file_name().and_then(|value| value.to_str()) else {
                continue;
            };

            let relative_path = relative_path
                .to_string_lossy()
                .replace('\\', "/")
                .to_ascii_lowercase();

            if relative_path.ends_with(&normalized_candidate) {
                matches.push((0, canonical_path));
            } else if name.eq_ignore_ascii_case(&target_name) {
                matches.push((1, canonical_path));
            }
        }
    }

    if matches.len() > 1 {
        warnings.push(format!(
            "Waey found {} files named `{file_name}` and used the shortest path.",
            matches.len()
        ));
    }

    matches.sort_by_key(|(score, path)| (*score, path.components().count()));
    Ok(matches.into_iter().map(|(_, path)| path).next())
}

struct FileAttachment {
    content: String,
    sha256: String,
    start_line: usize,
    end_line: usize,
    total_lines: usize,
}

fn read_file_attachment(
    path: &Path,
    requested_line: Option<usize>,
    selected_text: Option<&str>,
) -> Result<FileAttachment, String> {
    if is_protected_path(path) {
        return Err("Waey will not read protected or secret-like files.".to_string());
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
        sha256: sha256_hex(text.as_bytes()),
        start_line: start_index + 1,
        end_line: end_line.max(start_index + 1),
        total_lines,
    })
}

fn developer_context_header(
    active_title: Option<&str>,
    workspace_root: Option<&Path>,
    file_path: Option<&Path>,
    attachment: Option<&FileAttachment>,
    selected_text: Option<&str>,
    warnings: &[String],
    repository_map: &[String],
) -> String {
    let mut lines = vec!["Developer workspace context attached.".to_string()];

    if let Some(title) = non_empty(active_title) {
        lines.push(format!("Active window: {title}"));
    }

    if let Some(root) = workspace_root {
        lines.push(format!("Workspace root: {}", root.display()));
    }

    if let Some(path) = file_path {
        lines.push(format!("Workspace-relative file: {}", path.display()));
    }

    if let Some(attachment) = attachment {
        lines.push(format!(
            "Attached lines: {}-{} of {}",
            attachment.start_line, attachment.end_line, attachment.total_lines
        ));
        lines.push(format!("Current SHA-256: {}", attachment.sha256));
    }

    if let Some(selected_text) = non_empty(selected_text) {
        lines.push(format!("Focused or selected text: {selected_text}"));
    }

    if !warnings.is_empty() {
        lines.push("Warnings:".to_string());
        lines.extend(warnings.iter().map(|warning| format!("- {warning}")));
    }

    if !repository_map.is_empty() {
        lines.push("Workspace map:".to_string());
        lines.extend(repository_map.iter().map(|entry| format!("- {entry}")));
    }

    lines.join("\n")
}

fn workspace_repository_map(workspaces: &[PathBuf]) -> Vec<String> {
    let mut entries = Vec::new();
    let mut scanned = 0usize;

    for workspace in workspaces {
        let mut queue = VecDeque::from([workspace.clone()]);

        while let Some(path) = queue.pop_front() {
            if scanned >= MAX_REPOSITORY_MAP_SCAN_ENTRIES
                || entries.len() >= MAX_REPOSITORY_MAP_ENTRIES
            {
                return entries;
            }

            scanned += 1;

            let canonical_path = match fs::canonicalize(&path) {
                Ok(path) if path.starts_with(workspace) && !is_protected_path(&path) => path,
                _ => continue,
            };
            let relative_path = match canonical_path.strip_prefix(workspace) {
                Ok(path) if !path.as_os_str().is_empty() => path,
                _ => continue,
            };

            if should_skip_workspace_path(relative_path) {
                continue;
            }

            let metadata = match fs::metadata(&canonical_path) {
                Ok(metadata) => metadata,
                Err(_) => continue,
            };

            if metadata.is_dir() {
                if let Ok(children) = fs::read_dir(&canonical_path) {
                    for child in children.flatten() {
                        queue.push_back(child.path());
                    }
                }

                if relative_path.components().count() <= 2 {
                    entries.push(format!("{}/", relative_path.display()));
                }
                continue;
            }

            if looks_like_workspace_file(&relative_path.to_string_lossy())
                || matches!(
                    relative_path.file_name().and_then(|name| name.to_str()),
                    Some("package.json" | "Cargo.toml" | "README.md" | "pyproject.toml" | "go.mod")
                )
            {
                entries.push(relative_path.to_string_lossy().replace('\\', "/"));
            }
        }
    }

    entries
}

fn developer_status(
    label: &str,
    detail: &str,
    kind: DeveloperContextStatusKind,
    file_path: Option<&Path>,
    active_window_title: Option<&str>,
    attachment: Option<&FileAttachment>,
    warnings: &[String],
) -> DeveloperContextStatus {
    DeveloperContextStatus {
        label: label.to_string(),
        detail: detail.to_string(),
        kind,
        file_path: file_path.map(|path| path.to_string_lossy().to_string()),
        active_window_title: active_window_title.map(ToString::to_string),
        line_range: attachment.map(|attachment| DeveloperLineRange {
            start: attachment.start_line,
            end: attachment.end_line,
            total: attachment.total_lines,
        }),
        warnings: warnings.to_vec(),
    }
}

fn developer_status_detail(path: &Path, attachment: Option<&FileAttachment>) -> String {
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("workspace file");

    if let Some(attachment) = attachment {
        return format!(
            "{} lines {}-{}",
            file_name, attachment.start_line, attachment.end_line
        );
    }

    format!("{file_name} workbook summary")
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

fn sha256_hex(value: &[u8]) -> String {
    let digest = Sha256::digest(value);
    format!("{digest:x}")
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
        "vue" => "vue",
        "svelte" => "svelte",
        "astro" => "astro",
        "php" => "php",
        "rb" => "ruby",
        "swift" => "swift",
        "kt" | "kts" => "kotlin",
        "dart" => "dart",
        "sh" => "bash",
        "ps1" => "powershell",
        "go" => "go",
        "java" => "java",
        "cs" => "csharp",
        "cpp" | "cxx" | "cc" => "cpp",
        "c" | "h" => "c",
        "hpp" | "hh" => "cpp",
        "lua" => "lua",
        "r" => "r",
        "ex" | "exs" => "elixir",
        "scala" => "scala",
        "pl" | "pm" => "perl",
        "fs" | "fsx" => "fsharp",
        "clj" | "cljs" => "clojure",
        "erl" | "hrl" => "erlang",
        "zig" => "zig",
        _ => "",
    }
}

fn non_empty(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}
