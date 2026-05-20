import { useCallback, useEffect, useState } from "react";
import type { ChatMessage, Conversation } from "../../shared/types";
import {
  createChatConversation,
  deleteChatConversation,
  listChatConversations,
  listChatMessages,
  saveChatMessage,
} from "./historyCommands";

export function useConversationHistory() {
  const [conversations, setConversations] = useState<Conversation[]>([]);
  const [activeConversationId, setActiveConversationId] = useState<string | null>(null);
  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const [historyError, setHistoryError] = useState<string | null>(null);

  const refreshConversations = useCallback(async () => {
    try {
      setConversations(await listChatConversations());
    } catch (error) {
      setHistoryError(error instanceof Error ? error.message : String(error));
    }
  }, []);

  const loadConversation = useCallback(
    async (conversationId: string) => {
      setHistoryError(null);
      setActiveConversationId(conversationId);

      try {
        setMessages(await listChatMessages(conversationId));
        await refreshConversations();
      } catch (error) {
        setHistoryError(error instanceof Error ? error.message : String(error));
      }
    },
    [refreshConversations],
  );

  const startNewConversation = useCallback(() => {
    setActiveConversationId(null);
    setMessages([]);
    setHistoryError(null);
  }, []);

  const ensureConversation = useCallback(
    async (titleSeed: string) => {
      if (activeConversationId) {
        return activeConversationId;
      }

      const conversation = await createChatConversation({ title: titleSeed });

      setActiveConversationId(conversation.id);
      await refreshConversations();

      return conversation.id;
    },
    [activeConversationId, refreshConversations],
  );

  const persistMessage = useCallback(
    async (message: ChatMessage, conversationId: string) => {
      const savedMessage = await saveChatMessage({
        id: message.id,
        conversationId,
        role: message.role,
        content: message.content,
        capturePath: message.capturePath ?? null,
      });

      await refreshConversations();

      return savedMessage;
    },
    [refreshConversations],
  );

  const removeConversation = useCallback(
    async (conversationId: string) => {
      await deleteChatConversation(conversationId);

      if (activeConversationId === conversationId) {
        startNewConversation();
      }

      await refreshConversations();
    },
    [activeConversationId, refreshConversations, startNewConversation],
  );

  useEffect(() => {
    void refreshConversations();
  }, [refreshConversations]);

  return {
    activeConversationId,
    conversations,
    ensureConversation,
    historyError,
    loadConversation,
    messages,
    persistMessage,
    removeConversation,
    setMessages,
    startNewConversation,
  };
}
