mod capture;
mod dev_context;
mod guide;
mod history;
mod llm;
mod logger;
mod personas;
mod providers;
mod screen_intelligence;
mod settings;
mod spreadsheet;
mod storage;
mod ui_context;
mod workspace;

use capture::{capture_full_screen, capture_screen_region, CaptureRect, ScreenCapture};
use dev_context::{build_developer_context, write_developer_file};
use guide::{
    cancel_guide as dismiss_guide, complete_guide_step as confirm_guide,
    request_guide_adjustment as prepare_guide_adjustment, show_guide_overlay,
    start_guide_offer as begin_guide_offer, GuideOverlayRequest, GUIDE_ADJUSTMENT_REQUESTED_EVENT,
    GUIDE_WINDOW_LABEL,
};
use history::{
    create_conversation, delete_conversation, delete_message, list_conversations, list_messages,
    rename_conversation, save_message, set_conversation_pin, ChatMessage, ChatMessageDraft,
    Conversation, ConversationDraft, ConversationPinDraft, ConversationRenameDraft,
};
use llm::{cancel_chat_completion, stream_chat_completion, LlmChatRequest, LlmRequestRegistry};
use personas::{delete_persona, list_personas, save_persona, Persona, PersonaDraft};
use providers::{
    apply_waey_provider_update, bootstrap_waey_provider, check_waey_provider_update,
    delete_provider, list_providers, save_provider, LlmProvider, ManagedProviderUpdate,
    ProviderDraft,
};
use screen_intelligence::{
    record_desktop_action_outcome, DesktopActionAuditOutcome, DesktopActionExecutionRequest,
    DesktopActionKind, DesktopActionResult,
};
use serde::Serialize;
use settings::{get_settings, save_settings, AppSettings};
use spreadsheet::apply_developer_spreadsheet_edit;
use std::{sync::Mutex, thread};
use tauri::{AppHandle, Emitter, Manager, WebviewWindow};
use ui_context::{capture_ui_context, UiContextSnapshot};

const MAIN_WINDOW_LABEL: &str = "main";
const REGION_WINDOW_LABEL: &str = "region-selector";
const OVERLAY_OPENED_EVENT: &str = "overlay-opened";
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

#[derive(Default)]
struct ScreenContextTarget {
    window_handle: Mutex<Option<isize>>,
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
    let target_window_handle = screen_context_target_handle(&app);
    hide_window_safely(&app, MAIN_WINDOW_LABEL);
    std::thread::sleep(std::time::Duration::from_millis(150));
    let capture_result = capture_full_screen(target_window_handle);

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
    let target_window_handle = screen_context_target_handle(&app);
    hide_window_safely(&app, REGION_WINDOW_LABEL);
    std::thread::sleep(std::time::Duration::from_millis(150));
    let capture_result = capture_screen_region(rect, target_window_handle);

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
fn capture_current_ui_context(
    app: AppHandle,
    allow_clipboard_selection: bool,
    force_fresh: bool,
) -> Result<Option<UiContextSnapshot>, String> {
    if !attach_ui_context(&app) {
        return Ok(None);
    }

    let target_window_handle = screen_context_target_handle(&app);
    let current_cursor = screen_intelligence::capture_cursor_position();
    let context_cache = app
        .state::<screen_intelligence::ScreenContextCache>()
        .inner()
        .clone();
    // Clipboard-derived selection is intentionally never served from the
    // prefetch cache: the user's one-time permission applies only to this
    // exact request. Guide confirmations also require a new observation.
    let may_reuse_prefetch = !allow_clipboard_selection && !force_fresh;
    let context_result = context_cache.capture_or_reuse(
        target_window_handle,
        current_cursor,
        may_reuse_prefetch,
        || capture_ui_context(None, allow_clipboard_selection, target_window_handle),
    );
    if let Err(error) = &context_result {
        logger::warn(format!(
            "screen context capture failed for target {:?}: {error}",
            target_window_handle
        ));
    }
    context_result
}

/// Applies one renderer-approved semantic action. This intentionally accepts no
/// coordinates, keyboard input, shell command, or file operation. The native
/// UIA worker re-reads the original window and invokes only an exact, safe
/// control match before returning success.
#[tauri::command]
async fn execute_desktop_action(
    app: AppHandle,
    request: DesktopActionExecutionRequest,
) -> Result<DesktopActionResult, String> {
    let action_kind = request.action.kind();
    if !request.approved {
        audit_desktop_action(
            &app,
            action_kind,
            DesktopActionAuditOutcome::Blocked,
            "approval_required",
        );
        return Err("Desktop actions require a direct approval in Waey.".to_string());
    }

    let settings = get_settings(&app)?;
    if !settings.attach_ui_context {
        audit_desktop_action(
            &app,
            action_kind,
            DesktopActionAuditOutcome::Blocked,
            "screen_context_disabled",
        );
        return Err("Enable screen context before applying a verified desktop action.".to_string());
    }

    if let Some(capability) = request.action.required_capability() {
        if !settings.desktop_action_capabilities.contains(&capability) {
            audit_desktop_action(
                &app,
                action_kind,
                DesktopActionAuditOutcome::Blocked,
                "capability_not_granted",
            );
            return Err(
                "Enable this desktop capability in Settings before applying it.".to_string(),
            );
        }
    }

    if let Err(error) = screen_intelligence::validate_desktop_action(&request.action) {
        audit_desktop_action(
            &app,
            action_kind,
            DesktopActionAuditOutcome::Blocked,
            "invalid_action_payload",
        );
        return Err(error);
    }
    let target_window_handle = screen_context_target_handle(&app)
        .filter(|handle| *handle != 0)
        .ok_or_else(|| {
            audit_desktop_action(
                &app,
                action_kind,
                DesktopActionAuditOutcome::Blocked,
                "missing_target_window",
            );
            "Open Waey from the app you want to control, then request the action again.".to_string()
        })?;

    logger::info("applying an approved, verified semantic desktop action");
    hide_window_safely(&app, MAIN_WINDOW_LABEL);
    let action = request.action;
    let result = tauri::async_runtime::spawn_blocking(move || {
        screen_intelligence::execute_desktop_action(action, Some(target_window_handle))
    })
    .await;

    if let Err(error) = restore_main_window(&app) {
        logger::error(format!(
            "failed to restore Waey after desktop action: {error}"
        ));
        audit_desktop_action(
            &app,
            action_kind,
            DesktopActionAuditOutcome::Failed,
            "overlay_restore_failed",
        );
        return Err(error);
    }

    match result {
        Ok(Ok(result)) => {
            audit_desktop_action(
                &app,
                action_kind,
                DesktopActionAuditOutcome::Applied,
                "verified_execution",
            );
            Ok(result)
        }
        Ok(Err(error)) => {
            audit_desktop_action(
                &app,
                action_kind,
                DesktopActionAuditOutcome::Failed,
                "uia_execution_rejected",
            );
            Err(error)
        }
        Err(error) => {
            audit_desktop_action(
                &app,
                action_kind,
                DesktopActionAuditOutcome::Failed,
                "action_worker_unavailable",
            );
            Err(format!("Desktop action worker ended unexpectedly: {error}"))
        }
    }
}

fn audit_desktop_action(
    app: &AppHandle,
    action_kind: DesktopActionKind,
    outcome: DesktopActionAuditOutcome,
    reason_code: &'static str,
) {
    if let Err(error) = record_desktop_action_outcome(app, action_kind, outcome, reason_code) {
        logger::warn(format!("desktop action audit write failed: {error}"));
    }
}

#[tauri::command]
fn show_guide_step(app: AppHandle, request: GuideOverlayRequest) -> Result<(), String> {
    hide_window_safely(&app, MAIN_WINDOW_LABEL);

    if let Err(error) = show_guide_overlay(&app, request) {
        let _ = restore_main_window(&app);
        return Err(error);
    }

    Ok(())
}

#[tauri::command]
fn complete_guide_step(app: AppHandle) -> Result<(), String> {
    confirm_guide(&app)
}

#[tauri::command]
fn request_guide_adjustment(app: AppHandle) -> Result<(), String> {
    prepare_guide_adjustment(&app)?;
    restore_main_window(&app)?;
    app.emit(GUIDE_ADJUSTMENT_REQUESTED_EVENT, ())
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn start_guide_offer(app: AppHandle) -> Result<(), String> {
    begin_guide_offer(&app)
}

#[tauri::command]
fn cancel_guide_step(app: AppHandle) -> Result<(), String> {
    dismiss_guide(&app)?;
    restore_main_window(&app)
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
async fn bootstrap_managed_provider(app: AppHandle) -> Result<Option<LlmProvider>, String> {
    bootstrap_waey_provider(app).await
}

#[tauri::command]
async fn check_managed_provider_update(
    app: AppHandle,
) -> Result<Option<ManagedProviderUpdate>, String> {
    check_waey_provider_update(app).await
}

#[tauri::command]
async fn apply_managed_provider_update(app: AppHandle) -> Result<LlmProvider, String> {
    apply_waey_provider_update(app).await
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
fn cancel_llm_prompt(app: AppHandle, request_id: String) -> Result<(), String> {
    cancel_chat_completion(&app, request_id)
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
fn rename_chat_conversation(
    app: AppHandle,
    draft: ConversationRenameDraft,
) -> Result<Conversation, String> {
    rename_conversation(&app, draft)
}

#[tauri::command]
fn pin_chat_conversation(
    app: AppHandle,
    draft: ConversationPinDraft,
) -> Result<Conversation, String> {
    set_conversation_pin(&app, draft)
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
fn delete_chat_message(app: AppHandle, message_id: String) -> Result<(), String> {
    delete_message(&app, message_id)
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
    remember_screen_context_target(app);
    prefetch_screen_context(app);
    app.emit(OVERLAY_OPENED_EVENT, ())
        .map_err(|error| error.to_string())?;

    if !auto_capture_on_overlay(app) {
        logger::info("showing overlay without automatic capture");
        return restore_main_window(app);
    }

    hide_window_safely(app, MAIN_WINDOW_LABEL);
    std::thread::sleep(std::time::Duration::from_millis(150));
    let capture_result = capture_full_screen(screen_context_target_handle(app));

    restore_main_window(app)?;

    match capture_result {
        Ok(capture) => emit_capture_ready(app, &capture),
        Err(error) => {
            emit_capture_error(app, "fullScreen", &error);
            Ok(())
        }
    }
}

/// Starts UIA collection while the user is opening Waey, rather than delaying
/// the first prompt. The prompt path owns the final freshness decision and
/// surfaces any collection failure; this best-effort task only warms the cache.
fn prefetch_screen_context(app: &AppHandle) {
    if !attach_ui_context(app) {
        return;
    }

    let target_window_handle = screen_context_target_handle(app);
    let context_cache = app
        .state::<screen_intelligence::ScreenContextCache>()
        .inner()
        .clone();

    match context_cache.try_begin_capture(target_window_handle) {
        Ok(true) => {}
        Ok(false) => {
            logger::info("screen context prefetch skipped because a collection is already running");
            return;
        }
        Err(error) => {
            logger::warn(format!("failed to begin screen context prefetch: {error}"));
            return;
        }
    }

    let prefetch_cache = context_cache.clone();
    let spawn_result = thread::Builder::new()
        .name("waey-screen-context-prefetch".to_string())
        .spawn(move || {
            let result = capture_ui_context(None, false, target_window_handle);
            if let Err(error) = &result {
                logger::warn(format!(
                    "screen context prefetch failed for target {:?}: {error}",
                    target_window_handle
                ));
            }
            prefetch_cache.finish_capture(target_window_handle, &result);
        });

    if let Err(error) = spawn_result {
        let failure: Result<Option<UiContextSnapshot>, String> =
            Err(format!("Unable to start screen context prefetch: {error}"));
        context_cache.finish_capture(target_window_handle, &failure);
        logger::warn(format!("failed to spawn screen context prefetch: {error}"));
    }
}

fn show_region_selector(app: &AppHandle) -> Result<(), String> {
    remember_screen_context_target(app);
    hide_window_safely(app, MAIN_WINDOW_LABEL);
    show_window(app, REGION_WINDOW_LABEL)
}

fn remember_screen_context_target(app: &AppHandle) {
    let Some(window_handle) = screen_intelligence::capture_foreground_window_handle() else {
        logger::warn("could not capture the foreground window for screen context");
        return;
    };

    let state = app.state::<ScreenContextTarget>();
    match state.window_handle.lock() {
        Ok(mut stored_handle) => {
            *stored_handle = Some(window_handle);
            logger::info(format!(
                "stored screen context target window: {window_handle}"
            ));
        }
        Err(_) => logger::warn("screen context target lock was poisoned"),
    };
}

fn screen_context_target_handle(app: &AppHandle) -> Option<isize> {
    let window_handle = app
        .state::<ScreenContextTarget>()
        .window_handle
        .lock()
        .map(|stored_handle| *stored_handle)
        .unwrap_or_else(|_| {
            logger::warn("screen context target lock was poisoned");
            None
        });

    window_handle
}

fn show_window(app: &AppHandle, label: &str) -> Result<(), String> {
    let window = window_by_label(app, label)?;
    window.unminimize().map_err(|error| error.to_string())?;
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

fn attach_ui_context(app: &AppHandle) -> bool {
    match get_settings(app) {
        Ok(settings) => settings.attach_ui_context,
        Err(error) => {
            logger::warn(format!(
                "failed to read settings; attaching UI context: {error}"
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
        let region_shortcut = Shortcut::new(Some(Modifiers::CONTROL), Code::Space);

        if let Err(error) = app.handle().plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(move |app, shortcut, event| {
                    if event.state() != ShortcutState::Pressed {
                        return;
                    }

                    if any_waey_window_is_visible(app) {
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

        if let Err(error) = app.global_shortcut().register(overlay_shortcut) {
            logger::error(format!("failed to register Waey overlay shortcut: {error}"));
            let _ = restore_main_window(app.handle());
        }

        if let Err(error) = app.global_shortcut().register(region_shortcut) {
            logger::error(format!("failed to register Waey region shortcut: {error}"));
        }

        logger::info("registered fixed shortcuts: overlay=Alt+Space, region=Ctrl+Space");
    }
    Ok(())
}

fn any_waey_window_is_visible(app: &AppHandle) -> bool {
    [MAIN_WINDOW_LABEL, REGION_WINDOW_LABEL, GUIDE_WINDOW_LABEL]
        .iter()
        .any(|label| {
            app.get_webview_window(label)
                .and_then(|window| window.is_visible().ok())
                .unwrap_or(false)
        })
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
        .manage(LlmRequestRegistry::default())
        .manage(ScreenContextTarget::default())
        .manage(screen_intelligence::ScreenContextCache::default())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Err(error) = restore_main_window(app) {
                logger::error(format!(
                    "failed to restore Waey from second instance: {error}"
                ));
            }
        }))
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            let log_directory = app
                .path()
                .app_log_dir()
                .unwrap_or_else(|_| std::env::temp_dir().join("waey").join("logs"));
            logger::init(log_directory);
            logger::info("Waey starting");
            screen_intelligence::warm_up_windows_ui_automation();
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
            capture_current_ui_context,
            execute_desktop_action,
            show_guide_step,
            start_guide_offer,
            complete_guide_step,
            request_guide_adjustment,
            cancel_guide_step,
            cancel_region_selection,
            bootstrap_managed_provider,
            check_managed_provider_update,
            apply_managed_provider_update,
            list_llm_providers,
            save_llm_provider,
            delete_llm_provider,
            send_llm_prompt,
            cancel_llm_prompt,
            list_chat_conversations,
            create_chat_conversation,
            rename_chat_conversation,
            pin_chat_conversation,
            list_chat_messages,
            save_chat_message,
            delete_chat_message,
            delete_chat_conversation,
            list_prompt_personas,
            save_prompt_persona,
            delete_prompt_persona,
            get_app_settings,
            save_app_settings,
            build_developer_context,
            write_developer_file,
            apply_developer_spreadsheet_edit
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
