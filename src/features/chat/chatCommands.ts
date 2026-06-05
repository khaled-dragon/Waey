import { invoke } from "@tauri-apps/api/core";
import type { ChatMessage, LlmProvider, Persona, ScreenCapture } from "../../shared/types";

interface SendPromptInput {
  provider: LlmProvider;
  prompt: string;
  capture: ScreenCapture | null;
  persona: Persona | null;
  requestId: string;
  historyMessages: ChatMessage[];
}

export function sendLlmPrompt({
  provider,
  prompt,
  capture,
  persona,
  requestId,
  historyMessages,
}: SendPromptInput) {
  return invoke<void>("send_llm_prompt", {
    request: {
      requestId,
      provider,
      prompt,
      personaPrompt: persona?.prompt ?? null,
      capturePath: capture?.path ?? null,
      historyMessages: historyMessages.map((message) => ({
        role: message.role,
        content: message.content,
      })),
    },
  });
}
