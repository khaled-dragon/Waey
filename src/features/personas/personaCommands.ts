import { invoke } from "@tauri-apps/api/core";
import type { Persona, PersonaDraft } from "../../shared/types";

export function listPromptPersonas() {
  return invoke<Persona[]>("list_prompt_personas");
}

export function savePromptPersona(persona: PersonaDraft) {
  return invoke<Persona>("save_prompt_persona", { persona });
}

export function deletePromptPersona(personaId: string) {
  return invoke<void>("delete_prompt_persona", { personaId });
}
