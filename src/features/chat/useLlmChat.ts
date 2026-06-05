import { listen } from "@tauri-apps/api/event";
import { useCallback, useEffect, useRef, useState, type Dispatch, type SetStateAction } from "react";
import type { ChatMessage, LlmProvider, Persona, ScreenCapture, StreamState } from "../../shared/types";
import { sendLlmPrompt } from "./chatCommands";

interface UseLlmChatOptions {
  ensureConversation: (titleSeed: string) => Promise<string>;
  messages: ChatMessage[];
  persistMessage: (message: ChatMessage, conversationId: string) => Promise<ChatMessage>;
  setMessages: Dispatch<SetStateAction<ChatMessage[]>>;
}

interface StreamTokenEvent {
  requestId: string;
  token: string;
}

interface StreamStatusEvent {
  requestId: string;
}

interface StreamErrorEvent {
  requestId: string;
  message: string;
}

export function useLlmChat({
  ensureConversation,
  messages,
  persistMessage,
  setMessages,
}: UseLlmChatOptions) {
  const activeRequestId = useRef<string | null>(null);
  const assistantMessageByRequest = useRef<Map<string, ChatMessage>>(new Map());
  const conversationByRequest = useRef<Map<string, string>>(new Map());
  const contentByRequest = useRef<Map<string, string>>(new Map());
  const [streamState, setStreamState] = useState<StreamState>("idle");
  const [errorMessage, setErrorMessage] = useState<string | null>(null);

  const submitPrompt = useCallback(
    async (
      prompt: string,
      provider: LlmProvider | null,
      capture: ScreenCapture | null,
      persona: Persona | null,
    ) => {
      const trimmedPrompt = prompt.trim();

      if (!trimmedPrompt) {
        return;
      }

      if (!provider) {
        setErrorMessage("Add and select an API provider first.");
        return;
      }

      const conversationId = await ensureConversation(trimmedPrompt);
      const requestId = crypto.randomUUID();
      const createdAt = Math.floor(Date.now() / 1000);
      const userMessage: ChatMessage = {
        id: crypto.randomUUID(),
        role: "user",
        content: trimmedPrompt,
        capturePath: capture?.path ?? null,
        createdAt,
      };
      const assistantMessage: ChatMessage = {
        id: requestId,
        role: "assistant",
        content: "",
        createdAt,
      };
      const historyMessages = messages.filter((message) => message.content.trim()).slice(-12);

      activeRequestId.current = requestId;
      assistantMessageByRequest.current.set(requestId, assistantMessage);
      conversationByRequest.current.set(requestId, conversationId);
      contentByRequest.current.set(requestId, "");
      setStreamState("streaming");
      setErrorMessage(null);
      setMessages((currentMessages) => [...currentMessages, userMessage, assistantMessage]);

      try {
        await persistMessage(userMessage, conversationId);
        await sendLlmPrompt({
          provider,
          prompt: trimmedPrompt,
          capture,
          persona,
          requestId,
          historyMessages,
        });
      } catch (error) {
        setErrorMessage(error instanceof Error ? error.message : String(error));
        setStreamState("error");
      }
    },
    [ensureConversation, messages, persistMessage, setMessages],
  );

  useEffect(() => {
    const pendingListeners = [
      listen<StreamTokenEvent>("llm-stream-token", (event) => {
        if (event.payload.requestId !== activeRequestId.current) {
          return;
        }

        const currentContent = contentByRequest.current.get(event.payload.requestId) ?? "";
        const nextContent = currentContent + event.payload.token;
        contentByRequest.current.set(event.payload.requestId, nextContent);

        setMessages((currentMessages) =>
          currentMessages.map((message) =>
            message.id === event.payload.requestId
              ? { ...message, content: nextContent }
              : message,
          ),
        );
      }),
      listen<StreamStatusEvent>("llm-stream-done", (event) => {
        if (event.payload.requestId !== activeRequestId.current) {
          return;
        }

        const conversationId = conversationByRequest.current.get(event.payload.requestId);
        const assistantMessage = assistantMessageByRequest.current.get(event.payload.requestId);
        const content = contentByRequest.current.get(event.payload.requestId) ?? "";

        if (conversationId && assistantMessage && content.trim()) {
          void persistMessage({ ...assistantMessage, content }, conversationId);
        }

        cleanupRequest(event.payload.requestId);
        setStreamState("idle");
      }),
      listen<StreamErrorEvent>("llm-stream-error", (event) => {
        if (event.payload.requestId !== activeRequestId.current) {
          return;
        }

        cleanupRequest(event.payload.requestId);
        setErrorMessage(event.payload.message);
        setStreamState("error");
      }),
    ];

    return () => {
      void Promise.all(pendingListeners).then((listeners) => {
        listeners.forEach((removeListener) => removeListener());
      });
    };
  }, [persistMessage, setMessages]);

  function cleanupRequest(requestId: string) {
    activeRequestId.current = null;
    assistantMessageByRequest.current.delete(requestId);
    conversationByRequest.current.delete(requestId);
    contentByRequest.current.delete(requestId);
  }

  return {
    errorMessage,
    streamState,
    submitPrompt,
  };
}
