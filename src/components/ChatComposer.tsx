import { useState, type FormEvent } from "react";
import type { StreamState } from "../shared/types";

interface ChatComposerProps {
  streamState: StreamState;
  onCancelPrompt: () => Promise<void>;
  onSubmitPrompt: (prompt: string) => Promise<void>;
  isRtl: boolean;
}

export function ChatComposer({ streamState, onCancelPrompt, onSubmitPrompt, isRtl }: ChatComposerProps) {
  const [prompt, setPrompt] = useState("");
  const isStreaming = streamState === "streaming";

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
            <path d="M12 19V5"/><path d="M5 12l7-7 7 7"/>
          </svg>
        </button>
      )}
    </form>
  );
}
