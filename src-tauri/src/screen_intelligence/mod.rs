mod model;
mod normalizer;
mod windows_uia;

pub use model::{
    ScreenContextDiagnostics, ScreenContextPoint, ScreenContextSnapshot, UiContextRect,
    UiElementSummary, VisibleWindowSummary,
};
pub use windows_uia::{capture_foreground_window_handle, capture_windows_ui_context};

use serde::{Deserialize, Serialize};

pub const SCREEN_CONTEXT_SCHEMA_VERSION: u8 = 2;

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ScreenIntelligenceMode {
    #[default]
    Compatibility,
    Shadow,
    Enabled,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ScreenContextCollectionStatus {
    #[default]
    Complete,
    Partial,
    Unavailable,
}

pub fn current_mode() -> ScreenIntelligenceMode {
    match std::env::var("WAEY_SCREEN_INTELLIGENCE_MODE")
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase()
        .as_str()
    {
        "shadow" => ScreenIntelligenceMode::Shadow,
        "enabled" => ScreenIntelligenceMode::Enabled,
        _ => ScreenIntelligenceMode::Compatibility,
    }
}

pub fn collection_diagnostics(
    elapsed_ms: u64,
    element_count: usize,
    truncated: bool,
) -> ScreenContextDiagnostics {
    let mut warnings = Vec::new();

    if truncated {
        warnings
            .push("UI element list was truncated to stay within the capture budget.".to_string());
    }

    ScreenContextDiagnostics {
        mode: current_mode(),
        status: if truncated {
            ScreenContextCollectionStatus::Partial
        } else {
            ScreenContextCollectionStatus::Complete
        },
        elapsed_ms,
        element_count,
        truncated,
        warnings,
    }
}

#[cfg(test)]
mod tests {
    use super::{collection_diagnostics, ScreenContextCollectionStatus};

    #[test]
    fn complete_collection_has_no_warning() {
        let diagnostics = collection_diagnostics(120, 14, false);

        assert_eq!(diagnostics.status, ScreenContextCollectionStatus::Complete);
        assert!(diagnostics.warnings.is_empty());
        assert!(!diagnostics.truncated);
    }

    #[test]
    fn truncated_collection_is_explicitly_partial() {
        let diagnostics = collection_diagnostics(750, 120, true);

        assert_eq!(diagnostics.status, ScreenContextCollectionStatus::Partial);
        assert_eq!(diagnostics.warnings.len(), 1);
        assert!(diagnostics.truncated);
    }
}
