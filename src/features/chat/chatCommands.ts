import { invoke } from "@tauri-apps/api/core";
import type { ChatMessage, LlmProvider, Persona, ScreenCapture, UiContextSnapshot } from "../../shared/types";

interface SendPromptInput {
  provider: LlmProvider;
  prompt: string;
  capture: ScreenCapture | null;
  captures?: ScreenCapture[];
  capturePath?: string | null;
  capturePaths?: string[];
  uiContexts?: UiContextSnapshot[];
  persona: Persona | null;
  requestId: string;
  historyMessages: ChatMessage[];
}

export function sendLlmPrompt({
  provider,
  prompt,
  capture,
  captures,
  capturePath,
  capturePaths,
  uiContexts,
  persona,
  requestId,
  historyMessages,
}: SendPromptInput) {
  const attachedCapturePaths = captures?.map((attachedCapture) => attachedCapture.path) ?? capturePaths ?? [];
  const attachedUiContexts = uiContexts ?? captures
    ?.map((attachedCapture) => attachedCapture.uiContext)
    .filter((uiContext) => uiContext !== null && uiContext !== undefined);

  return invoke<void>("send_llm_prompt", {
    request: {
      requestId,
      provider,
      prompt,
      personaPrompt: persona?.prompt ?? null,
      capturePath: capture?.path ?? capturePath ?? null,
      capturePaths: attachedCapturePaths.length > 0 ? attachedCapturePaths : undefined,
      uiContexts: attachedUiContexts,
      historyMessages: historyMessages.map((message) => ({
        role: message.role,
        content: message.content,
      })),
    },
  });
}

export function cancelLlmPrompt(requestId: string) {
  return invoke<void>("cancel_llm_prompt", { requestId });
}
