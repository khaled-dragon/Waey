import { capturePreviewUrl } from "../features/capture";
import type { ChatMessage, ScreenCapture, StreamState } from "../shared/types";

interface ResponsePanelProps {
  capture: ScreenCapture | null;
  errorMessage: string | null;
  messages: ChatMessage[];
  streamState: StreamState;
}

export function ResponsePanel({ capture, errorMessage, messages, streamState }: ResponsePanelProps) {
  const previewUrl = capture ? capturePreviewUrl(capture) : null;

  return (
    <div className="response-panel">
      {errorMessage && <div className="error-msg">{errorMessage}</div>}

      {messages.length === 0 ? (
        <div className="response-empty">
          <div className="response-empty-icon">🐙</div>
          <div>
            {capture
              ? `${capture.width} × ${capture.height}px captured — ready for your question`
              : "Press Alt+Shift+Space to open with a screenshot, or use Smart Crop"}
          </div>
          {previewUrl && (
            <div className="capture-preview" style={{ width: "100%" }}>
              <img alt="Screen capture" src={previewUrl} />
            </div>
          )}
        </div>
      ) : (
        <>
          {capture && previewUrl && (
            <div className="capture-info">
              📸 {capture.width} × {capture.height}px attached
            </div>
          )}
          {messages.map((message) => (
            <div key={message.id} className={`message message--${message.role === "user" ? "user" : "assistant"}`}>
              <div className="message-role">{message.role === "user" ? "You" : "Waey"}</div>
              <div className="message-bubble">{message.content || "Thinking..."}</div>
            </div>
          ))}
          {streamState === "streaming" && (
            <div className="message message--assistant">
              <div className="message-role">Waey</div>
              <div className="message-bubble" style={{ color: "rgba(255,255,255,0.4)" }}>●●●</div>
            </div>
          )}
        </>
      )}
    </div>
  );
}
