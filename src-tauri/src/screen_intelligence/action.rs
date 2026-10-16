//! Narrow, verified desktop actions derived from a fresh UI Automation snapshot.
//!
//! The model can only *propose* one of these actions. The renderer requires a
//! separate user click, then the native collector re-resolves the target before
//! invoking it. This keeps model text from becoming an unchecked desktop command.

use super::{UiContextRect, UiElementSummary};
use serde::{Deserialize, Serialize};

const MAX_TEXT_LENGTH: usize = 240;
const MAX_ACTION_DIMENSION: u32 = 10_000;
const BOUNDS_TOLERANCE_PX: i32 = 32;
const SAFE_CLICK_ROLES: &[&str] = &["Button", "MenuItem", "Hyperlink"];
const BLOCKED_TARGET_TERMS: &[&str] = &[
    "delete",
    "remove",
    "erase",
    "discard",
    "uninstall",
    "format",
    "reset",
    "shutdown",
    "restart",
    "sign out",
    "log out",
    "purchase",
    "buy",
    "pay",
    "checkout",
    "submit",
    "send",
    "share",
    "publish",
    "upload",
    "download",
    "install",
    "allow",
    "permit",
    "grant",
    "password",
    "credential",
    "security",
    "close",
    "cancel",
    "حذف",
    "إزالة",
    "ازالة",
    "مسح",
    "إلغاء",
    "الغاء",
    "إرسال",
    "ارسال",
    "شراء",
    "دفع",
    "تثبيت",
    "تحميل",
    "تنزيل",
    "صلاحيات",
    "كلمة المرور",
    "كلمه المرور",
    "إعادة تشغيل",
    "اعادة تشغيل",
    "تسجيل الخروج",
    "إغلاق",
    "اغلاق",
];

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopActionExecutionRequest {
    pub action: DesktopAction,
    pub approved: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopAction {
    pub kind: DesktopActionKind,
    pub summary: String,
    pub target: DesktopActionTarget,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum DesktopActionKind {
    Click,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopActionTarget {
    pub role: String,
    pub label: Option<String>,
    pub automation_id: Option<String>,
    pub bounds: UiContextRect,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopActionResult {
    pub summary: String,
    pub target: DesktopActionTarget,
    pub verified: bool,
}

pub fn validate_desktop_action(action: &DesktopAction) -> Result<(), String> {
    if !matches!(action.kind, DesktopActionKind::Click) {
        return Err("That desktop action is not supported.".to_string());
    }

    validate_text(&action.summary, "action summary")?;
    validate_target(&action.target)
}

pub fn matches_desktop_action_target(
    candidate: &UiElementSummary,
    expected: &DesktopActionTarget,
) -> bool {
    if !candidate.is_enabled || candidate.is_offscreen || candidate.role != expected.role {
        return false;
    }

    if !matches_optional_text(expected.label.as_deref(), Some(candidate.name.as_str()))
        || !matches_optional_text(
            expected.automation_id.as_deref(),
            candidate.automation_id.as_deref(),
        )
    {
        return false;
    }

    rects_are_close(&candidate.bounds, &expected.bounds)
}

fn validate_target(target: &DesktopActionTarget) -> Result<(), String> {
    if !SAFE_CLICK_ROLES.contains(&target.role.as_str()) {
        return Err("Waey can only propose verified button, menu, or link clicks.".to_string());
    }

    let label = validate_optional_text(target.label.as_deref(), "target label")?;
    let automation_id =
        validate_optional_text(target.automation_id.as_deref(), "target automation id")?;

    if label.is_none() && automation_id.is_none() {
        return Err("A desktop action needs a visible label or automation id.".to_string());
    }

    if target.bounds.width == 0
        || target.bounds.height == 0
        || target.bounds.width > MAX_ACTION_DIMENSION
        || target.bounds.height > MAX_ACTION_DIMENSION
    {
        return Err("The desktop action target has invalid bounds.".to_string());
    }

    let target_description = format!(
        "{} {}",
        label.unwrap_or_default(),
        automation_id.unwrap_or_default()
    )
    .to_lowercase();
    if BLOCKED_TARGET_TERMS
        .iter()
        .any(|term| target_description.contains(term))
    {
        return Err("Waey will not automate sensitive or destructive controls.".to_string());
    }

    Ok(())
}

fn validate_text(value: &str, field: &str) -> Result<(), String> {
    let length = value.trim().chars().count();
    if length == 0 || length > MAX_TEXT_LENGTH {
        return Err(format!(
            "The {field} must contain 1 to {MAX_TEXT_LENGTH} characters."
        ));
    }

    Ok(())
}

fn validate_optional_text<'a>(
    value: Option<&'a str>,
    field: &str,
) -> Result<Option<&'a str>, String> {
    match value {
        Some(value) => {
            validate_text(value, field)?;
            Ok(Some(value))
        }
        None => Ok(None),
    }
}

fn matches_optional_text(expected: Option<&str>, actual: Option<&str>) -> bool {
    match expected {
        Some(expected) => actual
            .map(normalize_for_match)
            .is_some_and(|actual| actual == normalize_for_match(expected)),
        None => true,
    }
}

fn normalize_for_match(value: &str) -> String {
    value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

fn rects_are_close(left: &UiContextRect, right: &UiContextRect) -> bool {
    (i64::from(left.x) - i64::from(right.x)).abs() <= i64::from(BOUNDS_TOLERANCE_PX)
        && (i64::from(left.y) - i64::from(right.y)).abs() <= i64::from(BOUNDS_TOLERANCE_PX)
        && (left.width as i64 - right.width as i64).abs() <= i64::from(BOUNDS_TOLERANCE_PX)
        && (left.height as i64 - right.height as i64).abs() <= i64::from(BOUNDS_TOLERANCE_PX)
}

#[cfg(test)]
mod tests {
    use super::{
        matches_desktop_action_target, validate_desktop_action, DesktopAction, DesktopActionKind,
        DesktopActionTarget,
    };
    use crate::screen_intelligence::{UiContextRect, UiElementSummary};

    fn target(label: Option<&str>) -> DesktopActionTarget {
        DesktopActionTarget {
            role: "Button".to_string(),
            label: label.map(str::to_string),
            automation_id: Some("settingsButton".to_string()),
            bounds: UiContextRect {
                x: 120,
                y: 80,
                width: 96,
                height: 32,
            },
        }
    }

    fn candidate() -> UiElementSummary {
        UiElementSummary {
            role: "Button".to_string(),
            name: "Settings".to_string(),
            value: None,
            selected_text: None,
            automation_id: Some("settingsButton".to_string()),
            class_name: None,
            bounds: UiContextRect {
                x: 126,
                y: 76,
                width: 94,
                height: 34,
            },
            focused: false,
            under_cursor: false,
            is_enabled: true,
            is_offscreen: false,
            depth: 1,
            child_count: 0,
            parent_trail: Vec::new(),
        }
    }

    #[test]
    fn allows_a_narrow_verified_click() {
        let action = DesktopAction {
            kind: DesktopActionKind::Click,
            summary: "Open Settings".to_string(),
            target: target(Some("Settings")),
        };

        assert!(validate_desktop_action(&action).is_ok());
        assert!(matches_desktop_action_target(&candidate(), &action.target));
    }

    #[test]
    fn rejects_sensitive_controls_before_uia_is_called() {
        let action = DesktopAction {
            kind: DesktopActionKind::Click,
            summary: "Confirm".to_string(),
            target: target(Some("Delete account")),
        };

        assert!(validate_desktop_action(&action).is_err());
    }

    #[test]
    fn requires_all_supplied_identifiers_to_match() {
        let mut action_target = target(Some("Settings"));
        action_target.automation_id = Some("different".to_string());

        assert!(!matches_desktop_action_target(&candidate(), &action_target));
    }
}
