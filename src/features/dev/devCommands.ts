import { invoke } from "@tauri-apps/api/core";
import type { DeveloperContextResponse, UiContextSnapshot } from "../../shared/types";

interface BuildDeveloperContextInput {
  approved: boolean;
  prompt: string;
  uiContexts: UiContextSnapshot[];
}

export function buildDeveloperContext(input: BuildDeveloperContextInput) {
  return invoke<DeveloperContextResponse | null>("build_developer_context", {
    request: input,
  });
}

export function writeDeveloperFile(path: string, content: string, approved: boolean) {
  return invoke<void>("write_developer_file", {
    request: { path, content, approved },
  });
}

export function applyDeveloperSpreadsheetEdit(content: string, approved: boolean) {
  return invoke<void>("apply_developer_spreadsheet_edit", {
    request: { content, approved },
  });
}
