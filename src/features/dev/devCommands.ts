import { invoke } from "@tauri-apps/api/core";
import type { DeveloperContextResponse, UiContextSnapshot } from "../../shared/types";

export interface DeveloperFileAction {
  workspace: string;
  path: string;
  content: string;
  operation: "edit" | "create";
  expectedSha256?: string | null;
  overwrite?: boolean;
}

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

export function writeDeveloperFile(action: DeveloperFileAction, approved: boolean) {
  return invoke<void>("write_developer_file", {
    request: { ...action, approved },
  });
}

export function applyDeveloperSpreadsheetEdit(content: string, approved: boolean) {
  return invoke<void>("apply_developer_spreadsheet_edit", {
    request: { content, approved },
  });
}
