import { useEffect, useState, type PointerEvent } from "react";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { OctopusMascot } from "./components/OctopusMascot";
import { ChatComposer } from "./components/ChatComposer";
import { ConversationHistoryPanel } from "./components/ConversationHistoryPanel";
import { ResponsePanel } from "./components/ResponsePanel";
import { SettingsPanel } from "./components/SettingsPanel";
import type { PersonaDraft, ProviderDraft } from "./shared/types";
import { useLlmChat } from "./features/chat";
import { RegionSelector, useScreenCaptureEvents, captureCurrentScreen } from "./features/capture";
import { useConversationHistory } from "./features/history";
import { hideOverlayWindow, useOverlayShortcuts } from "./features/overlay";
import { usePersonas } from "./features/personas";
import { useProviders } from "./features/providers";
import { useAppSettings } from "./features/settings";

function App() {
  const activeWindow = new URLSearchParams(window.location.search).get("window");
  if (activeWindow === "region-selector") return <RegionSelector />;
  return <MainOverlay />;
}

type Panel = "chat" | "history" | "settings";

function MainOverlay() {
  useOverlayShortcuts();
  const { captureError, captures, clearCaptures, latestCapture, removeCapture, setCaptureError } = useScreenCaptureEvents();
  const [activePanel, setActivePanel] = useState<Panel>("chat");
  const [captureLimitMessage, setCaptureLimitMessage] = useState<string | null>(null);
  const [showClosePrompt, setShowClosePrompt] = useState(false);

  const { isLoadingSettings, settings, settingsError, updateSettings } = useAppSettings();
  const { deleteProvider, providers, saveProvider, selectedProvider, selectedProviderId, setSelectedProviderId } = useProviders();
  const { deletePersona, personaError, personas, savePersona, selectedPersona, selectedPersonaId, setSelectedPersonaId } = usePersonas();
  const { activeConversationId, conversations, ensureConversation, historyError, loadConversation, messages, persistMessage, removeMessage, removeConversation, renameConversation, setMessages, startNewConversation } = useConversationHistory();
  const { cancelPrompt, editLastUserMessage, errorMessage, streamState, submitPrompt } = useLlmChat({ ensureConversation, messages, persistMessage, removeMessage, setMessages });

  const isRtl = settings.language === "ar";
  const isDark = settings.theme === "dark" || (settings.theme === "system" && window.matchMedia("(prefers-color-scheme: dark)").matches);

  useEffect(() => {
    const savedProviderId = settings.selectedProviderId;

    if (savedProviderId && providers.some((provider) => provider.id === savedProviderId)) {
      setSelectedProviderId(savedProviderId);
    }
  }, [providers, settings.selectedProviderId, setSelectedProviderId]);

  useEffect(() => {
    const savedPersonaId = settings.selectedPersonaId;

    if (savedPersonaId && personas.some((persona) => persona.id === savedPersonaId)) {
      setSelectedPersonaId(savedPersonaId);
    }
  }, [personas, settings.selectedPersonaId, setSelectedPersonaId]);

  useEffect(() => {
    let isMounted = true;
    let removeListener: (() => void) | null = null;

    void listen("overlay-opened", () => {
      setActivePanel("chat");
      clearCaptures();
    }).then((unlisten) => {
      if (!isMounted) {
        unlisten();
        return;
      }

      removeListener = unlisten;
    });

    return () => {
      isMounted = false;
      removeListener?.();
    };
  }, [clearCaptures]);

  function openConversation(id: string) { setActivePanel("chat"); void loadConversation(id); }
  function startFreshConversation() { setActivePanel("chat"); startNewConversation(); }
  function renameSavedConversation(id: string, title: string) { return renameConversation(id, title); }
  function hasActiveConversation() { return activeConversationId !== null || messages.length > 0; }
  function selectProvider(id: string) {
    setSelectedProviderId(id);
    void updateSettings({ ...settings, selectedProviderId: id });
  }
  function selectPersona(id: string) {
    setSelectedPersonaId(id);
    void updateSettings({ ...settings, selectedPersonaId: id });
  }
  async function saveProviderAndSelect(draft: ProviderDraft) {
    const provider = await saveProvider(draft);
    void updateSettings({ ...settings, selectedProviderId: provider.id });
  }
  async function deleteProviderAndClear(id: string) {
    await deleteProvider(id);

    if (selectedProviderId === id || settings.selectedProviderId === id) {
      void updateSettings({ ...settings, selectedProviderId: "" });
    }
  }
  async function savePersonaAndSelect(draft: PersonaDraft) {
    const persona = await savePersona(draft);
    void updateSettings({ ...settings, selectedPersonaId: persona.id });
  }
  async function deletePersonaAndClear(id: string) {
    await deletePersona(id);

    if (selectedPersonaId === id || settings.selectedPersonaId === id) {
      void updateSettings({ ...settings, selectedPersonaId: "" });
    }
  }

  async function attachScreenCapture() {
    if (captures.length >= 3) {
      const message = isRtl ? "الحد الأقصى 3 صور" : "Maximum 3 screenshots";

      setCaptureLimitMessage(message);
      setTimeout(() => setCaptureLimitMessage(null), 1800);
      return;
    }

    setCaptureLimitMessage(null);
    setCaptureError(null);
    await captureCurrentScreen().catch(() => undefined);
  }

  const appWindow = getCurrentWindow();

  function handleTitlebarPointerDown(event: PointerEvent<HTMLDivElement>) {
    if (event.button !== 0) return;
    if ((event.target as HTMLElement).closest(".titlebar-controls")) return;

    void appWindow.startDragging();
  }

  function requestCloseOverlay() {
    if (hasActiveConversation()) {
      setShowClosePrompt(true);
      return;
    }

    void hideOverlayWindow();
  }

  function keepChatAndClose() {
    setShowClosePrompt(false);
    void hideOverlayWindow();
  }

  function endChatAndClose() {
    setShowClosePrompt(false);
    startNewConversation();
    clearCaptures();
    void hideOverlayWindow();
  }

  return (
    <div className={`app-shell ${isDark ? "theme-dark" : "theme-light"}`} dir={isRtl ? "rtl" : "ltr"}>
      <div
        className="titlebar"
        onPointerDownCapture={handleTitlebarPointerDown}
        onDoubleClick={(event) => {
          if ((event.target as HTMLElement).closest(".titlebar-controls")) return;
          void appWindow.toggleMaximize();
        }}
      >
        <div className="titlebar-left">
          <OctopusMascot size={26} state={streamState === "streaming" ? "thinking" : "idle"} />
          <span className="app-name">Waey</span>
          <span className="app-tagline">Screen-aware AI</span>
        </div>
        <div className="titlebar-controls">
          <button className="ctrl-btn ctrl-close" onClick={requestCloseOverlay} title="Close" type="button">
            <svg width="10" height="10" viewBox="0 0 10 10"><path d="M1 1l8 8M9 1L1 9" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round"/></svg>
          </button>
          <button className="ctrl-btn" onClick={() => void appWindow.toggleMaximize()} title="Maximize / Restore" type="button">
            <svg width="10" height="10" viewBox="0 0 10 10"><rect x="1.5" y="1.5" width="7" height="7" rx="1" stroke="currentColor" strokeWidth="1.4" fill="none"/></svg>
          </button>
        </div>
      </div>

      <div className="toolbar">
        <button className="screenshot-btn" onClick={() => void attachScreenCapture()} type="button">
          <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
            <path d="M23 19a2 2 0 01-2 2H3a2 2 0 01-2-2V8a2 2 0 012-2h4l2-3h6l2 3h4a2 2 0 012 2z"/>
            <circle cx="12" cy="13" r="4"/>
          </svg>
          <span>{captures.length > 0 ? (isRtl ? "إضافة لقطة شاشة" : "Add Screenshot") : (isRtl ? "إرفاق لقطة شاشة" : "Attach Screenshot")}</span>
          <span className="screenshot-count">{captures.length}/3</span>
        </button>
        {captureLimitMessage && <div className="capture-toast">{captureLimitMessage}</div>}
      </div>

      <div className="app-body">
        <nav className="sidebar-nav">
          <button className={`nav-item ${activePanel === "chat" ? "nav-item--active" : ""}`} onClick={() => setActivePanel("chat")} title={isRtl ? "المحادثة" : "Chat"}>
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round"><path d="M21 15a2 2 0 01-2 2H7l-4 4V5a2 2 0 012-2h14a2 2 0 012 2z"/></svg>
          </button>
          <button className={`nav-item ${activePanel === "history" ? "nav-item--active" : ""}`} onClick={() => setActivePanel("history")} title={isRtl ? "السجل" : "History"}>
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round"><circle cx="12" cy="12" r="10"/><polyline points="12 6 12 12 16 14"/></svg>
          </button>
          <button className={`nav-item nav-item--bottom ${activePanel === "settings" ? "nav-item--active" : ""}`} onClick={() => setActivePanel("settings")} title={isRtl ? "الإعدادات" : "Settings"}>
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round"><circle cx="12" cy="12" r="3"/><path d="M19.4 15a1.65 1.65 0 00.33 1.82l.06.06a2 2 0 010 2.83 2 2 0 01-2.83 0l-.06-.06a1.65 1.65 0 00-1.82-.33 1.65 1.65 0 00-1 1.51V21a2 2 0 01-4 0v-.09A1.65 1.65 0 009 19.4a1.65 1.65 0 00-1.82.33l-.06.06a2 2 0 01-2.83 0 2 2 0 010-2.83l.06-.06A1.65 1.65 0 004.68 15a1.65 1.65 0 00-1.51-1H3a2 2 0 010-4h.09A1.65 1.65 0 004.6 9a1.65 1.65 0 00-.33-1.82l-.06-.06a2 2 0 010-2.83 2 2 0 012.83 0l.06.06A1.65 1.65 0 009 4.68a1.65 1.65 0 001-1.51V3a2 2 0 014 0v.09a1.65 1.65 0 001 1.51 1.65 1.65 0 001.82-.33l.06-.06a2 2 0 012.83 0 2 2 0 010 2.83l-.06.06A1.65 1.65 0 0019.4 9a1.65 1.65 0 001.51 1H21a2 2 0 010 4h-.09a1.65 1.65 0 00-1.51 1z"/></svg>
          </button>
        </nav>

        <main className="main-content">
          {activePanel === "history" ? (
            <ConversationHistoryPanel
              activeConversationId={activeConversationId}
              conversations={conversations}
              onDeleteConversation={removeConversation}
              onOpenConversation={openConversation}
              onRenameConversation={renameSavedConversation}
              onStartNewConversation={startFreshConversation}
              isRtl={isRtl}
            />
          ) : activePanel === "settings" ? (
            <SettingsPanel
              errorMessage={settingsError}
              isLoading={isLoadingSettings}
              onChangeSettings={updateSettings}
              settings={settings}
              providers={providers}
              personas={personas}
              selectedProviderId={selectedProviderId}
              selectedPersonaId={selectedPersonaId}
              onSelectProvider={selectProvider}
              onSelectPersona={selectPersona}
              onSaveProvider={saveProviderAndSelect}
              onDeleteProvider={deleteProviderAndClear}
              onSavePersona={savePersonaAndSelect}
              onDeletePersona={deletePersonaAndClear}
              isRtl={isRtl}
            />
          ) : (
            <div className="chat-layout">
              <ResponsePanel
                capture={latestCapture}
                captures={captures}
                errorMessage={errorMessage ?? captureError ?? historyError ?? personaError}
                messages={messages}
                onEditLastUserMessage={(messageId, prompt) => {
                  if (!selectedProvider) {
                    setActivePanel("settings");
                    return Promise.resolve();
                  }

                  return editLastUserMessage(messageId, prompt, selectedProvider, selectedPersona);
                }}
                onRemoveCapture={removeCapture}
                streamState={streamState}
                isRtl={isRtl}
              />
              <ChatComposer
                onCancelPrompt={cancelPrompt}
                onSubmitPrompt={(prompt) => {
                  if (!selectedProvider) {
                    setActivePanel("settings");
                    return Promise.resolve();
                  }
                  setActivePanel("chat");
                  return submitPrompt(prompt, selectedProvider, captures, selectedPersona);
                }}
                streamState={streamState}
                isRtl={isRtl}
              />
            </div>
          )}
        </main>
      </div>
      {showClosePrompt && (
        <div className="close-chat-backdrop" role="presentation">
          <div className="close-chat-dialog" role="dialog" aria-modal="true" aria-labelledby="close-chat-title">
            <div className="close-chat-title" id="close-chat-title">
              {isRtl ? "إغلاق Waey؟" : "Close Waey?"}
            </div>
            <div className="close-chat-copy">
              {isRtl ? "هل تريد الرجوع لنفس الشات لاحقاً أم تبدأ شات جديد عند الفتح القادم؟" : "Keep this chat for next time, or end it and start fresh when Waey opens again?"}
            </div>
            <div className="close-chat-actions">
              <button className="btn-secondary" onClick={() => setShowClosePrompt(false)} type="button">
                {isRtl ? "إلغاء" : "Cancel"}
              </button>
              <button className="btn-secondary" onClick={endChatAndClose} type="button">
                {isRtl ? "إنهاء الشات" : "End Chat"}
              </button>
              <button className="btn-primary" onClick={keepChatAndClose} type="button">
                {isRtl ? "البقاء في الشات" : "Keep Chat"}
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

export default App;
