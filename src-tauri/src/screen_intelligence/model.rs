use super::{ScreenContextCollectionStatus, ScreenIntelligenceMode};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScreenContextSnapshot {
    #[serde(default)]
    pub schema_version: u8,
    pub platform: String,
    pub active_window_title: Option<String>,
    pub active_app_name: Option<String>,
    pub selected_text: Option<String>,
    pub selected_text_source: Option<String>,
    pub captured_at: u128,
    pub region: Option<UiContextRect>,
    #[serde(default)]
    pub cursor: Option<ScreenContextPoint>,
    #[serde(default)]
    pub active_window_bounds: Option<UiContextRect>,
    #[serde(default)]
    pub focused_element: Option<UiElementSummary>,
    #[serde(default)]
    pub pointed_element: Option<UiElementSummary>,
    #[serde(default)]
    pub visible_windows: Vec<VisibleWindowSummary>,
    #[serde(default)]
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
pub struct ScreenContextPoint {
    pub x: i32,
    pub y: i32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VisibleWindowSummary {
    pub title: String,
    pub app_name: Option<String>,
    pub bounds: UiContextRect,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UiElementSummary {
    pub role: String,
    pub name: String,
    pub value: Option<String>,
    pub selected_text: Option<String>,
    pub automation_id: Option<String>,
    #[serde(default)]
    pub class_name: Option<String>,
    pub bounds: UiContextRect,
    pub focused: bool,
    pub under_cursor: bool,
    #[serde(default)]
    pub is_enabled: bool,
    #[serde(default)]
    pub is_offscreen: bool,
    #[serde(default)]
    pub depth: u16,
    #[serde(default)]
    pub child_count: u16,
    #[serde(default)]
    pub parent_trail: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScreenContextDiagnostics {
    pub mode: ScreenIntelligenceMode,
    pub status: ScreenContextCollectionStatus,
    pub elapsed_ms: u64,
    pub element_count: usize,
    pub truncated: bool,
    pub warnings: Vec<String>,
}

impl Default for ScreenContextDiagnostics {
    fn default() -> Self {
        Self {
            mode: super::current_mode(),
            status: ScreenContextCollectionStatus::Complete,
            elapsed_ms: 0,
            element_count: 0,
            truncated: false,
            warnings: Vec::new(),
        }
    }
}
