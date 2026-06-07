use crate::logger;
use rusqlite::Connection;
use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};
use tauri::{AppHandle, Manager};

pub fn open_app_database(app: &AppHandle) -> Result<Connection, String> {
    let path = database_path(app)?;

    match open_and_migrate_database(&path) {
        Ok(connection) => Ok(connection),
        Err(error) if should_recover_database(&error) => {
            logger::error(format!("database recovery started: {error}"));
            backup_database(&path)?;
            open_and_migrate_database(&path)
        }
        Err(error) => Err(error),
    }
}

fn open_and_migrate_database(path: &Path) -> Result<Connection, String> {
    let connection = Connection::open(path).map_err(|error| error.to_string())?;

    connection
        .execute_batch("pragma foreign_keys = on;")
        .map_err(|error| error.to_string())?;
    run_migrations(&connection)?;

    Ok(connection)
}

fn database_path(app: &AppHandle) -> Result<PathBuf, String> {
    let directory = app
        .path()
        .app_config_dir()
        .map_err(|error| error.to_string())?;

    fs::create_dir_all(&directory).map_err(|error| error.to_string())?;

    Ok(directory.join("waey.sqlite"))
}

fn backup_database(path: &Path) -> Result<(), String> {
    if !path.exists() {
        return Ok(());
    }

    let backup_path = path.with_extension(format!("sqlite.broken-{}", timestamp_seconds()));

    fs::rename(path, &backup_path).map_err(|error| error.to_string())?;
    logger::warn(format!(
        "moved unreadable database to {}",
        backup_path.to_string_lossy()
    ));
    Ok(())
}

fn should_recover_database(error: &str) -> bool {
    let lower_error = error.to_lowercase();

    lower_error.contains("malformed") || lower_error.contains("not a database")
}

fn timestamp_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default()
}

fn run_migrations(connection: &Connection) -> Result<(), String> {
    connection
        .execute_batch(
            "create table if not exists llm_providers (
                id text primary key,
                name text not null,
                kind text not null,
                base_url text not null,
                api_key text not null,
                model text not null,
                managed integer not null default 0,
                created_at integer not null,
                updated_at integer not null
            );

            create table if not exists conversations (
                id text primary key,
                title text not null,
                created_at integer not null,
                updated_at integer not null
            );

            create table if not exists chat_messages (
                id text primary key,
                conversation_id text not null,
                role text not null,
                content text not null,
                capture_path text,
                created_at integer not null,
                foreign key(conversation_id) references conversations(id) on delete cascade
            );

            create index if not exists idx_chat_messages_conversation_created
            on chat_messages(conversation_id, created_at);

            create table if not exists personas (
                id text primary key,
                name text not null,
                prompt text not null,
                created_at integer not null,
                updated_at integer not null
            );

            create table if not exists app_settings (
                key text primary key,
                value text not null
            );",
        )
        .map_err(|error| error.to_string())?;

    ensure_column(
        connection,
        "llm_providers",
        "managed",
        "alter table llm_providers add column managed integer not null default 0",
    )?;

    Ok(())
}

fn ensure_column(
    connection: &Connection,
    table_name: &str,
    column_name: &str,
    alter_statement: &str,
) -> Result<(), String> {
    let mut statement = connection
        .prepare(&format!("pragma table_info({table_name})"))
        .map_err(|error| error.to_string())?;
    let columns = statement
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(|error| error.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;

    if columns.iter().any(|column| column == column_name) {
        return Ok(());
    }

    connection
        .execute(alter_statement, [])
        .map_err(|error| error.to_string())?;

    Ok(())
}
