use crate::screen_intelligence::{
    collection_diagnostics, ScreenContextDiagnostics, SCREEN_CONTEXT_SCHEMA_VERSION,
};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const MAX_UI_ELEMENTS: usize = 120;
const UI_CONTEXT_TIMEOUT: Duration = Duration::from_millis(750);

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UiContextSnapshot {
    #[serde(default)]
    pub schema_version: u8,
    pub platform: String,
    pub active_window_title: Option<String>,
    pub active_app_name: Option<String>,
    pub selected_text: Option<String>,
    pub selected_text_source: Option<String>,
    pub captured_at: u128,
    pub region: Option<UiContextRect>,
    pub elements: Vec<UiElementSummary>,
    #[serde(default)]
    pub diagnostics: ScreenContextDiagnostics,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UiContextRect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UiElementSummary {
    pub role: String,
    pub name: String,
    pub value: Option<String>,
    pub selected_text: Option<String>,
    pub automation_id: Option<String>,
    pub bounds: UiContextRect,
    pub focused: bool,
    pub under_cursor: bool,
}

pub fn capture_ui_context(
    region: Option<UiContextRect>,
    allow_clipboard_selection: bool,
) -> Option<UiContextSnapshot> {
    if !cfg!(target_os = "windows") {
        return None;
    }

    let started_at = Instant::now();
    let stdout = run_uia_script_hidden(UIA_SCRIPT, UI_CONTEXT_TIMEOUT, allow_clipboard_selection)?;
    let mut snapshot =
        serde_json::from_str::<UiContextSnapshot>(stdout.trim_start_matches('\u{feff}')).ok()?;
    let filtered_elements = filter_elements(snapshot.elements, region.as_ref());

    snapshot.schema_version = SCREEN_CONTEXT_SCHEMA_VERSION;
    snapshot.captured_at = timestamp_millis();
    snapshot.region = region;
    snapshot.diagnostics = collection_diagnostics(
        started_at.elapsed().as_millis().min(u128::from(u64::MAX)) as u64,
        filtered_elements.elements.len(),
        filtered_elements.truncated,
    );
    snapshot.elements = filtered_elements.elements;

    Some(snapshot)
}

fn run_uia_script_hidden(
    script: &str,
    timeout: Duration,
    allow_clipboard_selection: bool,
) -> Option<String> {
    let run_id = format!("{}_{}", std::process::id(), timestamp_millis());
    let script_path = temp_file_path(&run_id, "ps1");
    let runner_path = temp_file_path(&run_id, "vbs");
    let output_path = temp_file_path(&run_id, "json");

    fs::create_dir_all(script_path.parent()?).ok()?;
    fs::write(&script_path, powershell_file_content(script)).ok()?;
    fs::write(
        &runner_path,
        vbs_runner_content(&script_path, &output_path, allow_clipboard_selection),
    )
    .ok()?;

    let output = run_wscript_hidden(&runner_path, timeout);
    let json = output
        .filter(|output| output.status.success())
        .and_then(|_| fs::read_to_string(&output_path).ok());

    let _ = fs::remove_file(script_path);
    let _ = fs::remove_file(runner_path);
    let _ = fs::remove_file(output_path);

    json
}

fn run_wscript_hidden(runner_path: &PathBuf, timeout: Duration) -> Option<Output> {
    let mut command = Command::new("wscript.exe");
    command
        .args(["//B", "//NoLogo", runner_path.to_str()?])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        command.creation_flags(CREATE_NO_WINDOW);
    }

    let mut child = command.spawn().ok()?;
    let started_at = Instant::now();

    loop {
        if child.try_wait().ok()?.is_some() {
            return child.wait_with_output().ok();
        }

        if started_at.elapsed() >= timeout {
            let _ = child.kill();
            let _ = child.wait();
            return None;
        }

        thread::sleep(Duration::from_millis(20));
    }
}

fn temp_file_path(run_id: &str, extension: &str) -> PathBuf {
    std::env::temp_dir()
        .join("waey")
        .join("ui-context")
        .join(format!("waey-ui-context-{run_id}.{extension}"))
}

fn powershell_file_content(script: &str) -> String {
    format!("param([string]$OutputFile, [bool]$AllowClipboardSelection = $false)\n{script}")
}

fn vbs_runner_content(
    script_path: &PathBuf,
    output_path: &PathBuf,
    allow_clipboard_selection: bool,
) -> String {
    let powershell_path = std::env::var("SystemRoot")
        .map(|root| format!("{root}\\System32\\WindowsPowerShell\\v1.0\\powershell.exe"))
        .unwrap_or_else(|_| "powershell.exe".to_string());
    let command = format!(
        "\"{}\" -NoLogo -NoProfile -NonInteractive -ExecutionPolicy Bypass -File \"{}\" -OutputFile \"{}\" -AllowClipboardSelection ${}",
        powershell_path,
        script_path.display(),
        output_path.display(),
        if allow_clipboard_selection { "true" } else { "false" }
    );

    format!(
        "Set shell = CreateObject(\"WScript.Shell\")\nexitCode = shell.Run(\"{}\", 0, True)\nWScript.Quit exitCode\n",
        command.replace('"', "\"\"")
    )
}

struct FilteredElements {
    elements: Vec<UiElementSummary>,
    truncated: bool,
}

fn filter_elements(
    elements: Vec<UiElementSummary>,
    region: Option<&UiContextRect>,
) -> FilteredElements {
    let mut filtered = elements
        .into_iter()
        .filter(|element| {
            !element.name.trim().is_empty()
                || element.value.as_deref().unwrap_or("").trim().len() > 1
                || element.selected_text.as_deref().unwrap_or("").trim().len() > 1
        })
        .filter(|element| element.bounds.width > 0 && element.bounds.height > 0)
        .filter(|element| {
            region
                .map(|rect| intersects(&element.bounds, rect))
                .unwrap_or(true)
        })
        .collect::<Vec<_>>();

    filtered.sort_by_key(element_priority);
    let truncated = filtered.len() > MAX_UI_ELEMENTS;
    filtered.truncate(MAX_UI_ELEMENTS);

    FilteredElements {
        elements: filtered,
        truncated,
    }
}

fn element_priority(element: &UiElementSummary) -> u8 {
    if element.focused {
        return 0;
    }

    if element.under_cursor {
        return 1;
    }

    match element.role.as_str() {
        "Edit" | "Button" | "ComboBox" | "CheckBox" | "RadioButton" | "MenuItem" | "Hyperlink" => 2,
        "Text" => 3,
        _ => 4,
    }
}

fn intersects(a: &UiContextRect, b: &UiContextRect) -> bool {
    let a_right = a.x.saturating_add(a.width as i32);
    let a_bottom = a.y.saturating_add(a.height as i32);
    let b_right = b.x.saturating_add(b.width as i32);
    let b_bottom = b.y.saturating_add(b.height as i32);

    a.x < b_right && a_right > b.x && a.y < b_bottom && a_bottom > b.y
}

fn timestamp_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default()
}

const UIA_SCRIPT: &str = r#"
Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName System.Windows.Forms

function Clean-Text([string]$value, [int]$maxLength = 180) {
  if ([string]::IsNullOrWhiteSpace($value)) { return $null }
  $clean = ($value -replace '\s+', ' ').Trim()
  if ($clean.Length -gt $maxLength) { return $clean.Substring(0, $maxLength) }
  return $clean
}

function Get-ControlTypeName($element) {
  try {
    $programmatic = $element.Current.ControlType.ProgrammaticName
    if ($programmatic.StartsWith('ControlType.')) {
      return $programmatic.Substring(12)
    }
    return $programmatic
  } catch {
    return 'Unknown'
  }
}

function Get-SafeValue($element, [string]$role) {
  try {
    if ($element.GetCurrentPropertyValue([System.Windows.Automation.AutomationElement]::IsPasswordProperty)) {
      return $null
    }
  } catch {}

  if ($role -notin @('Edit', 'Document', 'Text')) { return $null }

  try {
    $pattern = $element.GetCurrentPattern([System.Windows.Automation.ValuePattern]::Pattern)
    return Clean-Text $pattern.Current.Value 240
  } catch {
    return $null
  }
}

function Get-SelectedText($element, [string]$role) {
  try {
    if ($element.GetCurrentPropertyValue([System.Windows.Automation.AutomationElement]::IsPasswordProperty)) {
      return $null
    }
  } catch {}

  if ($role -notin @('Edit', 'Document', 'Text')) { return $null }

  try {
    $textPattern = $element.GetCurrentPattern([System.Windows.Automation.TextPattern]::Pattern)
    $selection = $textPattern.GetSelection()
    if ($null -eq $selection -or $selection.Count -eq 0) { return $null }

    $selected = $selection[0].GetText(600)
    return Clean-Text $selected 600
  } catch {
    return $null
  }
}

function Get-ClipboardSelectedText {
  $originalClipboard = $null
  try { $originalClipboard = [System.Windows.Forms.Clipboard]::GetDataObject() } catch {}

  try {
    [System.Windows.Forms.SendKeys]::SendWait('^c')
    Start-Sleep -Milliseconds 90

    if ([System.Windows.Forms.Clipboard]::ContainsText()) {
      return Clean-Text ([System.Windows.Forms.Clipboard]::GetText()) 1200
    }

    return $null
  } catch {
    return $null
  } finally {
    if ($null -ne $originalClipboard) {
      try { [System.Windows.Forms.Clipboard]::SetDataObject($originalClipboard, $true) } catch {}
    }
  }
}

function Convert-Element($element, $cursorX, $cursorY) {
  try {
    if ($element.Current.IsOffscreen) { return $null }
    if ($element.GetCurrentPropertyValue([System.Windows.Automation.AutomationElement]::IsPasswordProperty)) { return $null }

    $rect = $element.Current.BoundingRectangle
    if ($rect.Width -le 0 -or $rect.Height -le 0) { return $null }

    $role = Get-ControlTypeName $element
    $name = Clean-Text $element.Current.Name
    $value = Get-SafeValue $element $role
    $selectedText = Get-SelectedText $element $role

    if ([string]::IsNullOrWhiteSpace($name) -and [string]::IsNullOrWhiteSpace($value) -and [string]::IsNullOrWhiteSpace($selectedText)) { return $null }

    $automationId = Clean-Text $element.Current.AutomationId 120
    $underCursor = $cursorX -ge $rect.Left -and $cursorX -le $rect.Right -and $cursorY -ge $rect.Top -and $cursorY -le $rect.Bottom
    $focused = $false
    try { $focused = [bool]$element.Current.HasKeyboardFocus } catch {}

    return [PSCustomObject]@{
      role = $role
      name = if ($name) { $name } else { '' }
      value = $value
      selectedText = $selectedText
      automationId = $automationId
      bounds = [PSCustomObject]@{
        x = [int][Math]::Round($rect.Left)
        y = [int][Math]::Round($rect.Top)
        width = [uint32][Math]::Round($rect.Width)
        height = [uint32][Math]::Round($rect.Height)
      }
      focused = [bool]$focused
      underCursor = [bool]$underCursor
    }
  } catch {
    return $null
  }
}

function Walk-Elements($root, $cursorX, $cursorY) {
  $walker = [System.Windows.Automation.TreeWalker]::ControlViewWalker
  $queue = New-Object System.Collections.Queue
  $queue.Enqueue([PSCustomObject]@{ Element = $root; Depth = 0 })
  $items = New-Object System.Collections.Generic.List[object]

  while ($queue.Count -gt 0 -and $items.Count -lt 140) {
    $current = $queue.Dequeue()
    $converted = Convert-Element $current.Element $cursorX $cursorY

    if ($null -ne $converted) {
      $items.Add($converted)
    }

    if ($current.Depth -ge 5) { continue }

    try {
      $child = $walker.GetFirstChild($current.Element)
      while ($null -ne $child) {
        $queue.Enqueue([PSCustomObject]@{ Element = $child; Depth = $current.Depth + 1 })
        $child = $walker.GetNextSibling($child)
      }
    } catch {}
  }

  return $items
}

function Get-ContextRoot {
  $focused = $null
  try { $focused = [System.Windows.Automation.AutomationElement]::FocusedElement } catch {}
  if ($null -eq $focused) { return $null }

  $walker = [System.Windows.Automation.TreeWalker]::ControlViewWalker
  $current = $focused
  $lastUseful = $focused

  while ($null -ne $current) {
    try {
      $role = Get-ControlTypeName $current
      if ($role -eq 'Window' -or $role -eq 'Pane' -or $role -eq 'Document') {
        $lastUseful = $current
      }

      $parent = $walker.GetParent($current)
      if ($null -eq $parent) { break }

      $parentRole = Get-ControlTypeName $parent
      if ($parentRole -eq 'Desktop') { break }

      $current = $parent
    } catch {
      break
    }
  }

  return $lastUseful
}

$cursorPoint = [System.Windows.Forms.Cursor]::Position
$activeElement = Get-ContextRoot

$activeWindowTitle = $null
$activeAppName = $null
$selectedText = $null
$selectedTextSource = $null
$elements = @()

if ($null -ne $activeElement) {
  try { $activeWindowTitle = Clean-Text $activeElement.Current.Name 220 } catch {}
  try {
    $processIdValue = $activeElement.Current.ProcessId
    $activeAppName = (Get-Process -Id $processIdValue -ErrorAction SilentlyContinue).ProcessName
  } catch {}
  $elements = Walk-Elements $activeElement $cursorPoint.X $cursorPoint.Y

  foreach ($item in $elements) {
    if (-not [string]::IsNullOrWhiteSpace($item.selectedText)) {
      $selectedText = $item.selectedText
      $selectedTextSource = 'uia'
      break
    }
  }
}

if ($AllowClipboardSelection -and [string]::IsNullOrWhiteSpace($selectedText)) {
  $clipboardSelection = Get-ClipboardSelectedText
  if (-not [string]::IsNullOrWhiteSpace($clipboardSelection)) {
    $selectedText = $clipboardSelection
    $selectedTextSource = 'clipboard'
  }
}

$snapshot = [PSCustomObject]@{
  platform = 'windows'
  activeWindowTitle = $activeWindowTitle
  activeAppName = $activeAppName
  selectedText = $selectedText
  selectedTextSource = $selectedTextSource
  capturedAt = 0
  region = $null
  elements = $elements
}

$json = $snapshot | ConvertTo-Json -Depth 8 -Compress
if ($OutputFile) {
  $utf8NoBom = New-Object System.Text.UTF8Encoding($false)
  [System.IO.File]::WriteAllText($OutputFile, $json, $utf8NoBom)
} else {
  $json
}
"#;
