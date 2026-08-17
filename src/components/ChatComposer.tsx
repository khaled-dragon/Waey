import { useEffect, useRef, useState, type FormEvent } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import type { DeveloperAccessLevel, StreamState } from "../shared/types";

interface ChatComposerProps {
  streamState: StreamState;
  onCancelPrompt: () => Promise<void>;
  onSubmitPrompt: (prompt: string, guideMode: boolean) => Promise<void>;
  isRtl: boolean;
  developerModeEnabled: boolean;
  developerAccessLevel: DeveloperAccessLevel;
  developerWorkspaces: string[];
  onChangeDeveloperAccessLevel: (accessLevel: DeveloperAccessLevel) => void;
  onChangeDeveloperWorkspaces: (workspaces: string[]) => Promise<void>;
  focusPromptKey: number;
}

const accessOptions: Array<{
  id: DeveloperAccessLevel;
  icon: string;
  label: string;
  description: string;
}> = [
  {
    id: "ask",
    icon: "A",
    label: "Ask for approval",
    description: "Ask before reading workspace files or applying edits.",
  },
  {
    id: "assist",
    icon: "P",
    label: "Approve for me",
    description: "Read allowed workspaces, ask before file edits.",
  },
  {
    id: "auto",
    icon: "F",
    label: "Full access",
    description: "Auto-apply Waey edit blocks inside allowed workspaces.",
  },
];

export function ChatComposer({
  streamState,
  onCancelPrompt,
  onSubmitPrompt,
  isRtl,
  developerModeEnabled,
  developerAccessLevel,
  developerWorkspaces,
  onChangeDeveloperAccessLevel,
  onChangeDeveloperWorkspaces,
  focusPromptKey,
}: ChatComposerProps) {
  const [prompt, setPrompt] = useState("");
  const [guideMode, setGuideMode] = useState(false);
  const [accessMenuOpen, setAccessMenuOpen] = useState(false);
  const [workspaceMenuOpen, setWorkspaceMenuOpen] = useState(false);
  const promptInputRef = useRef<HTMLTextAreaElement>(null);
  const isStreaming = streamState === "streaming";
  const activeAccess = accessOptions.find((option) => option.id === developerAccessLevel) ?? accessOptions[1];
  const activeWorkspace = developerWorkspaces[developerWorkspaces.length - 1] ?? null;

  useEffect(() => {
    if (focusPromptKey === 0) {
      return;
    }

    const animationFrame = window.requestAnimationFrame(() => promptInputRef.current?.focus());

    return () => window.cancelAnimationFrame(animationFrame);
  }, [focusPromptKey]);

  async function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (isStreaming || !prompt.trim()) return;
    await onSubmitPrompt(prompt, guideMode);
    setPrompt("");
    setGuideMode(false);
  }

  function handleKeyDown(event: React.KeyboardEvent<HTMLTextAreaElement>) {
    if (event.key === "Enter" && !event.shiftKey) {
      event.preventDefault();
      if (!isStreaming && prompt.trim()) {
        void onSubmitPrompt(prompt, guideMode).then(() => {
          setPrompt("");
          setGuideMode(false);
        });
      }
    }
  }

  async function selectDeveloperWorkspace() {
    const selectedPath = await open({
      directory: true,
      multiple: false,
      title: "Choose a Waey workspace",
    });

    if (typeof selectedPath !== "string" || selectedPath.trim().length === 0) {
      return;
    }

    const normalizedPath = selectedPath.trim();
    const nextWorkspaces = [
      ...developerWorkspaces.filter((workspace) => workspace !== normalizedPath),
      normalizedPath,
    ].slice(-8);

    await onChangeDeveloperWorkspaces(nextWorkspaces);
    setWorkspaceMenuOpen(false);
  }

  async function removeDeveloperWorkspace(workspace: string) {
    await onChangeDeveloperWorkspaces(developerWorkspaces.filter((item) => item !== workspace));
  }

  return (
    <form className="chat-composer" onSubmit={handleSubmit}>
      <button
        aria-pressed={guideMode}
        className={`guide-mode-trigger ${guideMode ? "guide-mode-trigger--active" : ""}`}
        disabled={isStreaming}
        onClick={() => setGuideMode((enabled) => !enabled)}
        title={guideMode
          ? (isRtl ? "الإرشاد خطوة بخطوة مفعّل للطلب القادم" : "Guide the next request step by step")
          : (isRtl ? "فعّل الإرشاد خطوة بخطوة" : "Enable step-by-step guide")}
        type="button"
      >
        <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
          <circle cx="12" cy="12" r="9" />
          <path d="M15 9l-2.2 4.2L9 15l2.2-4.2L15 9z" />
        </svg>
        <span>{isRtl ? "إرشاد" : "Guide"}</span>
      </button>
      {developerModeEnabled && (
        <div className="dev-access-menu-wrap">
          <button
            className="dev-workspace-button"
            onClick={() => void selectDeveloperWorkspace()}
            title={activeWorkspace ?? "Add local workspace"}
            type="button"
          >
            <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
              <path d="M3 6h6l2 2h10v10a2 2 0 0 1-2 2H3z" />
              <path d="M3 6v12a2 2 0 0 0 2 2" />
            </svg>
            <span>{activeWorkspace ? compactWorkspaceName(activeWorkspace) : (isRtl ? "إضافة مساحة" : "Add local space")}</span>
          </button>
          {developerWorkspaces.length > 0 && (
            <button
              className="dev-workspace-caret"
              onClick={() => setWorkspaceMenuOpen((isOpen) => !isOpen)}
              title="Workspace options"
              type="button"
            >
              <svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.4" strokeLinecap="round" strokeLinejoin="round">
                <path d="M6 9l6 6 6-6" />
              </svg>
            </button>
          )}
          <button
            className="dev-access-trigger"
            onClick={() => setAccessMenuOpen((isOpen) => !isOpen)}
            type="button"
          >
            <span className="dev-access-icon" aria-hidden="true">{activeAccess.icon}</span>
            <span>{activeAccess.label}</span>
            <svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.4" strokeLinecap="round" strokeLinejoin="round">
              <path d="M6 9l6 6 6-6" />
            </svg>
          </button>
          {workspaceMenuOpen && (
            <div className="dev-workspace-menu" role="menu">
              <div className="dev-access-menu-title">Local workspaces</div>
              {developerWorkspaces.map((workspace) => (
                <div className="dev-workspace-option" key={workspace}>
                  <button onClick={() => void onChangeDeveloperWorkspaces([...developerWorkspaces.filter((item) => item !== workspace), workspace])} title={workspace} type="button">
                    <span>{compactWorkspaceName(workspace)}</span>
                    <small>{workspace}</small>
                  </button>
                  <button className="dev-workspace-remove" onClick={() => void removeDeveloperWorkspace(workspace)} title="Remove workspace" type="button">x</button>
                </div>
              ))}
              <button className="dev-workspace-add" onClick={() => void selectDeveloperWorkspace()} type="button">Add another workspace</button>
            </div>
          )}
          {accessMenuOpen && (
            <div className="dev-access-menu" role="menu">
              <div className="dev-access-menu-title">How should Waey handle code actions?</div>
              {accessOptions.map((option) => (
                <button
                  className={`dev-access-option ${developerAccessLevel === option.id ? "dev-access-option--active" : ""}`}
                  key={option.id}
                  onClick={() => {
                    onChangeDeveloperAccessLevel(option.id);
                    setAccessMenuOpen(false);
                  }}
                  type="button"
                >
                  <span className="dev-access-option-icon">{option.icon}</span>
                  <span className="dev-access-option-copy">
                    <span>{option.label}</span>
                    <small>{option.description}</small>
                  </span>
                  {developerAccessLevel === option.id && <span className="dev-access-check">OK</span>}
                </button>
              ))}
            </div>
          )}
        </div>
      )}
      <textarea
        className="composer-textarea"
        onChange={(e) => setPrompt(e.currentTarget.value)}
        onKeyDown={handleKeyDown}
        placeholder={isRtl ? "اسأل Waey عن شاشتك..." : "Ask Waey about your screen..."}
        value={prompt}
        rows={2}
        ref={promptInputRef}
        dir={isRtl ? "rtl" : "ltr"}
      />
      {isStreaming ? (
        <button className="composer-send composer-stop" onClick={() => void onCancelPrompt()} title={isRtl ? "إيقاف الرد" : "Stop response"} type="button">
          <svg width="11" height="11" viewBox="0 0 24 24" fill="currentColor">
            <rect x="6" y="6" width="12" height="12" rx="2" />
          </svg>
        </button>
      ) : (
        <button className="composer-send" disabled={!prompt.trim()} type="submit">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.4" strokeLinecap="round" strokeLinejoin="round">
            <path d="M12 19V5" /><path d="M5 12l7-7 7 7" />
          </svg>
        </button>
      )}
    </form>
  );
}

function compactWorkspaceName(path: string) {
  const normalizedPath = path.replace(/\\/g, "/").replace(/\/$/, "");
  const parts = normalizedPath.split("/").filter(Boolean);
  const name = parts[parts.length - 1] ?? normalizedPath;

  if (name.length <= 26) {
    return name;
  }

  return `${name.slice(0, 12)}...${name.slice(-9)}`;
}
