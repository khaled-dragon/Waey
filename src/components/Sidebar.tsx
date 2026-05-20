import type { Conversation } from "../shared/types";

interface SidebarProps {
  activeConversationId: string | null;
  conversations: Conversation[];
  onOpenConversation: (conversationId: string) => void;
  onStartNewConversation: () => void;
}

export function Sidebar({
  activeConversationId,
  conversations,
  onOpenConversation,
  onStartNewConversation,
}: SidebarProps) {
  return (
    <aside className="flex min-h-0 flex-col border-b border-white/10 bg-black/25 p-5 md:border-b-0 md:border-r">
      <div className="flex items-center gap-3">
        <div className="grid size-11 place-items-center rounded-2xl bg-waey-bright font-black shadow-lg shadow-waey-bright/20">
          W
        </div>
        <div>
          <p className="text-lg font-semibold">Waey</p>
          <p className="text-xs text-white/50">Screen-aware AI</p>
        </div>
      </div>

      <button
        className="mt-6 rounded-2xl bg-waey-bright px-4 py-3 text-left text-sm font-semibold transition hover:bg-waey-red"
        onClick={onStartNewConversation}
        type="button"
      >
        New Chat
      </button>

      <div className="mt-5 min-h-0 flex-1 overflow-y-auto pr-1">
        <p className="mb-2 px-1 text-xs font-semibold uppercase tracking-[0.18em] text-white/35">
          History
        </p>
        <div className="grid gap-2">
          {conversations.length === 0 ? (
            <p className="rounded-2xl border border-white/10 bg-white/[0.04] px-4 py-3 text-sm text-white/45">
              No conversations yet.
            </p>
          ) : (
            conversations.map((conversation) => (
              <button
                className={`rounded-2xl px-4 py-3 text-left text-sm transition ${
                  activeConversationId === conversation.id
                    ? "bg-waey-bright text-white shadow-lg shadow-waey-bright/15"
                    : "text-white/62 hover:bg-white/10 hover:text-white"
                }`}
                key={conversation.id}
                onClick={() => onOpenConversation(conversation.id)}
                type="button"
              >
                <span className="block truncate font-medium">{conversation.title}</span>
                <span className="mt-1 block text-xs text-white/38">
                  {formatConversationDate(conversation.updatedAt)}
                </span>
              </button>
            ))
          )}
        </div>
      </div>
    </aside>
  );
}

function formatConversationDate(timestampSeconds: number) {
  return new Intl.DateTimeFormat(undefined, {
    dateStyle: "medium",
    timeStyle: "short",
  }).format(timestampSeconds * 1000);
}
