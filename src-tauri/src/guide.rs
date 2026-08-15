use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize};

pub const GUIDE_WINDOW_LABEL: &str = "guide-overlay";
const GUIDE_STEP_EVENT: &str = "guide-overlay-step";
const GUIDE_STEP_CONFIRMED_EVENT: &str = "guide-step-confirmed";
const GUIDE_CANCELLED_EVENT: &str = "guide-cancelled";
const GUIDE_WINDOW_WIDTH: u32 = 292;
const GUIDE_WINDOW_HEIGHT: u32 = 174;
const GUIDE_WINDOW_GAP: i32 = 18;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GuideOverlayRequest {
    pub caption: String,
    pub target: Option<GuideTarget>,
    pub step_index: u32,
    pub estimated_steps_left: u32,
    pub theme: GuideTheme,
    pub is_rtl: bool,
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

    window
        .set_size(PhysicalSize::new(GUIDE_WINDOW_WIDTH, GUIDE_WINDOW_HEIGHT))
        .map_err(|error| error.to_string())?;
    window
        .set_position(guide_position(&window, request.target.as_ref())?)
        .map_err(|error| error.to_string())?;
    window.unminimize().map_err(|error| error.to_string())?;
    window.show().map_err(|error| error.to_string())?;
    app.emit(GUIDE_STEP_EVENT, &request)
        .map_err(|error| error.to_string())?;
    window.set_focus().map_err(|error| error.to_string())?;

    Ok(())
}

pub fn complete_guide_step(app: &AppHandle) -> Result<(), String> {
    hide_guide_overlay(app)?;
    app.emit(GUIDE_STEP_CONFIRMED_EVENT, ())
        .map_err(|error| error.to_string())
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

    window.hide().map_err(|error| error.to_string())
}

fn validate_guide_request(request: &GuideOverlayRequest) -> Result<(), String> {
    if request.caption.trim().is_empty() {
        return Err("Guide step caption is required.".to_string());
    }

    if request.caption.chars().count() > 600 {
        return Err("Guide step caption is too long.".to_string());
    }

    if request.step_index == 0 {
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

#[cfg(test)]
mod tests {
    use super::{validate_guide_request, GuideOverlayRequest, GuideTheme};

    fn request() -> GuideOverlayRequest {
        GuideOverlayRequest {
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
