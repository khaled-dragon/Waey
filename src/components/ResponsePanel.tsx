import { useEffect, useRef, useState } from "react";
import { capturePreviewUrl } from "../features/capture";
import type { ChatMessage, ScreenCapture, StreamState } from "../shared/types";

interface ResponsePanelProps {
  capture: ScreenCapture | null;
  captures: ScreenCapture[];
  errorMessage: string | null;
  messages: ChatMessage[];
  onEditLastUserMessage: (messageId: string, prompt: string) => Promise<void>;
  onRemoveCapture: (path: string) => void;
  streamState: StreamState;
  isRtl: boolean;
}

export function ResponsePanel({ capture, captures, errorMessage, messages, onEditLastUserMessage, onRemoveCapture, streamState, isRtl }: ResponsePanelProps) {
  const previewUrl = capture ? capturePreviewUrl(capture) : null;
  const previewItems = captures.map((captureItem) => ({
    capture: captureItem,
    url: capturePreviewUrl(captureItem),
  }));
  const scrollContainerRef = useRef<HTMLDivElement>(null);
  const bottomRef = useRef<HTMLDivElement>(null);
  const shouldStickToBottomRef = useRef(true);
  const [editingMessageId, setEditingMessageId] = useState<string | null>(null);
  const [draftContent, setDraftContent] = useState("");
  const lastAssistantMessage = messages.slice().reverse().find((m: ChatMessage) => m.role === "assistant") ?? null;
  const lastUserMessageId = messages.slice().reverse().find((m: ChatMessage) => m.role === "user")?.id ?? null;
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

  function startEditing(message: ChatMessage) {
    setEditingMessageId(message.id);
    setDraftContent(message.content);
  }

  async function submitEdit(messageId: string) {
    if (!draftContent.trim()) {
      return;
    }

    await onEditLastUserMessage(messageId, draftContent);
    setEditingMessageId(null);
    setDraftContent("");
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
            {captures.length > 0
              ? `${captures.length}/3 ${isRtl ? "صور مرفقة" : "screenshots attached"}`
              : isRtl ? "اضغط Alt+Space لفتح Waey مع لقطة شاشة" : "Press Alt+Space to open Waey with a screenshot"}
          </div>
          {previewItems.length > 0 && <CapturePreviewStrip isRtl={isRtl} items={previewItems} onRemoveCapture={onRemoveCapture} />}
        </div>
      ) : (
        <>
          {capture && previewUrl && (
            <div className="capture-info">
              <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                <path d="M23 19a2 2 0 01-2 2H3a2 2 0 01-2-2V8a2 2 0 012-2h4l2-3h6l2 3h4a2 2 0 012 2z"/><circle cx="12" cy="13" r="4"/>
              </svg>
              {captures.length > 0 ? `${captures.length}/3` : `${capture.width} x ${capture.height}px`}
            </div>
          )}
          {previewItems.length > 0 && <CapturePreviewStrip isRtl={isRtl} items={previewItems} onRemoveCapture={onRemoveCapture} />}
          {messages.map((message) => (
            <div key={message.id} className={`message message--${message.role === "user" ? "user" : "assistant"}`}>
              <div className="message-role">
                <span>{message.role === "user" ? (isRtl ? "أنت" : "You") : "Waey"}</span>
                {message.role === "user" && message.id === lastUserMessageId && streamState !== "streaming" && editingMessageId !== message.id && (
                  <button className="message-action" onClick={() => startEditing(message)} type="button">
                    {isRtl ? "تعديل" : "Edit"}
                  </button>
                )}
              </div>
              {streamState === "streaming" && message.role === "assistant" && !message.content.trim() ? (
                <div className="thinking-indicator" role="status" aria-live="polite">
                  <div className="thinking-track">
                    <div className="thinking-gradient" />
                  </div>
                  <span>{isRtl ? "Waey يفكر..." : "Waey is thinking..."}</span>
                </div>
              ) : editingMessageId === message.id ? (
                <form className="message-edit-form" onSubmit={(event) => { event.preventDefault(); void submitEdit(message.id); }}>
                  <textarea
                    autoFocus
                    className="message-edit-input"
                    onChange={(event) => setDraftContent(event.currentTarget.value)}
                    onKeyDown={(event) => {
                      if (event.key === "Escape") {
                        setEditingMessageId(null);
                        setDraftContent("");
                      }
                    }}
                    value={draftContent}
                    rows={3}
                  />
                  <div className="message-edit-actions">
                    <button className="btn-secondary" onClick={() => { setEditingMessageId(null); setDraftContent(""); }} type="button">
                      {isRtl ? "إلغاء" : "Cancel"}
                    </button>
                    <button className="btn-primary" type="submit">
                      {isRtl ? "إرسال" : "Send"}
                    </button>
                  </div>
                </form>
              ) : (
                <div className="message-bubble">
                  {message.role === "assistant" ? <FormattedAssistantMessage content={message.content} isRtl={isRtl} /> : message.content}
                </div>
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

type MessageSegment =
  | { type: "text"; content: string }
  | { type: "code"; content: string; language: string };

interface FormattedAssistantMessageProps {
  content: string;
  isRtl: boolean;
}

function FormattedAssistantMessage({ content, isRtl }: FormattedAssistantMessageProps) {
  const segments = parseMessageSegments(content);

  return (
    <>
      {segments.map((segment, index) =>
        segment.type === "code" ? (
          <CodeBlock content={segment.content} isRtl={isRtl} key={`${segment.type}-${index}`} language={segment.language} />
        ) : (
          <span key={`${segment.type}-${index}`}>{segment.content}</span>
        ),
      )}
    </>
  );
}

interface CodeBlockProps {
  content: string;
  isRtl: boolean;
  language: string;
}

function CodeBlock({ content, isRtl, language }: CodeBlockProps) {
  const [copied, setCopied] = useState(false);
  const label = language || "code";

  async function copyCode() {
    try {
      await navigator.clipboard.writeText(content);
      setCopied(true);
      window.setTimeout(() => setCopied(false), 1200);
    } catch {
      setCopied(false);
    }
  }

  return (
    <div className="code-block">
      <div className="code-block-header">
        <span>{label}</span>
        <button className="code-copy-btn" onClick={() => void copyCode()} type="button">
          {copied ? (isRtl ? "تم النسخ" : "Copied") : (isRtl ? "نسخ" : "Copy")}
        </button>
      </div>
      <pre className="code-block-body" dir="ltr"><code>{content}</code></pre>
    </div>
  );
}

function parseMessageSegments(content: string): MessageSegment[] {
  const segments: MessageSegment[] = [];
  const codeBlockPattern = /```([a-zA-Z0-9_-]*)\n?([\s\S]*?)```/g;
  let cursor = 0;
  let match: RegExpExecArray | null;

  while ((match = codeBlockPattern.exec(content)) !== null) {
    if (match.index > cursor) {
      segments.push({ type: "text", content: content.slice(cursor, match.index) });
    }

    segments.push({
      type: "code",
      language: match[1]?.trim() ?? "",
      content: match[2]?.replace(/\n$/, "") ?? "",
    });
    cursor = match.index + match[0].length;
  }

  if (cursor < content.length) {
    segments.push({ type: "text", content: content.slice(cursor) });
  }

  return segments.length > 0 ? segments : [{ type: "text", content }];
}

interface CapturePreviewItem {
  capture: ScreenCapture;
  url: string;
}

interface CapturePreviewStripProps {
  isRtl: boolean;
  items: CapturePreviewItem[];
  onRemoveCapture: (path: string) => void;
}

function CapturePreviewStrip({ isRtl, items, onRemoveCapture }: CapturePreviewStripProps) {
  return (
    <div className="capture-preview-strip">
      {items.map(({ capture, url }, index) => (
        <div className="capture-preview-thumb" key={capture.path}>
          <img alt={`Screen capture ${index + 1}`} src={url} />
          <span className="capture-preview-index">{index + 1}</span>
          <button
            aria-label={isRtl ? "إزالة الصورة" : "Remove screenshot"}
            className="capture-preview-remove"
            onClick={() => onRemoveCapture(capture.path)}
            type="button"
          >
            ×
          </button>
        </div>
      ))}
    </div>
  );
}
