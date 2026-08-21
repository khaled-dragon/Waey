import { useCallback, useEffect, useRef, useState, type PointerEvent } from "react";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { OctopusMascot } from "./components/OctopusMascot";
import { GuideOverlay } from "./components/GuideOverlay";
import { GuideHighlight } from "./components/GuideHighlight";
import { ChatComposer } from "./components/ChatComposer";
import { ConversationHistoryPanel } from "./components/ConversationHistoryPanel";
import { ResponsePanel } from "./components/ResponsePanel";
import { SettingsPanel } from "./components/SettingsPanel";
import type { DeveloperContextStatus, DeveloperEditStatus, GuideStep, PersonaDraft, ProviderDraft, UiContextSnapshot } from "./shared/types";
import { useLlmChat } from "./features/chat";
import { RegionSelector, useScreenCaptureEvents, captureCurrentScreen, captureCurrentUiContext } from "./features/capture";
import { applyDeveloperSpreadsheetEdit, buildDeveloperContext, writeDeveloperFile, type DeveloperFileAction } from "./features/dev";
import { useConversationHistory } from "./features/history";
import { hideOverlayWindow, useOverlayShortcuts } from "./features/overlay";
import { usePersonas } from "./features/personas";
import { useProviders } from "./features/providers";
import { useAppSettings } from "./features/settings";
import { useAppUpdates } from "./features/updates";
import { useGuideSession } from "./features/guide";
import { showGuideStep } from "./features/guide/guideCommands";

function App() {
  const activeWindow = new URLSearchParams(window.location.search).get("window");
  if (activeWindow === "region-selector") return <RegionSelector />;
  if (activeWindow === "guide-overlay") return <GuideOverlay />;
  if (activeWindow === "guide-highlight") return <GuideHighlight />;
  return <MainOverlay />;
}

type Panel = "chat" | "history" | "settings";

const UI_CONTEXT_REUSE_WINDOW_MS = 2_500;

function MainOverlay() {
  useOverlayShortcuts();
  const { captureError, captures, clearCaptures, latestCapture, latestUiContexts, recordUiContext, removeCapture, setCaptureError } = useScreenCaptureEvents();
  const [activePanel, setActivePanel] = useState<Panel>("chat");
  const [captureLimitMessage, setCaptureLimitMessage] = useState<string | null>(null);
  const [showClosePrompt, setShowClosePrompt] = useState(false);
  const [guideComposerFocusKey, setGuideComposerFocusKey] = useState(0);
  const [guideUiContext, setGuideUiContext] = useState<UiContextSnapshot | null>(null);
  const [developerContextStatus, setDeveloperContextStatus] = useState<DeveloperContextStatus | null>(null);
  const [developerEditStatus, setDeveloperEditStatus] = useState<DeveloperEditStatus | null>(null);
  const pendingUiContextCapture = useRef<Promise<UiContextSnapshot | null> | null>(null);

  const { isLoadingSettings, settings, settingsError, updateSettings } = useAppSettings();
  const { checkForUpdate, dismissUpdate, installUpdate, updateState } = useAppUpdates();
  const { applyManagedUpdate, deleteProvider, dismissManagedUpdate, pendingManagedProviderUpdate, providers, saveProvider, selectedProvider, selectedProviderId, setSelectedProviderId } = useProviders();
  const { deletePersona, personaError, personas, savePersona, selectedPersona, selectedPersonaId, setSelectedPersonaId } = usePersonas();
  const { activeConversationId, conversations, ensureConversation, historyError, loadConversation, messages, pinConversation, persistMessage, removeMessage, removeConversation, renameConversation, setMessages, startNewConversation } = useConversationHistory();
  const { cancelPrompt, editLastUserMessage, errorMessage, streamState, submitPrompt, workedForMs } = useLlmChat({ ensureConversation, messages, persistMessage, removeMessage, setMessages });

  const isRtl = settings.language === "ar";
  const isDark = settings.theme === "dark" || (settings.theme === "system" && window.matchMedia("(prefers-color-scheme: dark)").matches);

  const captureFreshUiContext = useCallback(async (allowClipboardSelection = false) => {
    if (!allowClipboardSelection && pendingUiContextCapture.current) {
      return pendingUiContextCapture.current;
    }

    const capturePromise = captureCurrentUiContext(allowClipboardSelection)
      .then((context) => {
        if (context) {
          recordUiContext(context);
          setCaptureError(null);
        }

        return context;
      });

    if (!allowClipboardSelection) {
      pendingUiContextCapture.current = capturePromise;
      void capturePromise.finally(() => {
        if (pendingUiContextCapture.current === capturePromise) {
          pendingUiContextCapture.current = null;
        }
      });
    }

    return capturePromise;
  }, [recordUiContext, setCaptureError]);

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
      if (settings.attachUiContext) {
        void captureFreshUiContext().catch(() => undefined);
      }
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
  }, [captureFreshUiContext, clearCaptures, settings.attachUiContext]);

  function clearDeveloperStatuses() {
    setDeveloperContextStatus(null);
    setDeveloperEditStatus(null);
  }

  function openConversation(id: string) { setActivePanel("chat"); clearDeveloperStatuses(); void loadConversation(id); }
  function startFreshConversation() { setActivePanel("chat"); clearDeveloperStatuses(); startNewConversation(); }
  function renameSavedConversation(id: string, title: string) { return renameConversation(id, title); }
  function pinSavedConversation(id: string, pinned: boolean) { return pinConversation(id, pinned); }
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

  const freshPromptUiContexts = useCallback(async (allowClipboardSelection = false) => {
    if (!settings.attachUiContext) {
      return [];
    }

    const latestContext = latestUiContexts
      .slice()
      .sort((left, right) => right.capturedAt - left.capturedAt)[0] ?? null;

    if (!allowClipboardSelection
      && latestContext
      && Date.now() - latestContext.capturedAt <= UI_CONTEXT_REUSE_WINDOW_MS) {
      return [latestContext];
    }

    let freshContext: UiContextSnapshot | null = null;

    try {
      freshContext = await captureFreshUiContext(allowClipboardSelection);
    } catch (error) {
      const detail = error instanceof Error ? error.message : String(error);
      setCaptureError(isRtl
        ? `تعذر على Waey قراءة عناصر الشاشة لهذه الرسالة. السبب: ${detail}`
        : `Waey could not read the current screen structure for this message. Reason: ${detail}`);
      return [];
    }

    if (freshContext) {
      return [freshContext];
    }

    setCaptureError(isRtl
      ? "تعذر على Waey قراءة عناصر الشاشة لهذه الرسالة. لن يستخدم لقطة قديمة."
      : "Waey could not read the current screen structure for this message, so it will not use stale context.");
    return [];
  }, [captureFreshUiContext, isRtl, latestUiContexts, setCaptureError, settings.attachUiContext]);

  const continueGuide = useCallback(async (step: GuideStep, onContextCaptured: () => void) => {
    if (!selectedProvider) {
      throw new Error("Select an API provider to continue the guide.");
    }

    clearCaptures();
    const uiContexts = await freshPromptUiContexts();
    setGuideUiContext(uiContexts[0] ?? null);
    onContextCaptured();
    const continuationPrompt = `The user confirmed guide step ${step.stepIndex}. Inspect the fresh screen context and provide exactly one next guide step, or complete the guide if the task is finished.`;

    await submitPrompt(
      continuationPrompt,
      selectedProvider,
      [],
      selectedPersona,
      continuationPrompt,
      uiContexts,
      null,
      true,
      true,
    );
  }, [clearCaptures, freshPromptUiContexts, selectedPersona, selectedProvider, submitPrompt]);

  const handleGuideAdjustmentRequested = useCallback(() => {
    setActivePanel("chat");
    setGuideComposerFocusKey((currentKey) => currentKey + 1);
  }, []);

  const { beginGuide, beginGuideAdjustmentFollowUp, cancelGuide } = useGuideSession({
    isDark,
    isRtl,
    messages,
    uiContext: guideUiContext,
    onContinueGuide: continueGuide,
    onGuideAdjustmentRequested: handleGuideAdjustmentRequested,
    onError: setCaptureError,
    streamState,
  });

  async function promptWithDeveloperContext(prompt: string) {
    clearDeveloperStatuses();
    let uiContexts = await freshPromptUiContexts();
    let developerContext: string | null = null;

    if (settings.developerModeEnabled) {
      const approved = settings.developerAccessLevel !== "ask" || window.confirm("Allow Waey to read code context from your allowed workspaces for this prompt?");

      if (approved) {
        if (shouldOfferClipboardSelection(prompt, uiContexts) && window.confirm("Waey could not read the current selection through UI accessibility. Allow it to briefly copy the selected text? Your clipboard will be restored.")) {
          uiContexts = await freshPromptUiContexts(true);
        }

        const attachment = await buildDeveloperContext({
          approved,
          prompt,
          uiContexts,
        }).catch((error) => {
          setCaptureError(error instanceof Error ? error.message : String(error));
          return null;
        });

        if (attachment?.content.trim()) {
          developerContext = attachment.content;
          setDeveloperContextStatus(attachment.status);
        }
      }
    }

    return { developerContext, uiContexts };
  }

  async function submitPromptWithDeveloperContext(prompt: string, guideMode = false) {
    if (!selectedProvider) {
      setActivePanel("settings");
      return;
    }

    const adjustmentStep = guideMode ? null : beginGuideAdjustmentFollowUp();

    setActivePanel("chat");
    if (guideMode) {
      beginGuide();
    }
    const { developerContext, uiContexts } = await promptWithDeveloperContext(prompt);
    if (guideMode || adjustmentStep) {
      setGuideUiContext(uiContexts[0] ?? null);
    }

    if (adjustmentStep) {
      try {
        await showGuideStep({
          mode: "thinking",
          caption: isRtl
            ? "Waey يفحص الشاشة ويعيد ضبط الإرشاد."
            : "Waey is checking the screen and adjusting the guide.",
          target: null,
          stepIndex: 0,
          estimatedStepsLeft: 0,
          theme: isDark ? "dark" : "light",
          isRtl,
        });
      } catch (error) {
        await cancelGuide();
        setCaptureError(error instanceof Error ? error.message : String(error));
        return;
      }

      const continuationPrompt = `The user needs a different route for guide step ${adjustmentStep.stepIndex}. Their clarification is: ${prompt}\nInspect the fresh screen observation and provide exactly one next guide step, or complete the guide if the task is finished.`;

      return submitPrompt(
        prompt,
        selectedProvider,
        [],
        selectedPersona,
        continuationPrompt,
        uiContexts,
        developerContext,
        true,
        true,
      );
    }

    return submitPrompt(prompt, selectedProvider, captures, selectedPersona, prompt, uiContexts, developerContext, guideMode);
  }

  async function editLastUserMessageWithDeveloperContext(messageId: string, prompt: string) {
    if (!selectedProvider) {
      setActivePanel("settings");
      return;
    }

    setActivePanel("chat");
    const { developerContext, uiContexts } = await promptWithDeveloperContext(prompt);

    return editLastUserMessage(messageId, prompt, selectedProvider, selectedPersona, prompt, uiContexts, developerContext);
  }

  async function applyDeveloperEdit(action: DeveloperFileAction) {
    const verb = action.operation === "create" ? "Create" : "Apply this edit to";
    const approved = settings.developerAccessLevel === "auto" || window.confirm(`${verb} ${action.path}?`);

    if (!approved) {
      return;
    }

    await writeDeveloperFile(action, approved)
      .then(() => {
        setDeveloperEditStatus({
          label: action.operation === "create" ? "Created file" : "Applied edit",
          detail: action.path,
          kind: "applied",
        });
      })
      .catch((error) => {
        const message = error instanceof Error ? error.message : String(error);
        setDeveloperEditStatus({
          label: "Edit blocked",
          detail: message,
          kind: "blocked",
        });
        setCaptureError(message);
      });
  }

  async function applySpreadsheetEdit(content: string) {
    const approved = settings.developerAccessLevel === "auto" || window.confirm("Apply this spreadsheet edit?");

    if (!approved) {
      return;
    }

    await applyDeveloperSpreadsheetEdit(content, approved)
      .then(() => {
        setDeveloperEditStatus({
          label: "Applied spreadsheet edit",
          detail: "Workbook updated",
          kind: "applied",
        });
      })
      .catch((error) => {
        const message = error instanceof Error ? error.message : String(error);
        setDeveloperEditStatus({
          label: "Spreadsheet edit blocked",
          detail: message,
          kind: "blocked",
        });
        setCaptureError(message);
      });
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
    clearDeveloperStatuses();
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
              onPinConversation={pinSavedConversation}
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
              updateState={updateState}
              onCheckForUpdate={() => checkForUpdate(true)}
              onInstallUpdate={installUpdate}
            />
          ) : (
            <div className="chat-layout">
              <ResponsePanel
                capture={latestCapture}
                captures={captures}
                errorMessage={errorMessage ?? captureError ?? historyError ?? personaError}
                messages={messages}
                developerContextStatus={developerContextStatus}
                developerEditStatus={developerEditStatus}
                onEditLastUserMessage={editLastUserMessageWithDeveloperContext}
                onApplyDeveloperEdit={applyDeveloperEdit}
                onApplySpreadsheetEdit={applySpreadsheetEdit}
                onRemoveCapture={removeCapture}
                streamState={streamState}
                isRtl={isRtl}
                workedForMs={workedForMs}
                developerModeEnabled={settings.developerModeEnabled}
                developerAccessLevel={settings.developerAccessLevel}
              />
              <ChatComposer
                onCancelPrompt={async () => {
                  await cancelPrompt();
                  await cancelGuide();
                }}
                onSubmitPrompt={submitPromptWithDeveloperContext}
                focusPromptKey={guideComposerFocusKey}
                streamState={streamState}
                isRtl={isRtl}
                developerModeEnabled={settings.developerModeEnabled}
                developerAccessLevel={settings.developerAccessLevel}
                developerWorkspaces={settings.developerWorkspaces}
                onChangeDeveloperAccessLevel={(developerAccessLevel) => {
                  void updateSettings({ ...settings, developerAccessLevel });
                }}
                onChangeDeveloperWorkspaces={(developerWorkspaces) => updateSettings({ ...settings, developerWorkspaces })}
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
      {pendingManagedProviderUpdate && (
        <div className="close-chat-backdrop" role="presentation">
          <div className="close-chat-dialog" role="dialog" aria-modal="true" aria-labelledby="provider-update-title">
            <div className="close-chat-title" id="provider-update-title">
              {isRtl ? "تحديث مقترح لمزود Waey" : "Waey provider update"}
            </div>
            <div className="close-chat-copy">
              {pendingManagedProviderUpdate.message?.trim() ||
                (isRtl
                  ? `فيه تحديث جديد لمزود Waey الافتراضي. الموديل المقترح الآن هو ${pendingManagedProviderUpdate.provider.model}. التحديث لا يغير أي مزود أنت ضايفه بنفسك.`
                  : `A new managed Waey provider update is available. The recommended model is now ${pendingManagedProviderUpdate.provider.model}. Your custom providers will not be changed.`)}
            </div>
            <div className="close-chat-actions">
              <button className="btn-secondary" onClick={dismissManagedUpdate} type="button">
                {isRtl ? "لاحقا" : "Later"}
              </button>
              <button className="btn-primary" onClick={() => void applyManagedUpdate()} type="button">
                {isRtl ? "تحديث Waey" : "Update Waey"}
              </button>
            </div>
          </div>
        </div>
      )}
      {updateState.status === "available" && !pendingManagedProviderUpdate && (
        <div className="close-chat-backdrop" role="presentation">
          <div className="close-chat-dialog" role="dialog" aria-modal="true" aria-labelledby="app-update-title">
            <div className="close-chat-title" id="app-update-title">
              {isRtl ? "Waey update available" : "Waey update available"}
            </div>
            <div className="close-chat-copy">
              {updateState.latestVersion
                ? `Version ${updateState.latestVersion} is ready. You can install it now, or skip and check again from Settings.`
                : "A new Waey version is ready. You can install it now, or skip and check again from Settings."}
            </div>
            <div className="close-chat-actions">
              <button className="btn-secondary" onClick={dismissUpdate} type="button">
                Later
              </button>
              <button className="btn-primary" onClick={() => void installUpdate()} type="button">
                Update
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

function shouldOfferClipboardSelection(prompt: string, contexts: UiContextSnapshot[]) {
  const hasSelection = contexts.some((context) =>
    Boolean(context.selectedText?.trim() || context.elements.some((element) => element.selectedText?.trim())),
  );

  if (hasSelection) {
    return false;
  }

  const promptSuggestsSelection = /\b(select|selection|line|code|edit|fix)\b|حدد|المحدد|السطر|الكود/i.test(prompt);
  const activeAppLooksLikeIde = contexts.some((context) =>
    /code|cursor|windsurf|visual studio|jetbrains|idea/i.test(`${context.activeAppName ?? ""} ${context.activeWindowTitle ?? ""}`),
  );

  return promptSuggestsSelection || activeAppLooksLikeIde;
}

export default App;
