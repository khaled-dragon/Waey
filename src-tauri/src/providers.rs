use crate::storage::open_app_database;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use tauri::AppHandle;
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LlmProvider {
    pub id: String,
    pub name: String,
    pub kind: ProviderKind,
    pub base_url: String,
    pub api_key: String,
    pub model: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ProviderKind {
    Openrouter,
    Ollama,
    Custom,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderDraft {
    pub id: Option<String>,
    pub name: String,
    pub kind: ProviderKind,
    pub base_url: String,
    pub api_key: String,
    pub model: String,
}

pub fn list_providers(app: &AppHandle) -> Result<Vec<LlmProvider>, String> {
    let connection = open_app_database(app)?;
    let mut statement = connection
        .prepare(
            "select id, name, kind, base_url, api_key, model
             from llm_providers
             order by updated_at desc",
        )
        .map_err(|error| error.to_string())?;

    let providers = statement
        .query_map([], |row| {
            Ok(LlmProvider {
                id: row.get(0)?,
                name: row.get(1)?,
                kind: provider_kind_from_string(row.get::<_, String>(2)?),
                base_url: row.get(3)?,
                api_key: row.get(4)?,
                model: row.get(5)?,
            })
        })
        .map_err(|error| error.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;

    Ok(providers)
}

pub fn save_provider(app: &AppHandle, draft: ProviderDraft) -> Result<LlmProvider, String> {
    validate_provider_draft(&draft)?;

    let provider = LlmProvider {
        id: draft.id.unwrap_or_else(|| Uuid::new_v4().to_string()),
        name: draft.name.trim().to_string(),
        kind: draft.kind,
        base_url: draft.base_url.trim().trim_end_matches('/').to_string(),
        api_key: draft.api_key.trim().to_string(),
        model: draft.model.trim().to_string(),
    };
    let connection = open_app_database(app)?;

    connection
        .execute(
            "insert into llm_providers (id, name, kind, base_url, api_key, model, created_at, updated_at)
             values (?1, ?2, ?3, ?4, ?5, ?6, strftime('%s','now'), strftime('%s','now'))
             on conflict(id) do update set
               name = excluded.name,
               kind = excluded.kind,
               base_url = excluded.base_url,
               api_key = excluded.api_key,
               model = excluded.model,
               updated_at = strftime('%s','now')",
            params![
                &provider.id,
                &provider.name,
                provider_kind_to_string(&provider.kind),
                &provider.base_url,
                &provider.api_key,
                &provider.model
            ],
        )
        .map_err(|error| error.to_string())?;

    Ok(provider)
}

pub fn delete_provider(app: &AppHandle, provider_id: String) -> Result<(), String> {
    let connection = open_app_database(app)?;

    connection
        .execute("delete from llm_providers where id = ?1", params![provider_id])
        .map_err(|error| error.to_string())?;

    Ok(())
}

fn validate_provider_draft(draft: &ProviderDraft) -> Result<(), String> {
    if draft.name.trim().is_empty() {
        return Err("Provider name is required.".to_string());
    }

    if draft.base_url.trim().is_empty() {
        return Err("Provider base URL is required.".to_string());
    }

    if draft.model.trim().is_empty() {
        return Err("Provider model is required.".to_string());
    }

    Ok(())
}

fn provider_kind_to_string(kind: &ProviderKind) -> &'static str {
    match kind {
        ProviderKind::Openrouter => "openrouter",
        ProviderKind::Ollama => "ollama",
        ProviderKind::Custom => "custom",
    }
}

fn provider_kind_from_string(value: String) -> ProviderKind {
    match value.as_str() {
        "openrouter" => ProviderKind::Openrouter,
        "ollama" => ProviderKind::Ollama,
        _ => ProviderKind::Custom,
    }
}
