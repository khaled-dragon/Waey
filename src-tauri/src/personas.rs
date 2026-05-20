use crate::storage::open_app_database;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use tauri::AppHandle;
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Persona {
    pub id: String,
    pub name: String,
    pub prompt: String,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PersonaDraft {
    pub id: Option<String>,
    pub name: String,
    pub prompt: String,
}

pub fn list_personas(app: &AppHandle) -> Result<Vec<Persona>, String> {
    let connection = open_app_database(app)?;
    let mut statement = connection
        .prepare(
            "select id, name, prompt, created_at, updated_at
             from personas
             order by updated_at desc",
        )
        .map_err(|error| error.to_string())?;

    statement
        .query_map([], |row| {
            Ok(Persona {
                id: row.get(0)?,
                name: row.get(1)?,
                prompt: row.get(2)?,
                created_at: row.get(3)?,
                updated_at: row.get(4)?,
            })
        })
        .map_err(|error| error.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())
}

pub fn save_persona(app: &AppHandle, draft: PersonaDraft) -> Result<Persona, String> {
    validate_persona_draft(&draft)?;

    let persona = Persona {
        id: draft.id.unwrap_or_else(|| Uuid::new_v4().to_string()),
        name: draft.name.trim().to_string(),
        prompt: draft.prompt.trim().to_string(),
        created_at: timestamp_seconds(),
        updated_at: timestamp_seconds(),
    };
    let connection = open_app_database(app)?;

    connection
        .execute(
            "insert into personas (id, name, prompt, created_at, updated_at)
             values (?1, ?2, ?3, ?4, ?5)
             on conflict(id) do update set
               name = excluded.name,
               prompt = excluded.prompt,
               updated_at = strftime('%s','now')",
            params![
                &persona.id,
                &persona.name,
                &persona.prompt,
                persona.created_at,
                persona.updated_at
            ],
        )
        .map_err(|error| error.to_string())?;

    Ok(persona)
}

pub fn delete_persona(app: &AppHandle, persona_id: String) -> Result<(), String> {
    let connection = open_app_database(app)?;

    connection
        .execute("delete from personas where id = ?1", params![persona_id])
        .map_err(|error| error.to_string())?;

    Ok(())
}

fn validate_persona_draft(draft: &PersonaDraft) -> Result<(), String> {
    if draft.name.trim().is_empty() {
        return Err("Persona name is required.".to_string());
    }

    if draft.prompt.trim().is_empty() {
        return Err("Persona prompt is required.".to_string());
    }

    Ok(())
}

fn timestamp_seconds() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or_default()
}
