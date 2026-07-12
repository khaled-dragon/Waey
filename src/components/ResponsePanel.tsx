import { useEffect, useRef, useState } from "react";
import { capturePreviewUrl } from "../features/capture";
import type { ChatMessage, DeveloperAccessLevel, DeveloperContextStatus, DeveloperEditStatus, ScreenCapture, StreamState } from "../shared/types";

interface ResponsePanelProps {
  capture: ScreenCapture | null;
  captures: ScreenCapture[];
  errorMessage: string | null;
  messages: ChatMessage[];
  developerContextStatus: DeveloperContextStatus | null;
  developerEditStatus: DeveloperEditStatus | null;
  onEditLastUserMessage: (messageId: string, prompt: string) => Promise<void>;
  onApplyDeveloperEdit: (path: string, content: string) => Promise<void>;
  onRemoveCapture: (path: string) => void;
  streamState: StreamState;
  isRtl: boolean;
  workedForMs: number | null;
  developerModeEnabled: boolean;
  developerAccessLevel: DeveloperAccessLevel;
}

export function ResponsePanel({ capture, captures, errorMessage, messages, developerContextStatus, developerEditStatus, onEditLastUserMessage, onApplyDeveloperEdit, onRemoveCapture, streamState, isRtl, workedForMs, developerModeEnabled, developerAccessLevel }: ResponsePanelProps) {
  const previewUrl = capture ? capturePreviewUrl(capture) : null;
  const previewItems = captures.map((captureItem) => ({
    capture: captureItem,
    url: capturePreviewUrl(captureItem),
  }));
  const scrollContainerRef = useRef<HTMLDivElement>(null);
  const bottomRef = useRef<HTMLDivElement>(null);
  const shouldStickToBottomRef = useRef(true);
  const appliedDeveloperEditsRef = useRef<Set<string>>(new Set());
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

  useEffect(() => {
    if (!developerModeEnabled || developerAccessLevel !== "auto" || streamState === "streaming") {
      return;
    }

    const lastAssistant = messages.slice().reverse().find((message) => message.role === "assistant");

    if (!lastAssistant?.content.trim()) {
      return;
    }

    const developerEdits = parseDeveloperEditBlocks(lastAssistant.content);

    for (const edit of developerEdits) {
      const editKey = `${lastAssistant.id}:${edit.path}:${hashDeveloperEdit(edit.content)}`;

      if (appliedDeveloperEditsRef.current.has(editKey)) {
        continue;
      }

      appliedDeveloperEditsRef.current.add(editKey);
      void onApplyDeveloperEdit(edit.path, edit.content);
    }
  }, [developerAccessLevel, developerModeEnabled, messages, onApplyDeveloperEdit, streamState]);

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

    const nextContent = draftContent;
    setEditingMessageId(null);
    setDraftContent("");
    await onEditLastUserMessage(messageId, nextContent);
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
          {(developerContextStatus || developerEditStatus) && (
            <div className="developer-status-stack">
              {developerContextStatus && (
                <DeveloperStatusChip status={developerContextStatus} />
              )}
              {developerEditStatus && (
                <DeveloperEditStatusChip status={developerEditStatus} />
              )}
            </div>
          )}
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
              {streamState === "streaming" && message.role === "assistant" && !message.content.trim() && !message.reasoningContent?.trim() ? (
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
                  {message.role === "assistant" ? <FormattedAssistantMessage content={message.content} isRtl={isRtl} onApplyDeveloperEdit={onApplyDeveloperEdit} reasoningContent={message.reasoningContent} /> : message.content}
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
          {workedForMs !== null && (
            <div className="worked-timer">
              {isRtl ? "عمل لمدة" : "Worked for"} {formatDuration(workedForMs)}
            </div>
          )}
          <div ref={bottomRef} />
        </>
        )}
      </div>
    </div>
  );
}

function formatDuration(durationMs: number) {
  const totalSeconds = Math.max(0, Math.floor(durationMs / 1000));
  const minutes = Math.floor(totalSeconds / 60);
  const seconds = totalSeconds % 60;

  if (minutes === 0) {
    return `${seconds}s`;
  }

  return `${minutes}m ${seconds}s`;
}

type MessageSegment =
  | { type: "text"; content: string }
  | { type: "code"; content: string; language: string };

interface DeveloperStatusChipProps {
  status: DeveloperContextStatus;
}

function DeveloperStatusChip({ status }: DeveloperStatusChipProps) {
  const hasDetails = Boolean(status.filePath || status.activeWindowTitle || status.lineRange || status.warnings.length > 0);

  return (
    <details className={`developer-status-chip developer-status-chip--${status.kind}`} open={false}>
      <summary>
        <span>{status.label}</span>
        <small>{status.detail}</small>
      </summary>
      {hasDetails && (
        <div className="developer-status-details">
          {status.filePath && <div><strong>File</strong><span>{status.filePath}</span></div>}
          {status.lineRange && <div><strong>Lines</strong><span>{status.lineRange.start}-{status.lineRange.end} of {status.lineRange.total}</span></div>}
          {status.activeWindowTitle && <div><strong>Window</strong><span>{status.activeWindowTitle}</span></div>}
          {status.warnings.map((warning) => (
            <div key={warning}><strong>Note</strong><span>{warning}</span></div>
          ))}
        </div>
      )}
    </details>
  );
}

interface DeveloperEditStatusChipProps {
  status: DeveloperEditStatus;
}

function DeveloperEditStatusChip({ status }: DeveloperEditStatusChipProps) {
  return (
    <div className={`developer-status-chip developer-status-chip--${status.kind}`}>
      <span>{status.label}</span>
      <small>{status.detail}</small>
    </div>
  );
}

function hashDeveloperEdit(content: string) {
  let hash = 2166136261;

  for (let index = 0; index < content.length; index += 1) {
    hash ^= content.charCodeAt(index);
    hash = Math.imul(hash, 16777619);
  }

  return (hash >>> 0).toString(16);
}

interface FormattedAssistantMessageProps {
  content: string;
  isRtl: boolean;
  onApplyDeveloperEdit: (path: string, content: string) => Promise<void>;
  reasoningContent?: string;
}

function FormattedAssistantMessage({ content, isRtl, onApplyDeveloperEdit, reasoningContent }: FormattedAssistantMessageProps) {
  const { answer, thinking } = parseAssistantThinking(content);
  const visibleThinking = reasoningContent?.trim() || thinking;
  const segments = parseMessageSegments(answer);

  return (
    <>
      {visibleThinking.length > 0 && <ThinkingDisclosure content={visibleThinking} isRtl={isRtl} />}
      {segments.map((segment, index) =>
        segment.type === "code" ? (
          <CodeBlock content={segment.content} isRtl={isRtl} key={`${segment.type}-${index}`} language={segment.language} onApplyDeveloperEdit={onApplyDeveloperEdit} />
        ) : (
          <span key={`${segment.type}-${index}`}>{segment.content}</span>
        ),
      )}
    </>
  );
}

interface ThinkingDisclosureProps {
  content: string;
  isRtl: boolean;
}

function ThinkingDisclosure({ content, isRtl }: ThinkingDisclosureProps) {
  const summary = isRtl ? "طريقة التفكير" : "Thinking";

  return (
    <details className="thinking-disclosure">
      <summary>{summary}</summary>
      <div className="thinking-disclosure-body">{content}</div>
    </details>
  );
}

interface CodeBlockProps {
  content: string;
  isRtl: boolean;
  language: string;
  onApplyDeveloperEdit: (path: string, content: string) => Promise<void>;
}

function CodeBlock({ content, isRtl, language, onApplyDeveloperEdit }: CodeBlockProps) {
  const [copied, setCopied] = useState(false);
  const [applying, setApplying] = useState(false);
  const label = language || "code";
  const developerEdit = parseDeveloperEditBlock(language, content);

  async function copyCode() {
    try {
      await navigator.clipboard.writeText(content);
      setCopied(true);
      window.setTimeout(() => setCopied(false), 1200);
    } catch {
      setCopied(false);
    }
  }

  async function applyDeveloperEdit() {
    if (!developerEdit) {
      return;
    }

    setApplying(true);

    try {
      await onApplyDeveloperEdit(developerEdit.path, developerEdit.content);
    } finally {
      setApplying(false);
    }
  }

  return (
    <div className="code-block">
      <div className="code-block-header">
        <span>{label}</span>
        <div className="code-block-actions">
          {developerEdit && (
            <button className="code-copy-btn" disabled={applying} onClick={() => void applyDeveloperEdit()} type="button">
              {applying ? "..." : "Apply"}
            </button>
          )}
          <button className="code-copy-btn" onClick={() => void copyCode()} type="button">
            {copied ? (isRtl ? "تم النسخ" : "Copied") : (isRtl ? "نسخ" : "Copy")}
          </button>
        </div>
      </div>
      <pre className="code-block-body" dir="ltr"><code>{content}</code></pre>
    </div>
  );
}

function parseDeveloperEditBlock(language: string, content: string) {
  if (language !== "waey-edit") {
    return null;
  }

  const lines = content.split(/\r?\n/);
  const pathLine = lines[0]?.trim() ?? "";

  if (!pathLine.toLowerCase().startsWith("path:")) {
    return null;
  }

  const path = pathLine.slice("path:".length).trim();
  const replacement = lines.slice(1).join("\n");

  if (!path || !replacement.trim()) {
    return null;
  }

  return { path, content: replacement };
}

function parseDeveloperEditBlocks(content: string) {
  const edits: Array<{ path: string; content: string }> = [];
  const codeBlockPattern = /```waey-edit\n?([\s\S]*?)```/g;
  let match: RegExpExecArray | null;

  while ((match = codeBlockPattern.exec(content)) !== null) {
    const edit = parseDeveloperEditBlock("waey-edit", match[1] ?? "");

    if (edit) {
      edits.push(edit);
    }
  }

  return edits;
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

function parseAssistantThinking(content: string) {
  const thinkingParts: string[] = [];
  let answer = "";
  let cursor = 0;
  const completeThinkPattern = /<think>([\s\S]*?)<\/think>/gi;
  let match: RegExpExecArray | null;

  while ((match = completeThinkPattern.exec(content)) !== null) {
    answer += content.slice(cursor, match.index);
    thinkingParts.push(match[1]?.trim() ?? "");
    cursor = match.index + match[0].length;
  }

  answer += content.slice(cursor);

  const openThinkIndex = answer.toLowerCase().indexOf("<think>");
  if (openThinkIndex >= 0) {
    const visibleAnswer = answer.slice(0, openThinkIndex);
    const streamingThinking = answer.slice(openThinkIndex + "<think>".length);

    answer = visibleAnswer;
    thinkingParts.push(streamingThinking.trim());
  }

  const cleanAnswer = answer.replace(/<\/think>/gi, "").trimStart();
  const thinking = thinkingParts.filter(Boolean).join("\n\n");

  return {
    answer: cleanAnswer,
    thinking,
  };
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
