import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import type { CaptureRect, ScreenCapture, UiContextSnapshot } from "../../shared/types";

export function captureCurrentScreen() {
  return invoke<ScreenCapture>("capture_current_screen");
}

export function captureSelectedRegion(rect: CaptureRect) {
  return invoke<ScreenCapture>("capture_selected_region", { rect });
}

export function captureCurrentUiContext() {
  return invoke<UiContextSnapshot | null>("capture_current_ui_context");
}

export function showRegionSelector() {
  return invoke<void>("show_region_selector_window");
}

export function cancelRegionSelection() {
  return invoke<void>("cancel_region_selection");
}

export function capturePreviewUrl(capture: ScreenCapture) {
  return convertFileSrc(capture.path);
}
