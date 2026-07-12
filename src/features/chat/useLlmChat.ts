import { listen } from "@tauri-apps/api/event";
import { useCallback, useEffect, useRef, useState, type Dispatch, type SetStateAction } from "react";
import type { ChatMessage, LlmProvider, Persona, ScreenCapture, StreamState, UiContextSnapshot } from "../../shared/types";
import { cancelLlmPrompt, sendLlmPrompt } from "./chatCommands";

interface UseLlmChatOptions {
  ensureConversation: (titleSeed: string) => Promise<string>;
  messages: ChatMessage[];
  persistMessage: (message: ChatMessage, conversationId: string) => Promise<ChatMessage>;
  removeMessage: (messageId: string) => Promise<void>;
  setMessages: Dispatch<SetStateAction<ChatMessage[]>>;
}

interface StreamTokenEvent {
  requestId: string;
  token: string;
}

interface StreamStatusEvent {
  requestId: string;
  finishReason?: string | null;
}

interface StreamErrorEvent {
  requestId: string;
  message: string;
}

export function useLlmChat({
  ensureConversation,
  messages,
  persistMessage,
  removeMessage,
  setMessages,
}: UseLlmChatOptions) {
  const activeRequestId = useRef<string | null>(null);
  const assistantMessageByRequest = useRef<Map<string, ChatMessage>>(new Map());
  const conversationByRequest = useRef<Map<string, string>>(new Map());
  const contentByRequest = useRef<Map<string, string>>(new Map());
  const reasoningByRequest = useRef<Map<string, string>>(new Map());
  const startedAtByRequest = useRef<Map<string, number>>(new Map());
  const [activeStreamingRequestId, setActiveStreamingRequestId] = useState<string | null>(null);
  const [streamState, setStreamState] = useState<StreamState>("idle");
  const [errorMessage, setErrorMessage] = useState<string | null>(null);
  const [workedForMs, setWorkedForMs] = useState<number | null>(null);

  const submitPrompt = useCallback(
    async (
      prompt: string,
      provider: LlmProvider | null,
      captures: ScreenCapture[],
      persona: Persona | null,
      requestPrompt?: string,
      uiContexts?: UiContextSnapshot[],
    ) => {
      const trimmedPrompt = prompt.trim();
      const trimmedRequestPrompt = (requestPrompt ?? prompt).trim();

      if (!trimmedPrompt || !trimmedRequestPrompt) {
        return;
      }

      if (!provider) {
        setErrorMessage("Add and select an API provider first.");
        return;
      }

      const conversationId = await ensureConversation(trimmedPrompt);
      const requestId = crypto.randomUUID();
      const createdAt = Math.floor(Date.now() / 1000);
      const capturePaths = captures.map((capture) => capture.path).slice(0, 3);
      const userMessage: ChatMessage = {
        id: crypto.randomUUID(),
        role: "user",
        content: trimmedPrompt,
        capturePath: capturePaths[0] ?? null,
        capturePaths,
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
      reasoningByRequest.current.set(requestId, "");
      startedAtByRequest.current.set(requestId, Date.now());
      setActiveStreamingRequestId(requestId);
      setStreamState("streaming");
      setWorkedForMs(0);
      setErrorMessage(null);
      setMessages((currentMessages) => [...currentMessages, userMessage, assistantMessage]);

      try {
        await persistMessage(userMessage, conversationId);
        await sendLlmPrompt({
          provider,
          prompt: trimmedRequestPrompt,
          capture: captures[0] ?? null,
          captures,
          uiContexts,
          persona,
          requestId,
          historyMessages,
        });
      } catch (error) {
        if (activeRequestId.current !== requestId) {
          return;
        }

        setErrorMessage(error instanceof Error ? error.message : String(error));
        setStreamState("error");
      }
    },
    [ensureConversation, messages, persistMessage, setMessages],
  );

  const editLastUserMessage = useCallback(
    async (
      messageId: string,
      prompt: string,
      provider: LlmProvider | null,
      persona: Persona | null,
      requestPrompt?: string,
      uiContexts?: UiContextSnapshot[],
    ) => {
      const trimmedPrompt = prompt.trim();
      const trimmedRequestPrompt = (requestPrompt ?? prompt).trim();

      if (streamState === "streaming" || !trimmedPrompt || !trimmedRequestPrompt) {
        return;
      }

      if (!provider) {
        setErrorMessage("Add and select an API provider first.");
        return;
      }

      const userMessageIndex = messages.findIndex((message) => message.id === messageId);
      let lastUserMessageIndex = -1;

      for (let index = messages.length - 1; index >= 0; index -= 1) {
        if (messages[index]?.role === "user") {
          lastUserMessageIndex = index;
          break;
        }
      }

      const userMessage = messages[userMessageIndex];

      if (!userMessage || userMessage.role !== "user" || userMessageIndex !== lastUserMessageIndex) {
        setErrorMessage("Only the latest sent message can be edited.");
        return;
      }

      const conversationId = userMessage.conversationId ?? await ensureConversation(trimmedPrompt);
      const oldAssistantMessage = messages[userMessageIndex + 1]?.role === "assistant"
        ? messages[userMessageIndex + 1]
        : null;
      const requestId = crypto.randomUUID();
      const editedUserMessage: ChatMessage = {
        ...userMessage,
        content: trimmedPrompt,
        conversationId,
      };
      const assistantMessage: ChatMessage = {
        id: requestId,
        role: "assistant",
        content: "",
        conversationId,
        createdAt: Math.floor(Date.now() / 1000),
      };
      const historyMessages = messages
        .slice(0, userMessageIndex)
        .filter((message) => message.content.trim())
        .slice(-12);

      activeRequestId.current = requestId;
      assistantMessageByRequest.current.set(requestId, assistantMessage);
      conversationByRequest.current.set(requestId, conversationId);
      contentByRequest.current.set(requestId, "");
      reasoningByRequest.current.set(requestId, "");
      startedAtByRequest.current.set(requestId, Date.now());
      setActiveStreamingRequestId(requestId);
      setStreamState("streaming");
      setWorkedForMs(0);
      setErrorMessage(null);
      setMessages((currentMessages) => {
        const nextMessages = [...currentMessages];
        const currentUserIndex = nextMessages.findIndex((message) => message.id === messageId);

        if (currentUserIndex === -1) {
          return currentMessages;
        }

        nextMessages[currentUserIndex] = editedUserMessage;

        if (oldAssistantMessage) {
          return nextMessages.map((message) =>
            message.id === oldAssistantMessage.id ? assistantMessage : message,
          );
        }

        nextMessages.splice(currentUserIndex + 1, 0, assistantMessage);
        return nextMessages;
      });

      try {
        await persistMessage(editedUserMessage, conversationId);

        if (oldAssistantMessage) {
          await removeMessage(oldAssistantMessage.id);
        }

        await sendLlmPrompt({
          provider,
          prompt: trimmedRequestPrompt,
          capture: null,
          capturePath: editedUserMessage.capturePath ?? null,
          capturePaths: editedUserMessage.capturePaths ?? [],
          uiContexts,
          persona,
          requestId,
          historyMessages,
        });
      } catch (error) {
        if (activeRequestId.current !== requestId) {
          return;
        }

        setErrorMessage(error instanceof Error ? error.message : String(error));
        setStreamState("error");
      }
    },
    [ensureConversation, messages, persistMessage, removeMessage, setMessages, streamState],
  );

  const cancelPrompt = useCallback(async () => {
    const requestId = activeRequestId.current;

    if (!requestId) {
      return;
    }

    await cancelLlmPrompt(requestId).catch(() => undefined);
    setMessages((currentMessages) => currentMessages.filter((message) => message.id !== requestId));
    cleanupRequest(requestId);
    setErrorMessage(null);
    setWorkedForMs(null);
    setStreamState("idle");
  }, [setMessages]);

  useEffect(() => {
    if (streamState !== "streaming" || !activeStreamingRequestId) {
      return;
    }

    const timer = window.setInterval(() => {
      const startedAt = startedAtByRequest.current.get(activeStreamingRequestId);

      if (startedAt) {
        setWorkedForMs(Date.now() - startedAt);
      }
    }, 500);

    return () => window.clearInterval(timer);
  }, [activeStreamingRequestId, streamState]);

  useEffect(() => {
    const pendingListeners = [
      listen<StreamTokenEvent>("llm-stream-reasoning", (event) => {
        if (event.payload.requestId !== activeRequestId.current) {
          return;
        }

        const currentReasoning = reasoningByRequest.current.get(event.payload.requestId) ?? "";
        const nextReasoning = currentReasoning + event.payload.token;
        reasoningByRequest.current.set(event.payload.requestId, nextReasoning);

        setMessages((currentMessages) =>
          currentMessages.map((message) =>
            message.id === event.payload.requestId
              ? { ...message, reasoningContent: nextReasoning }
              : message,
          ),
        );
      }),
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

        if (event.payload.finishReason === "length") {
          setErrorMessage(
            "Waey stopped because this response reached the provider output limit. Try asking for a shorter answer or continue with a follow-up.",
          );
        }

        finalizeWorkedDuration(event.payload.requestId);
        cleanupRequest(event.payload.requestId);
        setStreamState("idle");
      }),
      listen<StreamErrorEvent>("llm-stream-error", (event) => {
        if (event.payload.requestId !== activeRequestId.current) {
          return;
        }

        finalizeWorkedDuration(event.payload.requestId);
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
    setActiveStreamingRequestId(null);
    assistantMessageByRequest.current.delete(requestId);
    conversationByRequest.current.delete(requestId);
    contentByRequest.current.delete(requestId);
    reasoningByRequest.current.delete(requestId);
    startedAtByRequest.current.delete(requestId);
  }

  function finalizeWorkedDuration(requestId: string) {
    const startedAt = startedAtByRequest.current.get(requestId);

    if (startedAt) {
      setWorkedForMs(Date.now() - startedAt);
    }
  }

  return {
    activeStreamingRequestId,
    cancelPrompt,
    editLastUserMessage,
    errorMessage,
    streamState,
    submitPrompt,
    workedForMs,
  };
}
