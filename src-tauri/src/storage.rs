use rusqlite::Connection;
use std::{fs, path::PathBuf};
use tauri::{AppHandle, Manager};

pub fn open_app_database(app: &AppHandle) -> Result<Connection, String> {
    let connection = Connection::open(database_path(app)?).map_err(|error| error.to_string())?;

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
        .map_err(|error| error.to_string())
}
