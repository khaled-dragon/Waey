use crate::storage::open_app_database;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use tauri::AppHandle;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub hotkey_overlay: String,
    pub hotkey_region: String,
    pub theme: ThemePreference,
    pub language: LanguagePreference,
    pub auto_capture_on_overlay: bool,
    pub selected_provider_id: Option<String>,
    pub selected_persona_id: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ThemePreference {
    System,
    Dark,
    Light,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LanguagePreference {
    En,
    Ar,
}

pub fn get_settings(app: &AppHandle) -> Result<AppSettings, String> {
    let connection = open_app_database(app)?;
    let mut settings = default_settings();
    let mut statement = connection
        .prepare("select key, value from app_settings")
        .map_err(|error| error.to_string())?;

    let rows = statement
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(|error| error.to_string())?;

    for row in rows {
        let (key, value) = row.map_err(|error| error.to_string())?;

        apply_setting_value(&mut settings, key.as_str(), value.as_str());
    }

    Ok(settings)
}

pub fn save_settings(app: &AppHandle, settings: AppSettings) -> Result<AppSettings, String> {
    validate_settings(&settings)?;

    let connection = open_app_database(app)?;
    let values = [
        ("hotkey_overlay", settings.hotkey_overlay.clone()),
        ("hotkey_region", settings.hotkey_region.clone()),
        ("theme", theme_to_string(&settings.theme).to_string()),
        (
            "language",
            language_to_string(&settings.language).to_string(),
        ),
        (
            "auto_capture_on_overlay",
            bool_to_string(settings.auto_capture_on_overlay).to_string(),
        ),
        (
            "selected_provider_id",
            settings.selected_provider_id.clone().unwrap_or_default(),
        ),
        (
            "selected_persona_id",
            settings.selected_persona_id.clone().unwrap_or_default(),
        ),
    ];

    for (key, value) in values {
        connection
            .execute(
                "insert into app_settings (key, value)
                 values (?1, ?2)
                 on conflict(key) do update set value = excluded.value",
                params![key, value],
            )
            .map_err(|error| error.to_string())?;
    }

    Ok(settings)
}

pub fn default_settings() -> AppSettings {
    AppSettings {
        hotkey_overlay: "Alt+Space".to_string(),
        hotkey_region: "Ctrl+Space".to_string(),
        theme: ThemePreference::Dark,
        language: LanguagePreference::En,
        auto_capture_on_overlay: true,
        selected_provider_id: None,
        selected_persona_id: None,
    }
}

fn validate_settings(settings: &AppSettings) -> Result<(), String> {
    if settings.hotkey_overlay.trim().is_empty() {
        return Err("Overlay hotkey is required.".to_string());
    }

    if settings.hotkey_region.trim().is_empty() {
        return Err("Region hotkey is required.".to_string());
    }

    Ok(())
}

fn apply_setting_value(settings: &mut AppSettings, key: &str, value: &str) {
    match key {
        "hotkey_overlay" => settings.hotkey_overlay = value.to_string(),
        "hotkey_region" => settings.hotkey_region = value.to_string(),
        "theme" => settings.theme = theme_from_string(value),
        "language" => settings.language = language_from_string(value),
        "auto_capture_on_overlay" => settings.auto_capture_on_overlay = value == "true",
        "selected_provider_id" => settings.selected_provider_id = optional_setting(value),
        "selected_persona_id" => settings.selected_persona_id = optional_setting(value),
        _ => {}
    }
}

fn theme_to_string(theme: &ThemePreference) -> &'static str {
    match theme {
        ThemePreference::System => "system",
        ThemePreference::Dark => "dark",
        ThemePreference::Light => "light",
    }
}

fn theme_from_string(value: &str) -> ThemePreference {
    match value {
        "system" => ThemePreference::System,
        "light" => ThemePreference::Light,
        _ => ThemePreference::Dark,
    }
}

fn language_to_string(language: &LanguagePreference) -> &'static str {
    match language {
        LanguagePreference::En => "en",
        LanguagePreference::Ar => "ar",
    }
}

fn language_from_string(value: &str) -> LanguagePreference {
    match value {
        "ar" => LanguagePreference::Ar,
        _ => LanguagePreference::En,
    }
}

fn bool_to_string(value: bool) -> &'static str {
    if value {
        "true"
    } else {
        "false"
    }
}

fn optional_setting(value: &str) -> Option<String> {
    let trimmed_value = value.trim();

    if trimmed_value.is_empty() {
        None
    } else {
        Some(trimmed_value.to_string())
    }
}
