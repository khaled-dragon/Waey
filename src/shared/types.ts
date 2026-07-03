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

export interface ManagedProviderUpdate {
  provider: LlmProvider;
  message?: string | null;
  revision?: number | null;
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
  pinned: boolean;
  createdAt: number;
  updatedAt: number;
}

export interface AppSettings {
  hotkeyOverlay: string;
  hotkeyRegion: string;
  theme: "system" | "dark" | "light";
  language: "en" | "ar";
  autoCaptureOnOverlay: boolean;
  launchOnStartup: boolean;
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
  reasoningContent?: string;
  conversationId?: string;
  capturePath?: string | null;
  capturePaths?: string[];
  createdAt: number;
}

export interface PersistedChatMessage extends ChatMessage {
  conversationId: string;
  capturePath: string | null;
  capturePaths: string[];
}

export interface ConversationDraft {
  title: string;
}

export interface ConversationRenameDraft {
  conversationId: string;
  title: string;
}

export interface ConversationPinDraft {
  conversationId: string;
  pinned: boolean;
}

export interface ChatMessageDraft {
  id?: string;
  conversationId: string;
  role: ChatMessageRole;
  content: string;
  capturePath?: string | null;
  capturePaths?: string[];
}

export type StreamState = "idle" | "streaming" | "error";
