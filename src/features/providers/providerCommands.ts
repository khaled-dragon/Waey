import { invoke } from "@tauri-apps/api/core";
import type { LlmProvider, ManagedProviderUpdate, ProviderDraft } from "../../shared/types";

export function bootstrapManagedProvider() {
  return invoke<LlmProvider | null>("bootstrap_managed_provider");
}

export function checkManagedProviderUpdate() {
  return invoke<ManagedProviderUpdate | null>("check_managed_provider_update");
}

export function applyManagedProviderUpdate() {
  return invoke<LlmProvider>("apply_managed_provider_update");
}

export function listLlmProviders() {
  return invoke<LlmProvider[]>("list_llm_providers");
}

export function saveLlmProvider(provider: ProviderDraft) {
  return invoke<LlmProvider>("save_llm_provider", { provider });
}

export function deleteLlmProvider(providerId: string) {
  return invoke<void>("delete_llm_provider", { providerId });
}
