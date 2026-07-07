use serde::{Deserialize, Serialize};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const MAX_UI_ELEMENTS: usize = 120;
const UI_CONTEXT_TIMEOUT: Duration = Duration::from_millis(750);

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UiContextSnapshot {
    pub platform: String,
    pub active_window_title: Option<String>,
    pub active_app_name: Option<String>,
    pub captured_at: u128,
    pub region: Option<UiContextRect>,
    pub elements: Vec<UiElementSummary>,
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
    pub automation_id: Option<String>,
    pub bounds: UiContextRect,
    pub focused: bool,
    pub under_cursor: bool,
}

pub fn capture_ui_context(region: Option<UiContextRect>) -> Option<UiContextSnapshot> {
    if !cfg!(target_os = "windows") {
        return None;
    }

    let output = run_powershell_hidden(UIA_SCRIPT, UI_CONTEXT_TIMEOUT)?;

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8(output.stdout).ok()?;
    let mut snapshot = serde_json::from_str::<UiContextSnapshot>(&stdout).ok()?;
    snapshot.captured_at = timestamp_millis();
    snapshot.region = region.clone();
    snapshot.elements = filter_elements(snapshot.elements, region);

    Some(snapshot)
}

fn run_powershell_hidden(script: &str, timeout: Duration) -> Option<std::process::Output> {
    let mut command = Command::new("powershell.exe");
    command
        .args([
            "-NoLogo",
            "-NoProfile",
            "-NonInteractive",
            "-WindowStyle",
            "Hidden",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            script,
        ])
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

fn filter_elements(
    elements: Vec<UiElementSummary>,
    region: Option<UiContextRect>,
) -> Vec<UiElementSummary> {
    let mut filtered = elements
        .into_iter()
        .filter(|element| {
            !element.name.trim().is_empty()
                || element.value.as_deref().unwrap_or("").trim().len() > 1
        })
        .filter(|element| element.bounds.width > 0 && element.bounds.height > 0)
        .filter(|element| {
            region
                .as_ref()
                .map(|rect| intersects(&element.bounds, rect))
                .unwrap_or(true)
        })
        .collect::<Vec<_>>();

    filtered.sort_by_key(element_priority);
    filtered.truncate(MAX_UI_ELEMENTS);
    filtered
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

function Convert-Element($element, $cursorX, $cursorY) {
  try {
    if ($element.Current.IsOffscreen) { return $null }
    if ($element.GetCurrentPropertyValue([System.Windows.Automation.AutomationElement]::IsPasswordProperty)) { return $null }

    $rect = $element.Current.BoundingRectangle
    if ($rect.Width -le 0 -or $rect.Height -le 0) { return $null }

    $role = Get-ControlTypeName $element
    $name = Clean-Text $element.Current.Name
    $value = Get-SafeValue $element $role

    if ([string]::IsNullOrWhiteSpace($name) -and [string]::IsNullOrWhiteSpace($value)) { return $null }

    $automationId = Clean-Text $element.Current.AutomationId 120
    $underCursor = $cursorX -ge $rect.Left -and $cursorX -le $rect.Right -and $cursorY -ge $rect.Top -and $cursorY -le $rect.Bottom
    $focused = $false
    try { $focused = [bool]$element.Current.HasKeyboardFocus } catch {}

    return [PSCustomObject]@{
      role = $role
      name = if ($name) { $name } else { '' }
      value = $value
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
$elements = @()

if ($null -ne $activeElement) {
  try { $activeWindowTitle = Clean-Text $activeElement.Current.Name 220 } catch {}
  try {
    $processIdValue = $activeElement.Current.ProcessId
    $activeAppName = (Get-Process -Id $processIdValue -ErrorAction SilentlyContinue).ProcessName
  } catch {}
  $elements = Walk-Elements $activeElement $cursorPoint.X $cursorPoint.Y
}

$snapshot = [PSCustomObject]@{
  platform = 'windows'
  activeWindowTitle = $activeWindowTitle
  activeAppName = $activeAppName
  capturedAt = 0
  region = $null
  elements = $elements
}

$snapshot | ConvertTo-Json -Depth 8 -Compress
"#;
