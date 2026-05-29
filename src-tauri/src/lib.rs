mod capture;
mod history;
mod llm;
mod logger;
mod personas;
mod providers;
mod settings;
mod storage;

use capture::{capture_full_screen, capture_screen_region, CaptureRect, ScreenCapture};
use history::{
    create_conversation, delete_conversation, list_conversations, list_messages, save_message,
    ChatMessage, ChatMessageDraft, Conversation, ConversationDraft,
};
use llm::{stream_chat_completion, LlmChatRequest};
use personas::{delete_persona, list_personas, save_persona, Persona, PersonaDraft};
use providers::{delete_provider, list_providers, save_provider, LlmProvider, ProviderDraft};
use serde::Serialize;
use settings::{get_settings, save_settings, AppSettings};
use tauri::{AppHandle, Emitter, Manager, WebviewWindow};

const MAIN_WINDOW_LABEL: &str = "main";
const REGION_WINDOW_LABEL: &str = "region-selector";
const CAPTURE_READY_EVENT: &str = "capture-ready";
const CAPTURE_ERROR_EVENT: &str = "capture-error";
const TRAY_OPEN_ID: &str = "open-waey";
const TRAY_QUIT_ID: &str = "quit-waey";

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CaptureError {
    source: &'static str,
    message: String,
    created_at: u128,
}

#[tauri::command]
fn show_overlay_window(app: AppHandle) -> Result<(), String> {
    show_overlay(&app)
}

#[tauri::command]
fn hide_overlay_window(app: AppHandle) -> Result<(), String> {
    hide_window(&app, MAIN_WINDOW_LABEL)
}

#[tauri::command]
fn show_region_selector_window(app: AppHandle) -> Result<(), String> {
    show_region_selector(&app)
}

#[tauri::command]
fn capture_current_screen(app: AppHandle) -> Result<ScreenCapture, String> {
    hide_window_safely(&app, MAIN_WINDOW_LABEL);
    std::thread::sleep(std::time::Duration::from_millis(150));
    let capture_result = capture_full_screen();

    restore_main_window(&app)?;

    match capture_result {
        Ok(capture) => {
            emit_capture_ready(&app, &capture)?;
            Ok(capture)
        }
        Err(error) => {
            emit_capture_error(&app, "fullScreen", &error);
            Err(error)
        }
    }
}

#[tauri::command]
fn capture_selected_region(app: AppHandle, rect: CaptureRect) -> Result<ScreenCapture, String> {
    hide_window_safely(&app, REGION_WINDOW_LABEL);
    std::thread::sleep(std::time::Duration::from_millis(150));
    let capture_result = capture_screen_region(rect);

    restore_main_window(&app)?;

    match capture_result {
        Ok(capture) => {
            emit_capture_ready(&app, &capture)?;
            Ok(capture)
        }
        Err(error) => {
            emit_capture_error(&app, "region", &error);
            Err(error)
        }
    }
}

#[tauri::command]
fn cancel_region_selection(app: AppHandle) -> Result<(), String> {
    hide_window(&app, REGION_WINDOW_LABEL)?;
    show_window(&app, MAIN_WINDOW_LABEL)
}

#[tauri::command]
fn list_llm_providers(app: AppHandle) -> Result<Vec<LlmProvider>, String> {
    list_providers(&app)
}

#[tauri::command]
fn save_llm_provider(app: AppHandle, provider: ProviderDraft) -> Result<LlmProvider, String> {
    save_provider(&app, provider)
}

#[tauri::command]
fn delete_llm_provider(app: AppHandle, provider_id: String) -> Result<(), String> {
    delete_provider(&app, provider_id)
}

#[tauri::command]
async fn send_llm_prompt(app: AppHandle, request: LlmChatRequest) -> Result<(), String> {
    stream_chat_completion(app, request).await
}

#[tauri::command]
fn list_chat_conversations(app: AppHandle) -> Result<Vec<Conversation>, String> {
    list_conversations(&app)
}

#[tauri::command]
fn create_chat_conversation(
    app: AppHandle,
    draft: ConversationDraft,
) -> Result<Conversation, String> {
    create_conversation(&app, draft)
}

#[tauri::command]
fn list_chat_messages(app: AppHandle, conversation_id: String) -> Result<Vec<ChatMessage>, String> {
    list_messages(&app, conversation_id)
}

#[tauri::command]
fn save_chat_message(app: AppHandle, message: ChatMessageDraft) -> Result<ChatMessage, String> {
    save_message(&app, message)
}

#[tauri::command]
fn delete_chat_conversation(app: AppHandle, conversation_id: String) -> Result<(), String> {
    delete_conversation(&app, conversation_id)
}

#[tauri::command]
fn list_prompt_personas(app: AppHandle) -> Result<Vec<Persona>, String> {
    list_personas(&app)
}

#[tauri::command]
fn save_prompt_persona(app: AppHandle, persona: PersonaDraft) -> Result<Persona, String> {
    save_persona(&app, persona)
}

#[tauri::command]
fn delete_prompt_persona(app: AppHandle, persona_id: String) -> Result<(), String> {
    delete_persona(&app, persona_id)
}

#[tauri::command]
fn get_app_settings(app: AppHandle) -> Result<AppSettings, String> {
    get_settings(&app)
}

#[tauri::command]
fn save_app_settings(app: AppHandle, settings: AppSettings) -> Result<AppSettings, String> {
    save_settings(&app, settings)
}

fn window_by_label(app: &AppHandle, label: &str) -> Result<WebviewWindow, String> {
    app.get_webview_window(label)
        .ok_or_else(|| format!("Window '{label}' was not found."))
}

fn show_overlay(app: &AppHandle) -> Result<(), String> {
    if !auto_capture_on_overlay(app) {
        logger::info("showing overlay without automatic capture");
        return restore_main_window(app);
    }

    hide_window_safely(app, MAIN_WINDOW_LABEL);
    std::thread::sleep(std::time::Duration::from_millis(150));
    let capture_result = capture_full_screen();

    restore_main_window(app)?;

    match capture_result {
        Ok(capture) => emit_capture_ready(app, &capture),
        Err(error) => {
            emit_capture_error(app, "fullScreen", &error);
            Ok(())
        }
    }
}

fn show_region_selector(app: &AppHandle) -> Result<(), String> {
    hide_window_safely(app, MAIN_WINDOW_LABEL);
    show_window(app, REGION_WINDOW_LABEL)
}

fn show_window(app: &AppHandle, label: &str) -> Result<(), String> {
    let window = window_by_label(app, label)?;
    window.show().map_err(|error| error.to_string())?;
    window.center().map_err(|error| error.to_string())?;
    window.set_focus().map_err(|error| error.to_string())
}

fn hide_window(app: &AppHandle, label: &str) -> Result<(), String> {
    let window = window_by_label(app, label)?;
    window.hide().map_err(|error| error.to_string())
}

fn hide_window_safely(app: &AppHandle, label: &str) {
    if let Err(error) = hide_window(app, label) {
        logger::warn(format!("failed to hide window '{label}': {error}"));
    }
}

fn restore_main_window(app: &AppHandle) -> Result<(), String> {
    show_window(app, MAIN_WINDOW_LABEL).map_err(|error| {
        logger::error(format!("failed to restore main window: {error}"));
        error
    })
}

fn emit_capture_ready(app: &AppHandle, capture: &ScreenCapture) -> Result<(), String> {
    app.emit(CAPTURE_READY_EVENT, capture)
        .map_err(|error| error.to_string())
}

fn emit_capture_error(app: &AppHandle, source: &'static str, message: &str) {
    logger::warn(format!("capture failed for {source}: {message}"));

    if let Err(error) = app.emit(
        CAPTURE_ERROR_EVENT,
        CaptureError {
            source,
            message: message.to_string(),
            created_at: current_timestamp_millis(),
        },
    ) {
        logger::warn(format!("failed to emit capture error: {error}"));
    }
}

fn auto_capture_on_overlay(app: &AppHandle) -> bool {
    match get_settings(app) {
        Ok(settings) => settings.auto_capture_on_overlay,
        Err(error) => {
            logger::warn(format!(
                "failed to read settings; using auto capture: {error}"
            ));
            true
        }
    }
}

fn current_timestamp_millis() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default()
}

fn register_global_shortcuts(app: &tauri::App) -> tauri::Result<()> {
    #[cfg(desktop)]
    {
        use tauri_plugin_global_shortcut::{
            Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState,
        };

        let overlay_shortcut = Shortcut::new(Some(Modifiers::ALT), Code::Space);
        let registered_overlay_shortcut = Shortcut::new(Some(Modifiers::ALT), Code::Space);
        let region_shortcut = Shortcut::new(Some(Modifiers::CONTROL), Code::Space);
        let registered_region_shortcut = Shortcut::new(Some(Modifiers::CONTROL), Code::Space);

        if let Err(error) = app.handle().plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(move |app, shortcut, event| {
                    if event.state() != ShortcutState::Pressed {
                        return;
                    }

                    let overlay_is_focused = app
                        .get_webview_window(MAIN_WINDOW_LABEL)
                        .and_then(|w| w.is_focused().ok())
                        .unwrap_or(false);

                    if overlay_is_focused {
                        return;
                    }

                    if shortcut == &overlay_shortcut {
                        if let Err(error) = show_overlay(app) {
                            logger::error(format!("failed to show Waey overlay: {error}"));
                        }
                    }
                    if shortcut == &region_shortcut {
                        if let Err(error) = show_region_selector(app) {
                            logger::error(format!("failed to show Waey region selector: {error}"));
                        }
                    }
                })
                .build(),
        ) {
            logger::error(format!(
                "failed to initialize global shortcut plugin: {error}"
            ));
            let _ = restore_main_window(app.handle());
            return Ok(());
        }

        if let Err(e) = app.global_shortcut().register(registered_overlay_shortcut) {
            logger::error(format!("failed to register overlay shortcut: {e}"));
            let _ = restore_main_window(app.handle());
        } else {
            logger::info("registered overlay shortcut");
        }
        if let Err(e) = app.global_shortcut().register(registered_region_shortcut) {
            logger::error(format!("failed to register region shortcut: {e}"));
        } else {
            logger::info("registered region shortcut");
        }
    }
    Ok(())
}

fn setup_tray(app: &tauri::App) -> tauri::Result<()> {
    #[cfg(desktop)]
    {
        use tauri::menu::{Menu, MenuItem};
        use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};

        let open_item = MenuItem::with_id(app, TRAY_OPEN_ID, "Open Waey", true, None::<&str>)?;
        let quit_item = MenuItem::with_id(app, TRAY_QUIT_ID, "Quit Waey", true, None::<&str>)?;
        let menu = Menu::with_items(app, &[&open_item, &quit_item])?;
        let mut tray_builder = TrayIconBuilder::with_id("waey-tray")
            .menu(&menu)
            .tooltip("Waey")
            .show_menu_on_left_click(false)
            .on_menu_event(|app, event| match event.id().as_ref() {
                TRAY_OPEN_ID => {
                    if let Err(error) = restore_main_window(app) {
                        logger::error(format!("failed to open Waey from tray: {error}"));
                    }
                }
                TRAY_QUIT_ID => app.exit(0),
                _ => {}
            })
            .on_tray_icon_event(|tray, event| {
                if let TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                } = event
                {
                    if let Err(error) = restore_main_window(tray.app_handle()) {
                        logger::error(format!("failed to open Waey from tray click: {error}"));
                    }
                }
            });

        if let Some(icon) = app.default_window_icon() {
            tray_builder = tray_builder.icon(icon.clone());
        }

        tray_builder.build(app)?;
        logger::info("tray initialized");
    }

    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let log_directory = app
                .path()
                .app_log_dir()
                .unwrap_or_else(|_| std::env::temp_dir().join("waey").join("logs"));
            logger::init(log_directory);
            logger::info("Waey starting");
            if let Err(error) = setup_tray(app) {
                logger::error(format!("failed to initialize tray: {error}"));
            }
            register_global_shortcuts(app)?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            show_overlay_window,
            hide_overlay_window,
            show_region_selector_window,
            capture_current_screen,
            capture_selected_region,
            cancel_region_selection,
            list_llm_providers,
            save_llm_provider,
            delete_llm_provider,
            send_llm_prompt,
            list_chat_conversations,
            create_chat_conversation,
            list_chat_messages,
            save_chat_message,
            delete_chat_conversation,
            list_prompt_personas,
            save_prompt_persona,
            delete_prompt_persona,
            get_app_settings,
            save_app_settings
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
