pub use crate::screen_intelligence::{
    ScreenContextDiagnostics, ScreenContextPoint, ScreenContextSnapshot as UiContextSnapshot,
    UiContextRect, UiElementSummary, VisibleWindowSummary,
};

pub fn capture_ui_context(
    region: Option<UiContextRect>,
    allow_clipboard_selection: bool,
) -> Option<UiContextSnapshot> {
    crate::screen_intelligence::capture_windows_ui_context(region, allow_clipboard_selection)
}
