import { useState, type FormEvent } from "react";
import type { DeveloperAccessLevel, StreamState } from "../shared/types";

interface ChatComposerProps {
  streamState: StreamState;
  onCancelPrompt: () => Promise<void>;
  onSubmitPrompt: (prompt: string) => Promise<void>;
  isRtl: boolean;
  developerModeEnabled: boolean;
  developerAccessLevel: DeveloperAccessLevel;
  onChangeDeveloperAccessLevel: (accessLevel: DeveloperAccessLevel) => void;
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
  onChangeDeveloperAccessLevel,
}: ChatComposerProps) {
  const [prompt, setPrompt] = useState("");
  const [accessMenuOpen, setAccessMenuOpen] = useState(false);
  const isStreaming = streamState === "streaming";
  const activeAccess = accessOptions.find((option) => option.id === developerAccessLevel) ?? accessOptions[1];

  async function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (isStreaming || !prompt.trim()) return;
    await onSubmitPrompt(prompt);
    setPrompt("");
  }

  function handleKeyDown(event: React.KeyboardEvent<HTMLTextAreaElement>) {
    if (event.key === "Enter" && !event.shiftKey) {
      event.preventDefault();
      if (!isStreaming && prompt.trim()) {
        void onSubmitPrompt(prompt).then(() => setPrompt(""));
      }
    }
  }

  return (
    <form className="chat-composer" onSubmit={handleSubmit}>
      {developerModeEnabled && (
        <div className="dev-access-menu-wrap">
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
