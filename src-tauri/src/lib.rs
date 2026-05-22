mod capture;
mod history;
mod llm;
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
use settings::{get_settings, save_settings, AppSettings};
use tauri::{AppHandle, Emitter, Manager, WebviewWindow};

const MAIN_WINDOW_LABEL: &str = "main";
const REGION_WINDOW_LABEL: &str = "region-selector";
const CAPTURE_READY_EVENT: &str = "capture-ready";

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
    show_window(&app, REGION_WINDOW_LABEL)
}

#[tauri::command]
fn capture_current_screen(app: AppHandle) -> Result<ScreenCapture, String> {
    let capture = capture_full_screen()?;
    app.emit(CAPTURE_READY_EVENT, &capture)
        .map_err(|error| error.to_string())?;

    Ok(capture)
}

#[tauri::command]
fn capture_selected_region(app: AppHandle, rect: CaptureRect) -> Result<ScreenCapture, String> {
    let capture = capture_screen_region(rect)?;

    hide_window(&app, REGION_WINDOW_LABEL)?;
    show_window(&app, MAIN_WINDOW_LABEL)?;
    app.emit(CAPTURE_READY_EVENT, &capture)
        .map_err(|error| error.to_string())?;

    Ok(capture)
}

#[tauri::command]
fn cancel_region_selection(app: AppHandle) -> Result<(), String> {
    hide_window(&app, REGION_WINDOW_LABEL)
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
    let capture = capture_full_screen()?;

    show_window(app, MAIN_WINDOW_LABEL)?;
    app.emit(CAPTURE_READY_EVENT, &capture)
        .map_err(|error| error.to_string())
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

        app.handle().plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(move |app, shortcut, event| {
                    if event.state() != ShortcutState::Pressed {
                        return;
                    }

                    if shortcut == &overlay_shortcut {
                        if let Err(error) = show_overlay(app) {
                            eprintln!("Failed to show Waey overlay: {error}");
                        }
                    }

                    if shortcut == &region_shortcut {
                        if let Err(error) = show_window(app, REGION_WINDOW_LABEL) {
                            eprintln!("Failed to show Waey region selector: {error}");
                        }
                    }
                })
                .build(),
        )?;

        app.global_shortcut()
            .register(registered_overlay_shortcut)
            .map_err(|e| tauri::Error::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?;
        app.global_shortcut()
            .register(registered_region_shortcut)
            .map_err(|e| tauri::Error::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?;
    }

    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
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
