export type ProviderKind = "openrouter" | "ollama" | "custom";

export interface LlmProvider {
  id: string;
  name: string;
  kind: ProviderKind;
  baseUrl: string;
  apiKey: string;
  model: string;
  managed: boolean;
  supportsVision: boolean;
}

export interface ProviderDraft {
  id?: string;
  name: string;
  kind: ProviderKind;
  baseUrl: string;
  apiKey: string;
  model: string;
  supportsVision: boolean;
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
  attachUiContext: boolean;
  launchOnStartup: boolean;
  selectedProviderId?: string;
  selectedPersonaId?: string;
  developerModeEnabled: boolean;
  developerAccessLevel: DeveloperAccessLevel;
  developerWorkspaces: string[];
}

export type DeveloperAccessLevel = "ask" | "assist" | "auto";

export interface DeveloperContextResponse {
  content: string;
  filePath?: string | null;
  status: DeveloperContextStatus;
  warnings: string[];
}

export interface DeveloperContextStatus {
  label: string;
  detail: string;
  kind: "attached" | "warning";
  filePath?: string | null;
  activeWindowTitle?: string | null;
  lineRange?: DeveloperLineRange | null;
  warnings: string[];
}

export interface DeveloperLineRange {
  start: number;
  end: number;
  total: number;
}

export interface DeveloperEditStatus {
  label: string;
  detail: string;
  kind: "applied" | "blocked";
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
  uiContext?: UiContextSnapshot | null;
}

export interface UiContextSnapshot {
  schemaVersion: number;
  platform: "windows" | "macos" | "linux";
  activeWindowTitle?: string | null;
  activeAppName?: string | null;
  selectedText?: string | null;
  selectedTextSource?: string | null;
  capturedAt: number;
  region?: CaptureRect | null;
  cursor?: ScreenContextPoint | null;
  activeWindowBounds?: CaptureRect | null;
  focusedElement?: UiElementSummary | null;
  pointedElement?: UiElementSummary | null;
  visibleWindows?: VisibleWindowSummary[];
  elements: UiElementSummary[];
  diagnostics: ScreenContextDiagnostics;
}

export interface ScreenContextPoint {
  x: number;
  y: number;
}

export interface VisibleWindowSummary {
  title: string;
  appName?: string | null;
  bounds: CaptureRect;
}

export type ScreenIntelligenceMode = "compatibility" | "shadow" | "enabled";

export type ScreenContextCollectionStatus = "complete" | "partial" | "unavailable";

export interface ScreenContextDiagnostics {
  mode: ScreenIntelligenceMode;
  status: ScreenContextCollectionStatus;
  elapsedMs: number;
  elementCount: number;
  truncated: boolean;
  warnings: string[];
}

export interface UiElementSummary {
  role: string;
  name: string;
  value?: string | null;
  selectedText?: string | null;
  automationId?: string | null;
  className?: string | null;
  bounds: CaptureRect;
  focused: boolean;
  underCursor: boolean;
  isEnabled?: boolean;
  isOffscreen?: boolean;
  depth?: number;
  childCount?: number;
  parentTrail?: string[];
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

export interface GuideTarget {
  label?: string | null;
  automationId?: string | null;
  bounds?: CaptureRect | null;
}

export interface GuideStep {
  kind: "step";
  caption: string;
  target?: GuideTarget | null;
  stepIndex: number;
  estimatedStepsLeft: number;
}

export interface GuideCompletion {
  kind: "complete";
  summary: string;
}

export type GuideResponse = GuideStep | GuideCompletion;

export type GuideTheme = "light" | "dark";

export interface GuideOverlayRequest extends GuideStep {
  theme: GuideTheme;
  isRtl: boolean;
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
