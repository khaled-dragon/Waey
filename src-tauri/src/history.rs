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
    pub capture_paths: Vec<String>,
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
pub struct ConversationRenameDraft {
    pub conversation_id: String,
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
    pub capture_paths: Option<Vec<String>>,
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

pub fn rename_conversation(
    app: &AppHandle,
    draft: ConversationRenameDraft,
) -> Result<Conversation, String> {
    let title = conversation_title(draft.title);
    let updated_at = timestamp_seconds();
    let connection = open_app_database(app)?;

    let changed_rows = connection
        .execute(
            "update conversations
             set title = ?1, updated_at = ?2
             where id = ?3",
            params![&title, updated_at, &draft.conversation_id],
        )
        .map_err(|error| error.to_string())?;

    if changed_rows == 0 {
        return Err("Conversation was not found.".to_string());
    }

    let mut statement = connection
        .prepare(
            "select id, title, created_at, updated_at
             from conversations
             where id = ?1",
        )
        .map_err(|error| error.to_string())?;

    statement
        .query_row(params![draft.conversation_id], |row| {
            Ok(Conversation {
                id: row.get(0)?,
                title: row.get(1)?,
                created_at: row.get(2)?,
                updated_at: row.get(3)?,
            })
        })
        .map_err(|error| error.to_string())
}

pub fn list_messages(app: &AppHandle, conversation_id: String) -> Result<Vec<ChatMessage>, String> {
    let connection = open_app_database(app)?;
    let mut statement = connection
        .prepare(
            "select id, conversation_id, role, content, capture_path, capture_paths, created_at
             from chat_messages
             where conversation_id = ?1
             order by created_at asc",
        )
        .map_err(|error| error.to_string())?;

    let rows = statement
        .query_map(params![conversation_id], |row| {
            let capture_path: Option<String> = row.get(4)?;
            let capture_paths = parse_capture_paths(row.get(5)?, capture_path.as_deref());

            Ok(ChatMessage {
                id: row.get(0)?,
                conversation_id: row.get(1)?,
                role: message_role_from_string(row.get::<_, String>(2)?),
                content: row.get(3)?,
                capture_path,
                capture_paths,
                created_at: row.get(6)?,
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

    let capture_paths = normalized_capture_paths(draft.capture_paths, draft.capture_path.as_deref());
    let capture_path = capture_paths.first().cloned().or(draft.capture_path);
    let capture_paths_json = serde_json::to_string(&capture_paths).map_err(|error| error.to_string())?;
    let message = ChatMessage {
        id: draft.id.unwrap_or_else(|| Uuid::new_v4().to_string()),
        conversation_id: draft.conversation_id,
        role: draft.role,
        content: draft.content.trim().to_string(),
        capture_path,
        capture_paths,
        created_at: timestamp_seconds(),
    };
    let connection = open_app_database(app)?;

    connection
        .execute(
            "insert into chat_messages (id, conversation_id, role, content, capture_path, capture_paths, created_at)
             values (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             on conflict(id) do update set
               content = excluded.content,
               capture_path = excluded.capture_path,
               capture_paths = excluded.capture_paths",
            params![
                &message.id,
                &message.conversation_id,
                message_role_to_string(&message.role),
                &message.content,
                &message.capture_path,
                &capture_paths_json,
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

pub fn delete_message(app: &AppHandle, message_id: String) -> Result<(), String> {
    let connection = open_app_database(app)?;

    connection
        .execute("delete from chat_messages where id = ?1", params![message_id])
        .map_err(|error| error.to_string())?;

    Ok(())
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

fn normalized_capture_paths(
    capture_paths: Option<Vec<String>>,
    fallback_capture_path: Option<&str>,
) -> Vec<String> {
    let mut paths = capture_paths
        .unwrap_or_default()
        .into_iter()
        .map(|path| path.trim().to_string())
        .filter(|path| !path.is_empty())
        .collect::<Vec<_>>();

    if paths.is_empty() {
        if let Some(path) = fallback_capture_path {
            if !path.trim().is_empty() {
                paths.push(path.trim().to_string());
            }
        }
    }

    paths.truncate(3);
    paths
}

fn parse_capture_paths(value: Option<String>, fallback_capture_path: Option<&str>) -> Vec<String> {
    let parsed_paths = value
        .and_then(|json| serde_json::from_str::<Vec<String>>(&json).ok())
        .unwrap_or_default();

    normalized_capture_paths(Some(parsed_paths), fallback_capture_path)
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
