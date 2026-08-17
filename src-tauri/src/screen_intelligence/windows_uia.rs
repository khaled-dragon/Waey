use super::{
    normalizer::normalize_snapshot, ScreenContextSnapshot, UiContextRect,
    SCREEN_CONTEXT_SCHEMA_VERSION,
};
use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const COLLECTION_TIMEOUT: Duration = Duration::from_millis(4_000);
const COLLECTOR_VERSION: &str = "v3";

pub fn capture_windows_ui_context(
    region: Option<UiContextRect>,
    allow_clipboard_selection: bool,
) -> Option<ScreenContextSnapshot> {
    if !cfg!(target_os = "windows") {
        return None;
    }

    let started_at = Instant::now();
    let raw_snapshot = run_collector(allow_clipboard_selection)?;
    let mut snapshot =
        serde_json::from_str::<ScreenContextSnapshot>(raw_snapshot.trim_start_matches('\u{feff}'))
            .ok()?;
    snapshot.schema_version = SCREEN_CONTEXT_SCHEMA_VERSION;
    snapshot.captured_at = timestamp_millis();
    snapshot.region = region.clone();

    Some(normalize_snapshot(
        snapshot,
        region.as_ref(),
        started_at.elapsed().as_millis().min(u128::from(u64::MAX)) as u64,
    ))
}

fn run_collector(allow_clipboard_selection: bool) -> Option<String> {
    let run_id = format!("{}_{}", std::process::id(), timestamp_millis());
    let script_path = collector_script_path();
    let runner_path = temporary_file_path(&run_id, "vbs");
    let output_path = temporary_file_path(&run_id, "json");

    fs::create_dir_all(script_path.parent()?).ok()?;
    ensure_collector_script(&script_path).ok()?;
    fs::write(
        &runner_path,
        vbs_runner_content(&script_path, &output_path, allow_clipboard_selection),
    )
    .ok()?;

    let output = run_wscript_hidden(&runner_path, COLLECTION_TIMEOUT);
    let json = output
        .filter(|process_output| process_output.status.success())
        .and_then(|_| fs::read_to_string(&output_path).ok());

    let _ = fs::remove_file(runner_path);
    let _ = fs::remove_file(output_path);

    json
}

fn collector_script_path() -> PathBuf {
    std::env::temp_dir()
        .join("waey")
        .join("screen-intelligence")
        .join(COLLECTOR_VERSION)
        .join("windows-uia-collector.ps1")
}

fn temporary_file_path(run_id: &str, extension: &str) -> PathBuf {
    std::env::temp_dir()
        .join("waey")
        .join("screen-intelligence")
        .join(COLLECTOR_VERSION)
        .join(format!("context-{run_id}.{extension}"))
}

fn ensure_collector_script(path: &PathBuf) -> Result<(), String> {
    let expected = powershell_file_content(UIA_COLLECTOR_SCRIPT);
    let current = fs::read_to_string(path).ok();

    if current.as_deref() != Some(expected.as_str()) {
        fs::write(path, expected).map_err(|error| error.to_string())?;
    }

    Ok(())
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

        thread::sleep(Duration::from_millis(15));
    }
}

fn timestamp_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default()
}

const UIA_COLLECTOR_SCRIPT: &str = r#"
Add-Type @"
using System;
using System.Runtime.InteropServices;
public static class WaeyNativeWindow {
  [DllImport("user32.dll", SetLastError = true)]
  public static extern IntPtr GetForegroundWindow();

  [DllImport("user32.dll", SetLastError = true)]
  public static extern bool SetProcessDPIAware();

  [DllImport("user32.dll", SetLastError = true)]
  public static extern bool SetProcessDpiAwarenessContext(IntPtr value);

  [DllImport("shcore.dll")]
  public static extern int SetProcessDpiAwareness(int value);

  public static void EnablePerMonitorDpiAwareness() {
    try { if (SetProcessDpiAwarenessContext(new IntPtr(-4))) { return; } } catch {}
    try { if (SetProcessDpiAwareness(2) == 0) { return; } } catch {}
    try { SetProcessDPIAware(); } catch {}
  }
}
"@

[WaeyNativeWindow]::EnablePerMonitorDpiAwareness()
Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName System.Windows.Forms

$MaxDepth = 15
$MaxElements = 2000
$CollectionBudgetMs = 1150
$TerminalRoles = @('DataItem', 'Cell', 'TreeItem')
$ScaffoldRoles = @('Pane', 'Group', 'Custom')

function Clean-Text([string]$value, [int]$maxLength = 240) {
  if ([string]::IsNullOrWhiteSpace($value)) { return $null }
  $clean = ($value -replace '\s+', ' ').Trim()
  if ($clean.Length -gt $maxLength) { return $clean.Substring(0, $maxLength - 1) + '…' }
  return $clean
}

function Get-ControlTypeName($element) {
  try {
    $programmatic = $element.Current.ControlType.ProgrammaticName
    if ($programmatic.StartsWith('ControlType.')) { return $programmatic.Substring(12) }
    return $programmatic
  } catch { return 'Unknown' }
}

function Get-ElementText($element, [string]$role) {
  try {
    if ($element.GetCurrentPropertyValue([System.Windows.Automation.AutomationElement]::IsPasswordProperty)) { return $null }
  } catch {}

  if ($role -notin @('Edit', 'Document', 'Text')) { return $null }
  try {
    $valuePattern = $element.GetCurrentPattern([System.Windows.Automation.ValuePattern]::Pattern)
    $value = Clean-Text $valuePattern.Current.Value 8000
    if ($value) { return $value }
  } catch {}
  try {
    $textPattern = $element.GetCurrentPattern([System.Windows.Automation.TextPattern]::Pattern)
    return Clean-Text ($textPattern.DocumentRange.GetText(8000)) 8000
  } catch { return $null }
}

function Get-SelectedText($element) {
  try {
    if ($element.GetCurrentPropertyValue([System.Windows.Automation.AutomationElement]::IsPasswordProperty)) { return $null }
    $textPattern = $element.GetCurrentPattern([System.Windows.Automation.TextPattern]::Pattern)
    $selection = $textPattern.GetSelection()
    if ($null -eq $selection -or $selection.Count -eq 0) { return $null }
    return Clean-Text ($selection[0].GetText(2400)) 2400
  } catch { return $null }
}

function Get-ClipboardSelectedText {
  $originalClipboard = $null
  try { $originalClipboard = [System.Windows.Forms.Clipboard]::GetDataObject() } catch {}
  try {
    [System.Windows.Forms.SendKeys]::SendWait('^c')
    Start-Sleep -Milliseconds 80
    if ([System.Windows.Forms.Clipboard]::ContainsText()) {
      return Clean-Text ([System.Windows.Forms.Clipboard]::GetText()) 2400
    }
    return $null
  } catch { return $null }
  finally {
    if ($null -ne $originalClipboard) {
      try { [System.Windows.Forms.Clipboard]::SetDataObject($originalClipboard, $true) } catch {}
    }
  }
}

function Get-ParentTrail($element) {
  $walker = [System.Windows.Automation.TreeWalker]::RawViewWalker
  $trail = New-Object System.Collections.Generic.List[string]
  $current = $element
  for ($index = 0; $index -lt 6; $index++) {
    try {
      $current = $walker.GetParent($current)
      if ($null -eq $current) { break }
      $role = Get-ControlTypeName $current
      $name = Clean-Text $current.Current.Name 80
      if ($role -eq 'Desktop') { break }
      if ($name) { $trail.Add("$role `"$name`"") } else { $trail.Add($role) }
    } catch { break }
  }
  return $trail.ToArray()
}

function Get-ChildCount($element) {
  try {
    $walker = [System.Windows.Automation.TreeWalker]::RawViewWalker
    $count = 0
    $child = $walker.GetFirstChild($element)
    while ($null -ne $child -and $count -lt 999) {
      $count++
      $child = $walker.GetNextSibling($child)
    }
    return $count
  } catch { return 0 }
}

function Convert-Element($element, [int]$cursorX, [int]$cursorY, [int]$depth) {
  try {
    $isPassword = [bool]$element.GetCurrentPropertyValue([System.Windows.Automation.AutomationElement]::IsPasswordProperty)
    if ($isPassword) { return $null }
    $rect = $element.Current.BoundingRectangle
    if ($rect.Width -le 0 -or $rect.Height -le 0) { return $null }

    $role = Get-ControlTypeName $element
    $name = Clean-Text $element.Current.Name 240
    $value = Get-ElementText $element $role
    $selectedText = Get-SelectedText $element
    $automationId = Clean-Text $element.Current.AutomationId 160
    $className = Clean-Text $element.Current.ClassName 160
    $isOffscreen = [bool]$element.Current.IsOffscreen
    $isEnabled = [bool]$element.Current.IsEnabled
    $focused = [bool]$element.Current.HasKeyboardFocus
    $underCursor = $cursorX -ge $rect.Left -and $cursorX -le $rect.Right -and $cursorY -ge $rect.Top -and $cursorY -le $rect.Bottom

    if ([string]::IsNullOrWhiteSpace($name) -and [string]::IsNullOrWhiteSpace($value) -and [string]::IsNullOrWhiteSpace($selectedText) -and -not $focused -and -not $underCursor) { return $null }

    return [PSCustomObject]@{
      role = $role
      name = if ($name) { $name } else { '' }
      value = $value
      selectedText = $selectedText
      automationId = $automationId
      className = $className
      bounds = [PSCustomObject]@{
        x = [int][Math]::Round($rect.Left)
        y = [int][Math]::Round($rect.Top)
        width = [uint32][Math]::Round($rect.Width)
        height = [uint32][Math]::Round($rect.Height)
      }
      focused = $focused
      underCursor = $underCursor
      isEnabled = $isEnabled
      isOffscreen = $isOffscreen
      depth = [uint16]$depth
      childCount = [uint16](Get-ChildCount $element)
      parentTrail = @(Get-ParentTrail $element)
    }
  } catch { return $null }
}

function Is-EmptyScaffold($item) {
  return $ScaffoldRoles -contains $item.role -and [string]::IsNullOrWhiteSpace($item.name) -and [string]::IsNullOrWhiteSpace($item.value) -and -not $item.focused -and -not $item.underCursor
}

function Walk-Elements($root, [int]$cursorX, [int]$cursorY) {
  $walker = [System.Windows.Automation.TreeWalker]::RawViewWalker
  $queue = New-Object System.Collections.Queue
  $queue.Enqueue([PSCustomObject]@{ Element = $root; Depth = 0 })
  $items = New-Object System.Collections.Generic.List[object]
  $timer = [System.Diagnostics.Stopwatch]::StartNew()

  while ($queue.Count -gt 0 -and $items.Count -lt $MaxElements -and $timer.ElapsedMilliseconds -lt $CollectionBudgetMs) {
    $current = $queue.Dequeue()
    $item = Convert-Element $current.Element $cursorX $cursorY $current.Depth
    if ($null -ne $item -and -not (Is-EmptyScaffold $item)) { $items.Add($item) }

    if ($current.Depth -ge $MaxDepth -or ($null -ne $item -and $TerminalRoles -contains $item.role)) { continue }
    try {
      $child = $walker.GetFirstChild($current.Element)
      while ($null -ne $child) {
        $queue.Enqueue([PSCustomObject]@{ Element = $child; Depth = $current.Depth + 1 })
        $child = $walker.GetNextSibling($child)
      }
    } catch {}
  }
  return $items.ToArray()
}

function Get-VisibleWindows {
  $items = New-Object System.Collections.Generic.List[object]
  try {
    $windows = [System.Windows.Automation.AutomationElement]::RootElement.FindAll([System.Windows.Automation.TreeScope]::Children, [System.Windows.Automation.Condition]::TrueCondition)
    foreach ($window in $windows) {
      if ($items.Count -ge 12) { break }
      try {
        $rect = $window.Current.BoundingRectangle
        $title = Clean-Text $window.Current.Name 240
        if ($window.Current.IsOffscreen -or $rect.Width -le 0 -or $rect.Height -le 0 -or -not $title) { continue }
        $processName = $null
        try { $processName = (Get-Process -Id $window.Current.ProcessId -ErrorAction SilentlyContinue).ProcessName } catch {}
        $items.Add([PSCustomObject]@{
          title = $title
          appName = $processName
          bounds = [PSCustomObject]@{ x = [int]$rect.Left; y = [int]$rect.Top; width = [uint32]$rect.Width; height = [uint32]$rect.Height }
        })
      } catch {}
    }
  } catch {}
  return $items.ToArray()
}

$cursor = [System.Windows.Forms.Cursor]::Position
$activeWindow = $null
try {
  $foregroundHandle = [WaeyNativeWindow]::GetForegroundWindow()
  if ($foregroundHandle -ne [IntPtr]::Zero) { $activeWindow = [System.Windows.Automation.AutomationElement]::FromHandle($foregroundHandle) }
} catch {}
if ($null -eq $activeWindow) {
  try { $activeWindow = [System.Windows.Automation.AutomationElement]::FocusedElement } catch {}
}

$focusedElement = $null
$pointedElement = $null
try { $focusedElement = [System.Windows.Automation.AutomationElement]::FocusedElement } catch {}
try { $pointedElement = [System.Windows.Automation.AutomationElement]::FromPoint($cursor) } catch {}

$activeWindowTitle = $null
$activeAppName = $null
$activeWindowBounds = $null
if ($null -ne $activeWindow) {
  try { $activeWindowTitle = Clean-Text $activeWindow.Current.Name 240 } catch {}
  try { $activeAppName = (Get-Process -Id $activeWindow.Current.ProcessId -ErrorAction SilentlyContinue).ProcessName } catch {}
  try {
    $rect = $activeWindow.Current.BoundingRectangle
    $activeWindowBounds = [PSCustomObject]@{ x = [int]$rect.Left; y = [int]$rect.Top; width = [uint32]$rect.Width; height = [uint32]$rect.Height }
  } catch {}
}

$focused = if ($null -ne $focusedElement) { Convert-Element $focusedElement $cursor.X $cursor.Y 0 } else { $null }
$pointed = if ($null -ne $pointedElement) { Convert-Element $pointedElement $cursor.X $cursor.Y 0 } else { $null }
$selectedText = if ($null -ne $focused) { $focused.selectedText } else { $null }
$selectedTextSource = if ($selectedText) { 'uia' } else { $null }
if (-not $selectedText -and $AllowClipboardSelection) {
  $selectedText = Get-ClipboardSelectedText
  if ($selectedText) { $selectedTextSource = 'clipboard' }
}

$snapshot = [PSCustomObject]@{
  platform = 'windows'
  activeWindowTitle = $activeWindowTitle
  activeAppName = $activeAppName
  selectedText = $selectedText
  selectedTextSource = $selectedTextSource
  capturedAt = 0
  region = $null
  cursor = [PSCustomObject]@{ x = [int]$cursor.X; y = [int]$cursor.Y }
  activeWindowBounds = $activeWindowBounds
  focusedElement = $focused
  pointedElement = $pointed
  visibleWindows = @(Get-VisibleWindows)
  elements = if ($null -ne $activeWindow) { @(Walk-Elements $activeWindow $cursor.X $cursor.Y) } else { @() }
}

$json = $snapshot | ConvertTo-Json -Depth 10 -Compress
$utf8NoBom = New-Object System.Text.UTF8Encoding($false)
[System.IO.File]::WriteAllText($OutputFile, $json, $utf8NoBom)
"#;
