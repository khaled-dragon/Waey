import { invoke } from "@tauri-apps/api/core";
import type { DesktopAction, DesktopActionResult } from "../../shared/types";

// `approved: true` is supplied only from the visible Apply button. The Rust
// command validates the action again and re-resolves its target before invoke.
export function executeDesktopAction(action: DesktopAction) {
  return invoke<DesktopActionResult>("execute_desktop_action", {
    request: { action, approved: true },
  });
}
