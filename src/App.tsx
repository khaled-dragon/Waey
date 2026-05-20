import { useState } from "react";
import { ActionBar } from "./components/ActionBar";
import { AssistantAvatar } from "./components/AssistantAvatar";
import { ChatComposer } from "./components/ChatComposer";
import { ConversationHistoryPanel } from "./components/ConversationHistoryPanel";
import { PersonaManager } from "./components/PersonaManager";
import { ProviderManager } from "./components/ProviderManager";
import { ResponsePanel } from "./components/ResponsePanel";
import { SettingsPanel } from "./components/SettingsPanel";
import { Sidebar } from "./components/Sidebar";
import { useLlmChat } from "./features/chat";
import { RegionSelector, useScreenCaptureEvents } from "./features/capture";
import { useConversationHistory } from "./features/history";
import { hideOverlayWindow, useOverlayShortcuts } from "./features/overlay";
import { usePersonas } from "./features/personas";
import { useProviders } from "./features/providers";
import { useAppSettings } from "./features/settings";

function App() {
  const activeWindow = new URLSearchParams(window.location.search).get("window");

  if (activeWindow === "region-selector") {
    return <RegionSelector />;
  }

  return <MainOverlay />;
}

function MainOverlay() {
  useOverlayShortcuts();
  const { latestCapture } = useScreenCaptureEvents();
  const [activePanel, setActivePanel] = useState<
    "chat" | "history" | "personas" | "providers" | "settings"
  >("chat");
  const { isLoadingSettings, settings, settingsError, updateSettings } = useAppSettings();
  const {
    deleteProvider,
    providers,
    saveProvider,
    selectedProvider,
    selectedProviderId,
    setSelectedProviderId,
  } = useProviders();
  const {
    deletePersona,
    personaError,
    personas,
    savePersona,
    selectedPersona,
    selectedPersonaId,
    setSelectedPersonaId,
  } = usePersonas();
  const {
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
  } = useConversationHistory();
  const { errorMessage, streamState, submitPrompt } = useLlmChat({
    ensureConversation,
    messages,
    persistMessage,
    setMessages,
  });

  function openConversation(conversationId: string) {
    setActivePanel("chat");
    void loadConversation(conversationId);
  }

  function startFreshConversation() {
    setActivePanel("chat");
    startNewConversation();
  }

  return (
    <main className="min-h-screen bg-transparent p-3 text-white">
      <div className="flex min-h-[calc(100vh-24px)] items-center justify-center">
        <section className="grid h-[596px] w-full overflow-hidden rounded-[28px] border border-white/10 bg-waey-ink/92 shadow-2xl shadow-black/45 backdrop-blur md:grid-cols-[220px_1fr]">
          <Sidebar
            activeConversationId={activeConversationId}
            conversations={conversations}
            onOpenConversation={openConversation}
            onStartNewConversation={startFreshConversation}
          />
          <div className="flex min-h-0 flex-col">
            <header
              className="flex items-center justify-between border-b border-white/10 px-6 py-4"
              data-tauri-drag-region
            >
              <div>
                <p className="text-xs font-semibold uppercase tracking-[0.28em] text-waey-coral">
                  Waey Desktop Assistant
                </p>
                <h1 className="mt-1 text-2xl font-semibold">Ask your screen.</h1>
              </div>
              <div className="flex items-center gap-3">
                <AssistantAvatar state="idle" />
                <button
                  className="grid size-9 place-items-center rounded-full border border-white/10 text-sm text-white/60 transition hover:border-waey-coral hover:text-white"
                  onClick={() => void hideOverlayWindow()}
                  type="button"
                >
                  x
                </button>
              </div>
            </header>

            <div className="flex flex-1 flex-col gap-5 px-6 py-5">
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
                <ResponsePanel
                  capture={latestCapture}
                  errorMessage={errorMessage ?? historyError ?? personaError}
                  messages={messages}
                  streamState={streamState}
                />
              )}
              <ChatComposer
                onSubmitPrompt={(prompt) => {
                  setActivePanel("chat");
                  return submitPrompt(prompt, selectedProvider, latestCapture, selectedPersona);
                }}
                streamState={streamState}
              />
            </div>
          </div>
        </section>
      </div>
    </main>
  );
}

export default App;
