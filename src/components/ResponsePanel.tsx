import { useEffect, useRef } from "react";
import { capturePreviewUrl } from "../features/capture";
import type { ChatMessage, ScreenCapture, StreamState } from "../shared/types";

interface ResponsePanelProps {
  capture: ScreenCapture | null;
  errorMessage: string | null;
  messages: ChatMessage[];
  streamState: StreamState;
  isRtl: boolean;
}

export function ResponsePanel({ capture, errorMessage, messages, streamState, isRtl }: ResponsePanelProps) {
  const previewUrl = capture ? capturePreviewUrl(capture) : null;
  const scrollContainerRef = useRef<HTMLDivElement>(null);
  const bottomRef = useRef<HTMLDivElement>(null);
  const shouldStickToBottomRef = useRef(true);
  const lastAssistantMessage = messages.slice().reverse().find((m: ChatMessage) => m.role === "assistant") ?? null;
  const isStreamingWithContent = streamState === "streaming" && lastAssistantMessage !== null && lastAssistantMessage.content.trim().length > 0;
  const isStreamingWithoutContent = streamState === "streaming" && lastAssistantMessage !== null && lastAssistantMessage.content.trim().length === 0;

  useEffect(() => {
    if (shouldStickToBottomRef.current) {
      bottomRef.current?.scrollIntoView({ behavior: "smooth" });
    }
  }, [messages, streamState]);

  function handleScroll() {
    const container = scrollContainerRef.current;
    if (!container) return;

    const distanceFromBottom = container.scrollHeight - container.scrollTop - container.clientHeight;
    shouldStickToBottomRef.current = distanceFromBottom < 48;
  }

  return (
    <div className="response-panel">
      <div className="response-scroll" onScroll={handleScroll} ref={scrollContainerRef}>
        {errorMessage && <div className="error-msg">{errorMessage}</div>}

      {messages.length === 0 ? (
        <div className="response-empty">
          <svg width="28" height="28" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.4" strokeLinecap="round" strokeLinejoin="round" style={{opacity:0.3}}>
            <rect x="2" y="3" width="20" height="14" rx="2"/><line x1="8" y1="21" x2="16" y2="21"/><line x1="12" y1="17" x2="12" y2="21"/>
          </svg>
          <div>
            {capture
              ? `${capture.width} x ${capture.height}px ${isRtl ? "تم التقاطها" : "captured"}`
              : isRtl ? "اضغط Alt+Space لفتح Waey مع لقطة شاشة" : "Press Alt+Space to open Waey with a screenshot"}
          </div>
          {previewUrl && (
            <div className="capture-preview" style={{width:"100%"}}>
              <img alt="Screen capture" src={previewUrl} />
            </div>
          )}
        </div>
      ) : (
        <>
          {capture && previewUrl && (
            <div className="capture-info">
              <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                <path d="M23 19a2 2 0 01-2 2H3a2 2 0 01-2-2V8a2 2 0 012-2h4l2-3h6l2 3h4a2 2 0 012 2z"/><circle cx="12" cy="13" r="4"/>
              </svg>
              {capture.width} x {capture.height}px
            </div>
          )}
          {messages.map((message) => (
            <div key={message.id} className={`message message--${message.role === "user" ? "user" : "assistant"}`}>
              <div className="message-role">{message.role === "user" ? (isRtl ? "أنت" : "You") : "Waey"}</div>
              {streamState === "streaming" && message.role === "assistant" && !message.content.trim() ? (
                <div className="thinking-indicator" role="status" aria-live="polite">
                  <div className="thinking-track">
                    <div className="thinking-gradient" />
                  </div>
                  <span>{isRtl ? "Waey يفكر..." : "Waey is thinking..."}</span>
                </div>
              ) : (
                <div className="message-bubble">{message.content}</div>
              )}
            </div>
          ))}
          {streamState === "streaming" && !isStreamingWithContent && !isStreamingWithoutContent && (
            <div className="message message--assistant">
              <div className="message-role">Waey</div>
              <div className="thinking-indicator" role="status" aria-live="polite">
                <div className="thinking-track">
                  <div className="thinking-gradient" />
                </div>
                <span>{isRtl ? "Waey يفكر..." : "Waey is thinking..."}</span>
              </div>
            </div>
          )}
          <div ref={bottomRef} />
        </>
        )}
      </div>
    </div>
  );
}
