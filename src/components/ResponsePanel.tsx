import { capturePreviewUrl } from "../features/capture";
import type { ChatMessage, ScreenCapture, StreamState } from "../shared/types";

interface ResponsePanelProps {
  capture: ScreenCapture | null;
  errorMessage: string | null;
  messages: ChatMessage[];
  streamState: StreamState;
}

export function ResponsePanel({
  capture,
  errorMessage,
  messages,
  streamState,
}: ResponsePanelProps) {
  const previewUrl = capture ? capturePreviewUrl(capture) : null;

  return (
    <section className="flex min-h-0 flex-1 flex-col rounded-3xl border border-white/10 bg-gradient-to-b from-white/[0.07] to-black/20 p-5">
      <div className="flex items-start justify-between gap-4">
        <div>
          <p className="text-sm font-semibold text-waey-coral">
            {capture ? `${capture.source} attached` : "Waiting for screen context"}
          </p>
          <h2 className="mt-2 text-2xl font-semibold tracking-tight">
            {messages.length > 0 ? "Conversation" : "Waey is ready for your prompt."}
          </h2>
        </div>
        {streamState === "streaming" ? (
          <span className="rounded-full bg-waey-bright/20 px-3 py-1 text-xs text-waey-coral">
            Streaming
          </span>
        ) : null}
      </div>

      {errorMessage ? (
        <p className="mt-4 rounded-2xl border border-waey-coral/40 bg-waey-bright/10 px-4 py-3 text-sm text-waey-coral">
          {errorMessage}
        </p>
      ) : null}

      <div className="mt-4 flex-1 space-y-3 overflow-y-auto pr-1">
        {messages.length === 0 ? (
          <EmptyState capture={capture} previewUrl={previewUrl} />
        ) : (
          messages.map((message) => <MessageBubble key={message.id} message={message} />)
        )}
      </div>
    </section>
  );
}

function EmptyState({
  capture,
  previewUrl,
}: {
  capture: ScreenCapture | null;
  previewUrl: string | null;
}) {
  return (
    <div className="rounded-2xl border border-white/10 bg-black/20 p-4">
      <p className="text-sm leading-6 text-white/62">
        {capture
          ? `Captured ${capture.width} x ${capture.height}px from (${capture.originX}, ${capture.originY}). This image will be sent with your next prompt.`
          : "Press Alt+Space to open Waey with a full-screen capture, or use Smart Crop to select a specific region."}
      </p>
      {previewUrl ? (
        <div className="mt-4 overflow-hidden rounded-2xl border border-white/10 bg-black/25">
          <img
            alt="Latest screen capture"
            className="max-h-52 w-full object-cover opacity-90"
            src={previewUrl}
          />
        </div>
      ) : null}
    </div>
  );
}

function MessageBubble({ message }: { message: ChatMessage }) {
  const isUserMessage = message.role === "user";

  return (
    <article
      className={`rounded-2xl border px-4 py-3 ${
        isUserMessage
          ? "ml-10 border-waey-coral/25 bg-waey-bright/10"
          : "mr-10 border-white/10 bg-black/25"
      }`}
    >
      <p className="mb-2 text-xs font-semibold uppercase tracking-[0.18em] text-white/35">
        {isUserMessage ? "You" : "Waey"}
      </p>
      <p className="whitespace-pre-wrap text-sm leading-6 text-white/82">
        {message.content || "Thinking..."}
      </p>
    </article>
  );
}
