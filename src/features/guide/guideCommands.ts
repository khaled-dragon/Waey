import { invoke } from "@tauri-apps/api/core";
import type { GuideOverlayRequest } from "../../shared/types";

export function showGuideStep(request: GuideOverlayRequest) {
  return invoke<void>("show_guide_step", { request });
}

export function startGuideOffer() {
  return invoke<void>("start_guide_offer");
}

export function completeGuideStep() {
  return invoke<void>("complete_guide_step");
}

export function requestGuideAdjustment() {
  return invoke<void>("request_guide_adjustment");
}

export function cancelGuideStep() {
  return invoke<void>("cancel_guide_step");
}
