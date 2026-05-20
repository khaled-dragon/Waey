import { useState, type FormEvent } from "react";
import type { StreamState } from "../shared/types";

interface ChatComposerProps {
  streamState: StreamState;
  onSubmitPrompt: (prompt: string) => Promise<void>;
}

export function ChatComposer({ streamState, onSubmitPrompt }: ChatComposerProps) {
  const [prompt, setPrompt] = useState("");
  const isStreaming = streamState === "streaming";

  async function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();

    if (isStreaming) {
      return;
    }

    await onSubmitPrompt(prompt);
    setPrompt("");
  }

  return (
    <form className="rounded-3xl border border-white/10 bg-black/25 p-3" onSubmit={handleSubmit}>
      <textarea
        className="min-h-28 w-full resize-none bg-transparent px-2 py-2 text-base text-white outline-none placeholder:text-white/35"
        onChange={(event) => setPrompt(event.currentTarget.value)}
        placeholder="Ask Waey about anything on your screen..."
        value={prompt}
      />
      <div className="flex items-center justify-between gap-3 pt-2">
        <p className="text-xs text-white/45">Alt+Space opens overlay. Esc closes it.</p>
        <button
          className="rounded-full bg-waey-bright px-5 py-2 text-sm font-semibold text-white shadow-lg shadow-waey-bright/20 transition hover:bg-waey-red disabled:cursor-not-allowed disabled:opacity-50"
          disabled={isStreaming}
          type="submit"
        >
          {isStreaming ? "Thinking..." : "Send"}
        </button>
      </div>
    </form>
  );
}
