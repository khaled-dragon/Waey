import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { ActionBar } from "./components/ActionBar";
import { ChatComposer } from "./components/ChatComposer";
import { ConversationHistoryPanel } from "./components/ConversationHistoryPanel";
import { PersonaManager } from "./components/PersonaManager";
import { ProviderManager } from "./components/ProviderManager";
import { ResponsePanel } from "./components/ResponsePanel";
import { SettingsPanel } from "./components/SettingsPanel";
import { useLlmChat } from "./features/chat";
import { RegionSelector, useScreenCaptureEvents } from "./features/capture";
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

type Panel = "chat" | "history" | "personas" | "providers" | "settings";

function MainOverlay() {
  useOverlayShortcuts();
  const { latestCapture } = useScreenCaptureEvents();
  const [activePanel, setActivePanel] = useState<Panel>("chat");

  const { isLoadingSettings, settings, settingsError, updateSettings } = useAppSettings();
  const { deleteProvider, providers, saveProvider, selectedProvider, selectedProviderId, setSelectedProviderId } = useProviders();
  const { deletePersona, personaError, personas, savePersona, selectedPersona, selectedPersonaId, setSelectedPersonaId } = usePersonas();
  const { activeConversationId, conversations, ensureConversation, historyError, loadConversation, messages, persistMessage, removeConversation, setMessages, startNewConversation } = useConversationHistory();
  const { errorMessage, streamState, submitPrompt } = useLlmChat({ ensureConversation, messages, persistMessage, setMessages });

  function openConversation(id: string) { setActivePanel("chat"); void loadConversation(id); }
  function startFreshConversation() { setActivePanel("chat"); startNewConversation(); }

  const navItems: { id: Panel; icon: string; label: string }[] = [
    { id: "chat", icon: "💬", label: "Chat" },
    { id: "history", icon: "🕐", label: "History" },
    { id: "providers", icon: "⚡", label: "Providers" },
    { id: "personas", icon: "🎭", label: "Personas" },
    { id: "settings", icon: "⚙️", label: "Settings" },
  ];

  return (
    <div className="app-shell">
      <div className="titlebar" data-tauri-drag-region>
        <div className="titlebar-left" data-tauri-drag-region>
          <div className="app-logo">
            <img src="/assets/logo.svg" alt="Waey" width="22" height="22" onError={(e) => { (e.target as HTMLImageElement).style.display = 'none'; }} />
            <div className="app-logo-fallback">W</div>
          </div>
          <span className="app-name">Waey</span>
          <span className="app-tagline">Screen-aware AI</span>
        </div>
        <div className="titlebar-controls">
          <button className="ctrl-btn ctrl-min" onClick={() => invoke("hide_overlay_window")} title="Hide">
            <svg width="10" height="2" viewBox="0 0 10 2"><rect width="10" height="2" rx="1" fill="currentColor"/></svg>
          </button>
          <button className="ctrl-btn ctrl-close" onClick={() => void hideOverlayWindow()} title="Close">
            <svg width="10" height="10" viewBox="0 0 10 10"><path d="M1 1l8 8M9 1L1 9" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round"/></svg>
          </button>
        </div>
      </div>

      <div className="app-body">
        <nav className="sidebar-nav">
          {navItems.map((item) => (
            <button
              key={item.id}
              className={`nav-item ${activePanel === item.id ? "nav-item--active" : ""}`}
              onClick={() => setActivePanel(item.id)}
              title={item.label}
            >
              <span className="nav-icon">{item.icon}</span>
              <span className="nav-label">{item.label}</span>
            </button>
          ))}
        </nav>

        <main className="main-content">
          {activePanel === "providers" ? (
            <ProviderManager
              onDeleteProvider={deleteProvider}
              onSaveProvider={saveProvider}
              onSelectProvider={setSelectedProviderId}
              providers={providers}
              selectedProviderId={selectedProviderId}
            />
          ) : activePanel === "personas" ? (
            <PersonaManager
              onDeletePersona={deletePersona}
              onSavePersona={savePersona}
              onSelectPersona={setSelectedPersonaId}
              personas={personas}
              selectedPersonaId={selectedPersonaId}
            />
          ) : activePanel === "settings" ? (
            <SettingsPanel
              errorMessage={settingsError}
              isLoading={isLoadingSettings}
              onChangeSettings={updateSettings}
              settings={settings}
            />
          ) : activePanel === "history" ? (
            <ConversationHistoryPanel
              activeConversationId={activeConversationId}
              conversations={conversations}
              onDeleteConversation={removeConversation}
              onOpenConversation={openConversation}
              onStartNewConversation={startFreshConversation}
            />
          ) : (
            <div className="chat-layout">
              <ActionBar
                onOpenHistory={() => setActivePanel("history")}
                onOpenPersonas={() => setActivePanel("personas")}
                onOpenProviders={() => setActivePanel("providers")}
                onOpenSettings={() => setActivePanel("settings")}
                onSelectPersona={setSelectedPersonaId}
                onSelectProvider={setSelectedProviderId}
                personas={personas}
                providers={providers}
                selectedPersonaId={selectedPersonaId}
                selectedProviderId={selectedProviderId}
              />
              <ResponsePanel
                capture={latestCapture}
                errorMessage={errorMessage ?? historyError ?? personaError}
                messages={messages}
                streamState={streamState}
              />
              <ChatComposer
                onSubmitPrompt={(prompt) => {
                  setActivePanel("chat");
                  return submitPrompt(prompt, selectedProvider, latestCapture, selectedPersona);
                }}
                streamState={streamState}
              />
            </div>
          )}
        </main>
      </div>
    </div>
  );
}

export default App;
