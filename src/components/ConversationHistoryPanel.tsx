import type { Conversation } from "../shared/types";

interface ConversationHistoryPanelProps {
  activeConversationId: string | null;
  conversations: Conversation[];
  onDeleteConversation: (conversationId: string) => Promise<void>;
  onOpenConversation: (conversationId: string) => void;
  onStartNewConversation: () => void;
}

export function ConversationHistoryPanel({
  activeConversationId,
  conversations,
  onDeleteConversation,
  onOpenConversation,
  onStartNewConversation,
}: ConversationHistoryPanelProps) {
  return (
    <section className="min-h-0 flex-1 overflow-y-auto rounded-3xl border border-white/10 bg-black/30 p-4">
      <div className="flex items-start justify-between gap-4">
        <div>
          <p className="text-sm font-semibold text-waey-coral">Conversation History</p>
          <h2 className="mt-1 text-xl font-semibold">Saved locally</h2>
        </div>
        <button
          className="rounded-full bg-waey-bright px-4 py-2 text-sm font-semibold hover:bg-waey-red"
          onClick={onStartNewConversation}
          type="button"
        >
          New Chat
        </button>
      </div>

      <div className="mt-4 grid gap-2">
        {conversations.length === 0 ? (
          <p className="rounded-2xl border border-white/10 bg-white/[0.04] px-4 py-3 text-sm text-white/50">
            Your conversations will appear here after the first prompt.
          </p>
        ) : (
          conversations.map((conversation) => (
            <div
              className="flex items-center justify-between gap-3 rounded-2xl border border-white/10 bg-white/[0.04] px-4 py-3"
              key={conversation.id}
            >
              <button
                className="min-w-0 text-left"
                onClick={() => onOpenConversation(conversation.id)}
                type="button"
              >
                <p className="truncate text-sm font-semibold">{conversation.title}</p>
                <p className="truncate text-xs text-white/45">
                  Updated {formatConversationDate(conversation.updatedAt)}
                </p>
              </button>
              <div className="flex items-center gap-2">
                {activeConversationId === conversation.id ? (
                  <span className="rounded-full bg-waey-bright/20 px-3 py-1 text-xs text-waey-coral">
                    Open
                  </span>
                ) : null}
                <button
                  className="rounded-full border border-white/10 px-3 py-1 text-xs text-white/55 hover:border-waey-coral hover:text-white"
                  onClick={() => void onDeleteConversation(conversation.id)}
                  type="button"
                >
                  Delete
                </button>
              </div>
            </div>
          ))
        )}
      </div>
    </section>
  );
}

function formatConversationDate(timestampSeconds: number) {
  return new Intl.DateTimeFormat(undefined, {
    dateStyle: "medium",
    timeStyle: "short",
  }).format(timestampSeconds * 1000);
}
