import { invoke } from "@tauri-apps/api/core";
import type { LlmProvider, ProviderDraft } from "../../shared/types";

export function listLlmProviders() {
  return invoke<LlmProvider[]>("list_llm_providers");
}

export function saveLlmProvider(provider: ProviderDraft) {
  return invoke<LlmProvider>("save_llm_provider", { provider });
}

export function deleteLlmProvider(providerId: string) {
  return invoke<void>("delete_llm_provider", { providerId });
}
