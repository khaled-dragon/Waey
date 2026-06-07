use crate::{logger, storage::open_app_database};
use rusqlite::params;
use serde::{Deserialize, Serialize};
use tauri::AppHandle;
use uuid::Uuid;

const WAEY_MANAGED_PROVIDER_ENDPOINT: &str =
    "https://khaled135-waey-preset.hf.space/waey-provider";
const WAEY_MANAGED_PROVIDER_ID: &str = "waey-managed-groq";

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LlmProvider {
    pub id: String,
    pub name: String,
    pub kind: ProviderKind,
    pub base_url: String,
    pub api_key: String,
    pub model: String,
    pub managed: bool,
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

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ManagedProviderPayload {
    id: String,
    name: String,
    kind: ProviderKind,
    base_url: String,
    api_key: String,
    model: String,
    managed: bool,
}

pub fn list_providers(app: &AppHandle) -> Result<Vec<LlmProvider>, String> {
    let connection = open_app_database(app)?;
    let mut statement = connection
        .prepare(
            "select id, name, kind, base_url, api_key, model, managed
             from llm_providers
             order by managed desc, updated_at desc",
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
                managed: row.get::<_, i64>(6)? == 1,
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
        managed: false,
    };
    let connection = open_app_database(app)?;

    connection
        .execute(
            "insert into llm_providers (id, name, kind, base_url, api_key, model, managed, created_at, updated_at)
             values (?1, ?2, ?3, ?4, ?5, ?6, 0, strftime('%s','now'), strftime('%s','now'))
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

pub async fn bootstrap_waey_provider(app: AppHandle) -> Result<Option<LlmProvider>, String> {
    if managed_provider_exists(&app)? {
        return Ok(None);
    }

    let payload = match fetch_managed_provider().await {
        Ok(payload) => payload,
        Err(error) => {
            logger::warn(format!("managed provider bootstrap skipped: {error}"));
            return Ok(None);
        }
    };

    let provider = LlmProvider {
        id: normalize_managed_provider_id(payload.id),
        name: payload.name.trim().to_string(),
        kind: payload.kind,
        base_url: payload.base_url.trim().trim_end_matches('/').to_string(),
        api_key: payload.api_key.trim().to_string(),
        model: payload.model.trim().to_string(),
        managed: payload.managed,
    };

    validate_managed_provider(&provider)?;
    save_managed_provider(&app, &provider)?;

    Ok(Some(provider))
}

pub fn delete_provider(app: &AppHandle, provider_id: String) -> Result<(), String> {
    let connection = open_app_database(app)?;

    connection
        .execute(
            "delete from llm_providers where id = ?1 and managed = 0",
            params![provider_id],
        )
        .map_err(|error| error.to_string())?;

    Ok(())
}

async fn fetch_managed_provider() -> Result<ManagedProviderPayload, String> {
    reqwest::get(WAEY_MANAGED_PROVIDER_ENDPOINT)
        .await
        .map_err(|error| error.to_string())?
        .error_for_status()
        .map_err(|error| error.to_string())?
        .json::<ManagedProviderPayload>()
        .await
        .map_err(|error| error.to_string())
}

fn managed_provider_exists(app: &AppHandle) -> Result<bool, String> {
    let connection = open_app_database(app)?;
    let count: i64 = connection
        .query_row(
            "select count(*) from llm_providers where id = ?1 and managed = 1",
            params![WAEY_MANAGED_PROVIDER_ID],
            |row| row.get(0),
        )
        .map_err(|error| error.to_string())?;

    Ok(count > 0)
}

fn save_managed_provider(app: &AppHandle, provider: &LlmProvider) -> Result<(), String> {
    let connection = open_app_database(app)?;

    connection
        .execute(
            "insert into llm_providers (id, name, kind, base_url, api_key, model, managed, created_at, updated_at)
             values (?1, ?2, ?3, ?4, ?5, ?6, 1, strftime('%s','now'), strftime('%s','now'))
             on conflict(id) do nothing",
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

    Ok(())
}

fn validate_managed_provider(provider: &LlmProvider) -> Result<(), String> {
    if !provider.managed {
        return Err("Managed provider payload is not marked as managed.".to_string());
    }

    if provider.name.trim().is_empty()
        || provider.base_url.trim().is_empty()
        || provider.model.trim().is_empty()
        || provider.api_key.trim().is_empty()
    {
        return Err("Managed provider payload is incomplete.".to_string());
    }

    Ok(())
}

fn normalize_managed_provider_id(_id: String) -> String {
    WAEY_MANAGED_PROVIDER_ID.to_string()
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
