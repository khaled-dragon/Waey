use std::{
    sync::atomic::{AtomicU64, Ordering},
    time::Duration,
};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize, WebviewWindow};

pub const GUIDE_WINDOW_LABEL: &str = "guide-overlay";
pub const GUIDE_HIGHLIGHT_WINDOW_LABEL: &str = "guide-highlight";
const GUIDE_STEP_EVENT: &str = "guide-overlay-step";
const GUIDE_HIGHLIGHT_EVENT: &str = "guide-highlight-target";
const GUIDE_OFFER_STARTED_EVENT: &str = "guide-offer-started";
const GUIDE_STEP_CONFIRMED_EVENT: &str = "guide-step-confirmed";
const GUIDE_CANCELLED_EVENT: &str = "guide-cancelled";
pub const GUIDE_ADJUSTMENT_REQUESTED_EVENT: &str = "guide-adjustment-requested";
const GUIDE_WINDOW_WIDTH: u32 = 292;
const GUIDE_WINDOW_HEIGHT: u32 = 174;
const GUIDE_WINDOW_GAP: i32 = 18;
const GUIDE_MOVE_DURATION_MS: u64 = 190;
const GUIDE_MOVE_STEPS: u64 = 12;

static GUIDE_MOVE_GENERATION: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GuideOverlayRequest {
    pub mode: GuideOverlayMode,
    pub caption: String,
    pub target: Option<GuideTarget>,
    pub step_index: u32,
    pub estimated_steps_left: u32,
    pub theme: GuideTheme,
    pub is_rtl: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum GuideOverlayMode {
    Offer,
    Step,
    Thinking,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GuideTarget {
    pub label: Option<String>,
    pub automation_id: Option<String>,
    pub bounds: Option<GuideBounds>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GuideBounds {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum GuideTheme {
    Light,
    Dark,
}

pub fn show_guide_overlay(app: &AppHandle, request: GuideOverlayRequest) -> Result<(), String> {
    validate_guide_request(&request)?;
    let window = app
        .get_webview_window(GUIDE_WINDOW_LABEL)
        .ok_or_else(|| "Guide window was not found.".to_string())?;
    let target_position = guide_position(&window, request.target.as_ref())?;
    let was_visible = window.is_visible().map_err(|error| error.to_string())?;
    let start_position = if was_visible {
        window.outer_position().map_err(|error| error.to_string())?
    } else {
        initial_guide_position(&window, request.target.as_ref())?
    };

    window
        .set_size(PhysicalSize::new(GUIDE_WINDOW_WIDTH, GUIDE_WINDOW_HEIGHT))
        .map_err(|error| error.to_string())?;
    window
        .set_position(start_position)
        .map_err(|error| error.to_string())?;
    window.unminimize().map_err(|error| error.to_string())?;
    window.show().map_err(|error| error.to_string())?;
    update_guide_highlight(app, &request)?;
    app.emit(GUIDE_STEP_EVENT, &request)
        .map_err(|error| error.to_string())?;

    animate_guide_window(window, start_position, target_position);

    Ok(())
}

pub fn start_guide_offer(app: &AppHandle) -> Result<(), String> {
    hide_guide_overlay(app)?;
    app.emit(GUIDE_OFFER_STARTED_EVENT, ())
        .map_err(|error| error.to_string())
}

pub fn complete_guide_step(app: &AppHandle) -> Result<(), String> {
    hide_guide_overlay(app)?;
    app.emit(GUIDE_STEP_CONFIRMED_EVENT, ())
        .map_err(|error| error.to_string())
}

pub fn request_guide_adjustment(app: &AppHandle) -> Result<(), String> {
    hide_guide_overlay(app)
}

pub fn cancel_guide(app: &AppHandle) -> Result<(), String> {
    hide_guide_overlay(app)?;
    app.emit(GUIDE_CANCELLED_EVENT, ())
        .map_err(|error| error.to_string())
}

pub fn hide_guide_overlay(app: &AppHandle) -> Result<(), String> {
    let window = app
        .get_webview_window(GUIDE_WINDOW_LABEL)
        .ok_or_else(|| "Guide window was not found.".to_string())?;

    window.hide().map_err(|error| error.to_string())?;
    hide_guide_highlight(app)
}

fn update_guide_highlight(app: &AppHandle, request: &GuideOverlayRequest) -> Result<(), String> {
    let highlight = app
        .get_webview_window(GUIDE_HIGHLIGHT_WINDOW_LABEL)
        .ok_or_else(|| "Guide highlight window was not found.".to_string())?;
    let Some(bounds) = request
        .target
        .as_ref()
        .and_then(|target| target.bounds.as_ref())
    else {
        return highlight.hide().map_err(|error| error.to_string());
    };

    const PADDING: i32 = 8;
    let width = bounds.width.saturating_add((PADDING * 2) as u32);
    let height = bounds.height.saturating_add((PADDING * 2) as u32);
    highlight
        .set_size(PhysicalSize::new(width, height))
        .map_err(|error| error.to_string())?;
    highlight
        .set_position(PhysicalPosition::new(
            bounds.x - PADDING,
            bounds.y - PADDING,
        ))
        .map_err(|error| error.to_string())?;
    highlight
        .set_ignore_cursor_events(true)
        .map_err(|error| error.to_string())?;
    highlight.show().map_err(|error| error.to_string())?;
    app.emit(GUIDE_HIGHLIGHT_EVENT, &request.theme)
        .map_err(|error| error.to_string())
}

fn hide_guide_highlight(app: &AppHandle) -> Result<(), String> {
    let highlight = app
        .get_webview_window(GUIDE_HIGHLIGHT_WINDOW_LABEL)
        .ok_or_else(|| "Guide highlight window was not found.".to_string())?;
    highlight.hide().map_err(|error| error.to_string())
}

fn validate_guide_request(request: &GuideOverlayRequest) -> Result<(), String> {
    if request.caption.trim().is_empty() {
        return Err("Guide step caption is required.".to_string());
    }

    if request.caption.chars().count() > 600 {
        return Err("Guide step caption is too long.".to_string());
    }

    if request.mode == GuideOverlayMode::Step && request.step_index == 0 {
        return Err("Guide step index must start at 1.".to_string());
    }

    if let Some(bounds) = request
        .target
        .as_ref()
        .and_then(|target| target.bounds.as_ref())
    {
        if bounds.width == 0
            || bounds.height == 0
            || bounds.width > 10_000
            || bounds.height > 10_000
        {
            return Err("Guide target bounds are invalid.".to_string());
        }
    }

    Ok(())
}

fn guide_position(
    window: &tauri::WebviewWindow,
    target: Option<&GuideTarget>,
) -> Result<PhysicalPosition<i32>, String> {
    let monitor = window
        .current_monitor()
        .map_err(|error| error.to_string())?;
    let Some(monitor) = monitor else {
        return Ok(PhysicalPosition::new(64, 64));
    };

    let monitor_position = monitor.position();
    let monitor_size = monitor.size();
    let minimum_x = monitor_position.x;
    let minimum_y = monitor_position.y;
    let maximum_x = minimum_x + monitor_size.width as i32 - GUIDE_WINDOW_WIDTH as i32;
    let maximum_y = minimum_y + monitor_size.height as i32 - GUIDE_WINDOW_HEIGHT as i32;

    let (desired_x, desired_y) = target
        .and_then(|target| target.bounds.as_ref())
        .map(|bounds| {
            let right_x = bounds.x + bounds.width as i32 + GUIDE_WINDOW_GAP;
            let left_x = bounds.x - GUIDE_WINDOW_WIDTH as i32 - GUIDE_WINDOW_GAP;
            let centered_y =
                bounds.y + (bounds.height as i32 / 2) - (GUIDE_WINDOW_HEIGHT as i32 / 2);
            let x = if right_x <= maximum_x {
                right_x
            } else {
                left_x
            };
            (x, centered_y)
        })
        .unwrap_or((minimum_x + 40, minimum_y + 40));

    let x = desired_x.clamp(minimum_x, maximum_x.max(minimum_x));
    let y = desired_y.clamp(minimum_y, maximum_y.max(minimum_y));

    Ok(PhysicalPosition::new(x, y))
}

fn initial_guide_position(
    window: &WebviewWindow,
    target: Option<&GuideTarget>,
) -> Result<PhysicalPosition<i32>, String> {
    let monitor = window
        .current_monitor()
        .map_err(|error| error.to_string())?;
    let Some(monitor) = monitor else {
        return Ok(PhysicalPosition::new(64, 64));
    };

    let monitor_position = monitor.position();
    let monitor_size = monitor.size();
    let minimum_x = monitor_position.x;
    let minimum_y = monitor_position.y;
    let maximum_x = minimum_x + monitor_size.width as i32 - GUIDE_WINDOW_WIDTH as i32;
    let maximum_y = minimum_y + monitor_size.height as i32 - GUIDE_WINDOW_HEIGHT as i32;
    let desired_x = target
        .and_then(|guide_target| guide_target.bounds.as_ref())
        .map(|bounds| bounds.x - GUIDE_WINDOW_WIDTH as i32 - (GUIDE_WINDOW_GAP * 2))
        .unwrap_or(minimum_x + 36);
    let desired_y = target
        .and_then(|guide_target| guide_target.bounds.as_ref())
        .map(|bounds| bounds.y - 24)
        .unwrap_or(minimum_y + 36);

    Ok(PhysicalPosition::new(
        desired_x.clamp(minimum_x, maximum_x.max(minimum_x)),
        desired_y.clamp(minimum_y, maximum_y.max(minimum_y)),
    ))
}

fn animate_guide_window(
    window: WebviewWindow,
    start: PhysicalPosition<i32>,
    target: PhysicalPosition<i32>,
) {
    if start == target {
        return;
    }

    let generation = GUIDE_MOVE_GENERATION.fetch_add(1, Ordering::SeqCst) + 1;
    std::thread::spawn(move || {
        let pause = Duration::from_millis(GUIDE_MOVE_DURATION_MS / GUIDE_MOVE_STEPS);

        for step in 1..=GUIDE_MOVE_STEPS {
            if GUIDE_MOVE_GENERATION.load(Ordering::SeqCst) != generation {
                return;
            }

            let progress = step as f64 / GUIDE_MOVE_STEPS as f64;
            let eased = 1.0 - (1.0 - progress).powi(3);
            let x = start.x + ((target.x - start.x) as f64 * eased).round() as i32;
            let y = start.y + ((target.y - start.y) as f64 * eased).round() as i32;

            if window.set_position(PhysicalPosition::new(x, y)).is_err() {
                return;
            }

            std::thread::sleep(pause);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::{validate_guide_request, GuideOverlayMode, GuideOverlayRequest, GuideTheme};

    fn request() -> GuideOverlayRequest {
        GuideOverlayRequest {
            mode: GuideOverlayMode::Step,
            caption: "Open the File menu.".to_string(),
            target: None,
            step_index: 1,
            estimated_steps_left: 2,
            theme: GuideTheme::Dark,
            is_rtl: false,
        }
    }

    #[test]
    fn rejects_an_empty_caption() {
        let mut invalid_request = request();
        invalid_request.caption.clear();

        assert!(validate_guide_request(&invalid_request).is_err());
    }

    #[test]
    fn accepts_a_non_targeted_instruction() {
        assert!(validate_guide_request(&request()).is_ok());
    }
}
