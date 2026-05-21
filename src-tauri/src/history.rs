use crate::storage::open_app_database;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use tauri::AppHandle;
use uuid::Uuid;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Conversation {
    pub id: String,
    pub title: String,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatMessage {
    pub id: String,
    pub conversation_id: String,
    pub role: ChatMessageRole,
    pub content: String,
    pub capture_path: Option<String>,
    pub created_at: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ChatMessageRole {
    User,
    Assistant,
    System,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConversationDraft {
    pub title: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatMessageDraft {
    pub id: Option<String>,
    pub conversation_id: String,
    pub role: ChatMessageRole,
    pub content: String,
    pub capture_path: Option<String>,
}

pub fn list_conversations(app: &AppHandle) -> Result<Vec<Conversation>, String> {
    let connection = open_app_database(app)?;
    let mut statement = connection
        .prepare(
            "select id, title, created_at, updated_at
             from conversations
             order by updated_at desc",
        )
        .map_err(|error| error.to_string())?;

    let rows = statement
        .query_map([], |row| {
            Ok(Conversation {
                id: row.get(0)?,
                title: row.get(1)?,
                created_at: row.get(2)?,
                updated_at: row.get(3)?,
            })
        })
        .map_err(|error| error.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;
    Ok(rows)
}

pub fn create_conversation(
    app: &AppHandle,
    draft: ConversationDraft,
) -> Result<Conversation, String> {
    let title = conversation_title(draft.title);
    let conversation = Conversation {
        id: Uuid::new_v4().to_string(),
        title,
        created_at: timestamp_seconds(),
        updated_at: timestamp_seconds(),
    };
    let connection = open_app_database(app)?;

    connection
        .execute(
            "insert into conversations (id, title, created_at, updated_at)
             values (?1, ?2, ?3, ?4)",
            params![
                &conversation.id,
                &conversation.title,
                conversation.created_at,
                conversation.updated_at
            ],
        )
        .map_err(|error| error.to_string())?;

    Ok(conversation)
}

pub fn list_messages(app: &AppHandle, conversation_id: String) -> Result<Vec<ChatMessage>, String> {
    let connection = open_app_database(app)?;
    let mut statement = connection
        .prepare(
            "select id, conversation_id, role, content, capture_path, created_at
             from chat_messages
             where conversation_id = ?1
             order by created_at asc",
        )
        .map_err(|error| error.to_string())?;

    let rows = statement
        .query_map(params![conversation_id], |row| {
            Ok(ChatMessage {
                id: row.get(0)?,
                conversation_id: row.get(1)?,
                role: message_role_from_string(row.get::<_, String>(2)?),
                content: row.get(3)?,
                capture_path: row.get(4)?,
                created_at: row.get(5)?,
            })
        })
        .map_err(|error| error.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;
    Ok(rows)
}

pub fn save_message(app: &AppHandle, draft: ChatMessageDraft) -> Result<ChatMessage, String> {
    if draft.content.trim().is_empty() {
        return Err("Message content is required.".to_string());
    }

    let message = ChatMessage {
        id: draft.id.unwrap_or_else(|| Uuid::new_v4().to_string()),
        conversation_id: draft.conversation_id,
        role: draft.role,
        content: draft.content.trim().to_string(),
        capture_path: draft.capture_path,
        created_at: timestamp_seconds(),
    };
    let connection = open_app_database(app)?;

    connection
        .execute(
            "insert into chat_messages (id, conversation_id, role, content, capture_path, created_at)
             values (?1, ?2, ?3, ?4, ?5, ?6)
             on conflict(id) do update set
               content = excluded.content,
               capture_path = excluded.capture_path",
            params![
                &message.id,
                &message.conversation_id,
                message_role_to_string(&message.role),
                &message.content,
                &message.capture_path,
                message.created_at
            ],
        )
        .map_err(|error| error.to_string())?;

    connection
        .execute(
            "update conversations
             set updated_at = strftime('%s','now')
             where id = ?1",
            params![&message.conversation_id],
        )
        .map_err(|error| error.to_string())?;

    Ok(message)
}

pub fn delete_conversation(app: &AppHandle, conversation_id: String) -> Result<(), String> {
    let connection = open_app_database(app)?;

    connection
        .execute(
            "delete from conversations where id = ?1",
            params![conversation_id],
        )
        .map_err(|error| error.to_string())?;

    Ok(())
}

fn conversation_title(title: String) -> String {
    let trimmed_title = title.trim();

    if trimmed_title.is_empty() {
        return "New conversation".to_string();
    }

    trimmed_title.chars().take(64).collect()
}

fn timestamp_seconds() -> i64 {
    chrono_like_timestamp()
}

fn chrono_like_timestamp() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or_default()
}

fn message_role_to_string(role: &ChatMessageRole) -> &'static str {
    match role {
        ChatMessageRole::User => "user",
        ChatMessageRole::Assistant => "assistant",
        ChatMessageRole::System => "system",
    }
}

fn message_role_from_string(value: String) -> ChatMessageRole {
    match value.as_str() {
        "user" => ChatMessageRole::User,
        "assistant" => ChatMessageRole::Assistant,
        _ => ChatMessageRole::System,
    }
}
