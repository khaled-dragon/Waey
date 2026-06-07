export type ProviderKind = "openrouter" | "ollama" | "custom";

export interface LlmProvider {
  id: string;
  name: string;
  kind: ProviderKind;
  baseUrl: string;
  apiKey: string;
  model: string;
  managed: boolean;
}

export interface ProviderDraft {
  id?: string;
  name: string;
  kind: ProviderKind;
  baseUrl: string;
  apiKey: string;
  model: string;
}

export interface Persona {
  id: string;
  name: string;
  prompt: string;
  createdAt: number;
  updatedAt: number;
}

export interface PersonaDraft {
  id?: string;
  name: string;
  prompt: string;
}

export interface Conversation {
  id: string;
  title: string;
  createdAt: number;
  updatedAt: number;
}

export interface AppSettings {
  hotkeyOverlay: string;
  hotkeyRegion: string;
  theme: "system" | "dark" | "light";
  language: "en" | "ar";
  autoCaptureOnOverlay: boolean;
  selectedProviderId?: string;
  selectedPersonaId?: string;
}

export type ScreenCaptureSource = "fullScreen" | "region";

export interface ScreenCapture {
  path: string;
  width: number;
  height: number;
  originX: number;
  originY: number;
  source: ScreenCaptureSource;
  createdAt: number;
}

export interface ScreenCaptureError {
  source: ScreenCaptureSource;
  message: string;
  createdAt: number;
}

export interface CaptureRect {
  x: number;
  y: number;
  width: number;
  height: number;
}

export type ChatMessageRole = "user" | "assistant" | "system";

export interface ChatMessage {
  id: string;
  role: ChatMessageRole;
  content: string;
  conversationId?: string;
  capturePath?: string | null;
  createdAt: number;
}

export interface PersistedChatMessage extends ChatMessage {
  conversationId: string;
  capturePath: string | null;
}

export interface ConversationDraft {
  title: string;
}

export interface ChatMessageDraft {
  id?: string;
  conversationId: string;
  role: ChatMessageRole;
  content: string;
  capturePath?: string | null;
}

export type StreamState = "idle" | "streaming" | "error";
