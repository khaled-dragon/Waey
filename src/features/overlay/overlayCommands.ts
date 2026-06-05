import { invoke } from "@tauri-apps/api/core";

export function showOverlayWindow() {
  return invoke<void>("show_overlay_window");
}

export function hideOverlayWindow() {
  return invoke<void>("hide_overlay_window");
}
