use crate::settings::{get_settings, DeveloperAccessLevel};
use serde::Deserialize;
use std::fs;
use std::path::{Component, Path, PathBuf};
use tauri::AppHandle;
use umya_spreadsheet::{new_file, reader, writer, Workbook, Worksheet};

const MAX_WORKBOOK_BYTES: u64 = 8 * 1024 * 1024;
const MAX_SUMMARY_SHEETS: usize = 5;
const MAX_SUMMARY_ROWS: u32 = 24;
const MAX_SUMMARY_COLUMNS: u32 = 12;
const MAX_SHEET_ACTIONS: usize = 200;
const MAX_APPEND_VALUES: usize = 128;

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeveloperSpreadsheetEditRequest {
    pub content: String,
    pub approved: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SpreadsheetEditPlan {
    path: String,
    actions: Vec<SpreadsheetAction>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
enum SpreadsheetAction {
    AddSheet {
        sheet: String,
    },
    SetCell {
        sheet: String,
        cell: String,
        value: CellInput,
    },
    SetFormula {
        sheet: String,
        cell: String,
        formula: String,
    },
    AppendRow {
        sheet: String,
        values: Vec<CellInput>,
    },
    ClearCell {
        sheet: String,
        cell: String,
    },
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum CellInput {
    Text(String),
    Number(f64),
    Bool(bool),
    Empty(()),
}

#[tauri::command]
pub fn apply_developer_spreadsheet_edit(
    app: AppHandle,
    request: DeveloperSpreadsheetEditRequest,
) -> Result<(), String> {
    let settings = get_settings(&app)?;

    if !settings.developer_mode_enabled {
        return Err("Developer Mode is not enabled.".to_string());
    }

    if !matches!(settings.developer_access_level, DeveloperAccessLevel::Auto) && !request.approved {
        return Err("This spreadsheet edit needs approval.".to_string());
    }

    let plan = parse_edit_plan(&request.content)?;

    if plan.actions.is_empty() {
        return Err("Spreadsheet edit has no actions.".to_string());
    }

    if plan.actions.len() > MAX_SHEET_ACTIONS {
        return Err(format!(
            "Spreadsheet edit supports up to {MAX_SHEET_ACTIONS} actions at once."
        ));
    }

    let workspaces = normalized_workspaces(&settings.developer_workspaces)?;
    let target_path =
        resolve_workbook_target(&PathBuf::from(clean_path_input(&plan.path)), &workspaces)?;

    if is_secret_path(&target_path) {
        return Err("Waey will not edit secret-like files.".to_string());
    }

    if !is_xlsx_path(&target_path) {
        return Err("Waey spreadsheet edits currently support .xlsx files only.".to_string());
    }

    if target_path.exists() {
        let metadata = fs::metadata(&target_path).map_err(|error| error.to_string())?;

        if metadata.len() > MAX_WORKBOOK_BYTES {
            return Err("This workbook is too large for safe quick edits.".to_string());
        }
    }

    let mut workbook = if target_path.exists() {
        reader::xlsx::read(&target_path)
            .map_err(|error| format!("Failed to read workbook: {error}"))?
    } else {
        new_file()
    };

    for action in plan.actions {
        apply_action(&mut workbook, action)?;
    }

    writer::xlsx::write(&workbook, &target_path)
        .map_err(|error| format!("Failed to write workbook: {error}"))
}

pub fn summarize_workbook(path: &Path) -> Result<String, String> {
    if !is_xlsx_path(path) {
        return Err("Waey can only summarize .xlsx spreadsheets.".to_string());
    }

    let metadata = fs::metadata(path).map_err(|error| error.to_string())?;

    if metadata.len() > MAX_WORKBOOK_BYTES {
        return Err("This workbook is too large for quick Dev Mode context.".to_string());
    }

    let workbook =
        reader::xlsx::read(path).map_err(|error| format!("Failed to read workbook: {error}"))?;
    let mut lines = vec![
        "Spreadsheet workbook context attached.".to_string(),
        format!("Workbook: {}", path.display()),
        "Waey can apply spreadsheet edits from a fenced `waey-sheet-edit` JSON block.".to_string(),
        "Supported actions: addSheet, setCell, setFormula, appendRow, clearCell.".to_string(),
    ];

    for sheet in workbook
        .get_sheet_collection()
        .iter()
        .take(MAX_SUMMARY_SHEETS)
    {
        let (max_column, max_row) = sheet.highest_column_and_row();
        lines.push(format!(
            "\nSheet `{}`: {} rows x {} columns",
            sheet.get_name(),
            max_row,
            max_column
        ));

        let row_limit = max_row.min(MAX_SUMMARY_ROWS);
        let column_limit = max_column.min(MAX_SUMMARY_COLUMNS);

        if row_limit == 0 || column_limit == 0 {
            lines.push("- empty sheet".to_string());
            continue;
        }

        for row in 1..=row_limit {
            let values = (1..=column_limit)
                .map(|column| {
                    let value = sheet.formatted_value((column, row));
                    if value.trim().is_empty() {
                        "".to_string()
                    } else {
                        value
                            .replace('\n', " ")
                            .replace('\r', " ")
                            .replace('\t', " ")
                    }
                })
                .collect::<Vec<_>>();

            if values.iter().any(|value| !value.trim().is_empty()) {
                lines.push(format!("R{row}: {}", values.join(" | ")));
            }
        }
    }

    Ok(lines.join("\n"))
}

fn parse_edit_plan(content: &str) -> Result<SpreadsheetEditPlan, String> {
    let plan = serde_json::from_str::<SpreadsheetEditPlan>(content.trim())
        .map_err(|error| format!("Invalid waey-sheet-edit JSON: {error}"))?;

    if plan.path.trim().is_empty() {
        return Err("Spreadsheet edit path is required.".to_string());
    }

    Ok(plan)
}

fn apply_action(workbook: &mut Workbook, action: SpreadsheetAction) -> Result<(), String> {
    match action {
        SpreadsheetAction::AddSheet { sheet } => {
            ensure_sheet(workbook, &sheet)?;
        }
        SpreadsheetAction::SetCell { sheet, cell, value } => {
            let cell = validate_cell(&cell)?;
            let worksheet = ensure_sheet(workbook, &sheet)?;
            set_cell_value_by_address(worksheet, &cell, value);
        }
        SpreadsheetAction::SetFormula {
            sheet,
            cell,
            formula,
        } => {
            let cell = validate_cell(&cell)?;
            let formula = formula.trim().trim_start_matches('=').trim();

            if formula.is_empty() {
                return Err("Formula cannot be empty.".to_string());
            }

            let worksheet = ensure_sheet(workbook, &sheet)?;
            worksheet.cell_mut(cell.as_str()).set_formula(formula);
        }
        SpreadsheetAction::AppendRow { sheet, values } => {
            if values.len() > MAX_APPEND_VALUES {
                return Err(format!(
                    "appendRow supports up to {MAX_APPEND_VALUES} values at once."
                ));
            }

            let worksheet = ensure_sheet(workbook, &sheet)?;
            let row = worksheet.highest_row().saturating_add(1).max(1);

            for (index, value) in values.into_iter().enumerate() {
                set_cell_value_by_position(worksheet, (index + 1) as u32, row, value);
            }
        }
        SpreadsheetAction::ClearCell { sheet, cell } => {
            let cell = validate_cell(&cell)?;
            let worksheet = ensure_sheet(workbook, &sheet)?;
            worksheet.remove_cell(cell.as_str());
        }
    }

    Ok(())
}

fn ensure_sheet<'a>(workbook: &'a mut Workbook, sheet: &str) -> Result<&'a mut Worksheet, String> {
    let sheet_name = sheet.trim();

    if sheet_name.is_empty() {
        return Err("Sheet name is required.".to_string());
    }

    if workbook.sheet_by_name(sheet_name).is_err() {
        workbook
            .new_sheet(sheet_name)
            .map_err(|error| format!("Failed to create sheet `{sheet_name}`: {error}"))?;
    }

    workbook
        .sheet_by_name_mut(sheet_name)
        .map_err(|error| format!("Failed to open sheet `{sheet_name}`: {error}"))
}

fn set_cell_value_by_address(worksheet: &mut Worksheet, cell_address: &str, value: CellInput) {
    let cell = worksheet.cell_mut(cell_address);

    match value {
        CellInput::Text(value) => {
            cell.set_value(value);
        }
        CellInput::Number(value) => {
            cell.set_value_number(value);
        }
        CellInput::Bool(value) => {
            cell.set_value_bool(value);
        }
        CellInput::Empty(_) => {
            cell.set_value("");
        }
    };
}

fn set_cell_value_by_position(worksheet: &mut Worksheet, column: u32, row: u32, value: CellInput) {
    let cell = worksheet.cell_mut((column, row));

    match value {
        CellInput::Text(value) => {
            cell.set_value(value);
        }
        CellInput::Number(value) => {
            cell.set_value_number(value);
        }
        CellInput::Bool(value) => {
            cell.set_value_bool(value);
        }
        CellInput::Empty(_) => {
            cell.set_value("");
        }
    };
}

fn resolve_workbook_target(
    requested_path: &Path,
    workspaces: &[PathBuf],
) -> Result<PathBuf, String> {
    if requested_path.as_os_str().is_empty() {
        return Err("Target workbook path is required.".to_string());
    }

    if requested_path.exists() {
        let target_path = fs::canonicalize(requested_path)
            .map_err(|error| format!("Failed to resolve workbook: {error}"))?;

        if !target_path.is_file() {
            return Err("Target workbook path is not a file.".to_string());
        }

        if !workspaces
            .iter()
            .any(|workspace| target_path.starts_with(workspace))
        {
            return Err("Target workbook is outside the allowed workspaces.".to_string());
        }

        return Ok(target_path);
    }

    let parent = requested_path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .ok_or_else(|| {
            "New workbooks must include a parent folder inside an allowed workspace.".to_string()
        })?;
    let parent = fs::canonicalize(parent)
        .map_err(|error| format!("Failed to resolve target folder: {error}"))?;

    if !parent.is_dir() {
        return Err("Target parent is not a folder.".to_string());
    }

    if !workspaces
        .iter()
        .any(|workspace| parent.starts_with(workspace))
    {
        return Err("Target folder is outside the allowed workspaces.".to_string());
    }

    let file_name = requested_path
        .file_name()
        .ok_or_else(|| "New workbook path is missing a file name.".to_string())?;

    Ok(parent.join(file_name))
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

fn validate_cell(cell: &str) -> Result<String, String> {
    let cell = cell.trim().to_ascii_uppercase();

    if cell.len() < 2 || cell.len() > 8 {
        return Err(format!("Invalid cell address `{cell}`."));
    }

    let mut seen_digit = false;
    let mut row_digits = String::new();

    for character in cell.chars() {
        if character.is_ascii_alphabetic() {
            if seen_digit {
                return Err(format!("Invalid cell address `{cell}`."));
            }
        } else if character.is_ascii_digit() {
            seen_digit = true;
            row_digits.push(character);
        } else {
            return Err(format!("Invalid cell address `{cell}`."));
        }
    }

    if !seen_digit || row_digits.parse::<u32>().unwrap_or(0) == 0 {
        return Err(format!("Invalid cell address `{cell}`."));
    }

    Ok(cell)
}

fn clean_path_input(value: &str) -> String {
    value
        .trim()
        .trim_matches('"')
        .trim_matches('\'')
        .trim()
        .to_string()
}

fn is_xlsx_path(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| extension.eq_ignore_ascii_case("xlsx"))
        .unwrap_or(false)
}

fn is_secret_path(path: &Path) -> bool {
    path.components().any(|component| match component {
        Component::Normal(value) => {
            let value = value.to_string_lossy().to_ascii_lowercase();
            value == ".env"
                || value.starts_with(".env.")
                || value.ends_with(".pem")
                || value.ends_with(".key")
                || value.ends_with(".pfx")
                || value.ends_with(".p12")
                || value.contains("secret")
                || value.contains("credential")
        }
        _ => false,
    })
}
