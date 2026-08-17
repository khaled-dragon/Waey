pub use crate::screen_intelligence::{
    ScreenContextDiagnostics, ScreenContextPoint, ScreenContextSnapshot as UiContextSnapshot,
    UiContextRect, UiElementSummary, VisibleWindowSummary,
};

pub fn capture_ui_context(
    region: Option<UiContextRect>,
    allow_clipboard_selection: bool,
    target_window_handle: Option<isize>,
) -> Result<Option<UiContextSnapshot>, String> {
    crate::screen_intelligence::capture_windows_ui_context(
        region,
        allow_clipboard_selection,
        target_window_handle,
    )
}
