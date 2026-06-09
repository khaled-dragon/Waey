import { invoke } from "@tauri-apps/api/core";
import type {
  ChatMessageDraft,
  Conversation,
  ConversationDraft,
  ConversationRenameDraft,
  PersistedChatMessage,
} from "../../shared/types";

export function listChatConversations() {
  return invoke<Conversation[]>("list_chat_conversations");
}

export function createChatConversation(draft: ConversationDraft) {
  return invoke<Conversation>("create_chat_conversation", { draft });
}

export function renameChatConversation(draft: ConversationRenameDraft) {
  return invoke<Conversation>("rename_chat_conversation", { draft });
}

export function listChatMessages(conversationId: string) {
  return invoke<PersistedChatMessage[]>("list_chat_messages", { conversationId });
}

export function saveChatMessage(message: ChatMessageDraft) {
  return invoke<PersistedChatMessage>("save_chat_message", { message });
}

export function deleteChatMessage(messageId: string) {
  return invoke<void>("delete_chat_message", { messageId });
}

export function deleteChatConversation(conversationId: string) {
  return invoke<void>("delete_chat_conversation", { conversationId });
}
