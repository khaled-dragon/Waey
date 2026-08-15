import { invoke } from "@tauri-apps/api/core";
import type { GuideOverlayRequest } from "../../shared/types";

export function showGuideStep(request: GuideOverlayRequest) {
  return invoke<void>("show_guide_step", { request });
}

export function completeGuideStep() {
  return invoke<void>("complete_guide_step");
}

export function cancelGuideStep() {
  return invoke<void>("cancel_guide_step");
}
