import type { Conversation } from "../shared/types";

interface ConversationHistoryPanelProps {
  activeConversationId: string | null;
  conversations: Conversation[];
  onDeleteConversation: (conversationId: string) => Promise<void>;
  onOpenConversation: (conversationId: string) => void;
  onStartNewConversation: () => void;
  isRtl: boolean;
}

export function ConversationHistoryPanel({ activeConversationId, conversations, onDeleteConversation, onOpenConversation, onStartNewConversation, isRtl }: ConversationHistoryPanelProps) {
  return (
    <div className="panel">
      <div className="panel-header" style={{display:"flex",alignItems:"flex-start",justifyContent:"space-between"}}>
        <div>
          <div className="panel-label">{isRtl ? "محفوظ محلياً" : "Saved locally"}</div>
          <div className="panel-title">{isRtl ? "السجل" : "History"}</div>
        </div>
        <button className="btn-primary" onClick={onStartNewConversation} type="button" style={{padding:"5px 12px",fontSize:"12px"}}>
          {isRtl ? "محادثة جديدة" : "New Chat"}
        </button>
      </div>

      <div className="item-list">
        {conversations.length === 0 ? (
          <div className="empty-list">{isRtl ? "لا توجد محادثات بعد" : "No conversations yet"}</div>
        ) : (
          conversations.map((conv) => (
            <div key={conv.id} className={`list-item ${activeConversationId === conv.id ? "list-item--active" : ""}`}>
              <button className="list-item-info" onClick={() => onOpenConversation(conv.id)} type="button">
                <div className="list-item-name">{conv.title}</div>
                <div className="list-item-sub">{formatDate(conv.updatedAt)}</div>
              </button>
              <div className="list-item-actions">
                {activeConversationId === conv.id && <span className="badge-active">{isRtl ? "مفتوح" : "Open"}</span>}
                <button className="btn-secondary" onClick={() => void onDeleteConversation(conv.id)} type="button">
                  {isRtl ? "حذف" : "Delete"}
                </button>
              </div>
            </div>
          ))
        )}
      </div>
    </div>
  );
}

function formatDate(ts: number) {
  return new Intl.DateTimeFormat(undefined, { dateStyle: "medium", timeStyle: "short" }).format(ts * 1000);
}
