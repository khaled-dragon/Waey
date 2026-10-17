//! Narrow, verified desktop actions derived from a fresh UI Automation snapshot.
//!
//! The model can only *propose* one of these actions. The renderer requires a
//! separate user click, then the native collector re-resolves the target before
//! acting. This keeps model text from becoming an unchecked desktop command.

use super::{UiContextRect, UiElementSummary};
use crate::settings::DesktopActionCapability;
use serde::{Deserialize, Serialize};

const MAX_SUMMARY_LENGTH: usize = 240;
const MAX_TARGET_TEXT_LENGTH: usize = 240;
const MAX_ENTRY_TEXT_LENGTH: usize = 2_000;
const MAX_ACTION_DIMENSION: u32 = 10_000;
const BOUNDS_TOLERANCE_PX: i32 = 32;
const SAFE_CLICK_ROLES: &[&str] = &["Button", "MenuItem", "Hyperlink"];
const SAFE_TEXT_ENTRY_ROLES: &[&str] = &["Edit", "ComboBox"];
const SAFE_SCROLL_ROLES: &[&str] = &["Document", "List", "DataGrid", "Tree"];
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
const BLOCKED_ENTRY_TERMS: &[&str] = &[
    "password",
    "passcode",
    "api key",
    "api_key",
    "access token",
    "refresh token",
    "bearer ",
    "secret",
    "private key",
    "ssh-rsa",
    "كلمة المرور",
    "كلمه المرور",
    "رمز المرور",
    "مفتاح api",
    "مفتاح الوصول",
    "رمز الوصول",
    "سر ",
];

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopActionExecutionRequest {
    pub action: DesktopAction,
    pub approved: bool,
}

/// A tagged union makes each capability's required payload explicit. It is
/// deliberately not a free-form command or coordinate-based instruction.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum DesktopAction {
    Click {
        summary: String,
        target: DesktopActionTarget,
    },
    SetText {
        summary: String,
        target: DesktopActionTarget,
        text: String,
    },
    Scroll {
        summary: String,
        target: DesktopActionTarget,
        direction: DesktopScrollDirection,
    },
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum DesktopActionKind {
    Click,
    SetText,
    Scroll,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum DesktopScrollDirection {
    Up,
    Down,
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
    pub kind: DesktopActionKind,
    pub summary: String,
    pub target: DesktopActionTarget,
    pub verified: bool,
}

impl DesktopAction {
    pub fn kind(&self) -> DesktopActionKind {
        match self {
            Self::Click { .. } => DesktopActionKind::Click,
            Self::SetText { .. } => DesktopActionKind::SetText,
            Self::Scroll { .. } => DesktopActionKind::Scroll,
        }
    }

    pub fn summary(&self) -> &str {
        match self {
            Self::Click { summary, .. }
            | Self::SetText { summary, .. }
            | Self::Scroll { summary, .. } => summary,
        }
    }

    pub fn target(&self) -> &DesktopActionTarget {
        match self {
            Self::Click { target, .. }
            | Self::SetText { target, .. }
            | Self::Scroll { target, .. } => target,
        }
    }

    pub fn required_capability(&self) -> Option<DesktopActionCapability> {
        match self {
            Self::Click { .. } => None,
            Self::SetText { .. } => Some(DesktopActionCapability::TextEntry),
            Self::Scroll { .. } => Some(DesktopActionCapability::ScreenNavigation),
        }
    }
}

pub fn validate_desktop_action(action: &DesktopAction) -> Result<(), String> {
    validate_text(action.summary(), "action summary", MAX_SUMMARY_LENGTH)?;

    match action {
        DesktopAction::Click { target, .. } => validate_target(
            target,
            SAFE_CLICK_ROLES,
            "Waey can only propose verified button, menu, or link clicks.",
        ),
        DesktopAction::SetText { target, text, .. } => {
            validate_target(
                target,
                SAFE_TEXT_ENTRY_ROLES,
                "Waey can only enter text in a verified edit field or combo box.",
            )?;
            validate_text(text, "text to enter", MAX_ENTRY_TEXT_LENGTH)?;
            if text.contains('\0') {
                return Err("Text entry cannot contain a null character.".to_string());
            }
            if contains_blocked_entry_term(text) {
                return Err(
                    "Waey will not enter passwords, tokens, keys, or other secrets.".to_string(),
                );
            }
            Ok(())
        }
        DesktopAction::Scroll { target, .. } => validate_target(
            target,
            SAFE_SCROLL_ROLES,
            "Waey can only scroll a verified document, list, data grid, or tree.",
        ),
    }
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

fn validate_target(
    target: &DesktopActionTarget,
    allowed_roles: &[&str],
    invalid_role_message: &str,
) -> Result<(), String> {
    if !allowed_roles.contains(&target.role.as_str()) {
        return Err(invalid_role_message.to_string());
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

fn validate_text(value: &str, field: &str, maximum_length: usize) -> Result<(), String> {
    let length = value.trim().chars().count();
    if length == 0 || length > maximum_length {
        return Err(format!(
            "The {field} must contain 1 to {maximum_length} characters."
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
            validate_text(value, field, MAX_TARGET_TEXT_LENGTH)?;
            Ok(Some(value))
        }
        None => Ok(None),
    }
}

fn contains_blocked_entry_term(value: &str) -> bool {
    let value = value.to_lowercase();
    BLOCKED_ENTRY_TERMS.iter().any(|term| value.contains(term))
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
        matches_desktop_action_target, validate_desktop_action, DesktopAction, DesktopActionTarget,
        DesktopScrollDirection,
    };
    use crate::screen_intelligence::{UiContextRect, UiElementSummary};
    use crate::settings::DesktopActionCapability;

    fn target(role: &str, label: Option<&str>) -> DesktopActionTarget {
        DesktopActionTarget {
            role: role.to_string(),
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
        let action = DesktopAction::Click {
            summary: "Open Settings".to_string(),
            target: target("Button", Some("Settings")),
        };

        assert!(validate_desktop_action(&action).is_ok());
        assert!(matches_desktop_action_target(&candidate(), action.target()));
    }

    #[test]
    fn rejects_sensitive_controls_before_uia_is_called() {
        let action = DesktopAction::Click {
            summary: "Confirm".to_string(),
            target: target("Button", Some("Delete account")),
        };

        assert!(validate_desktop_action(&action).is_err());
    }

    #[test]
    fn allows_explicit_text_entry_only_for_edit_controls() {
        let action = DesktopAction::SetText {
            summary: "Fill search".to_string(),
            target: target("Edit", Some("Search")),
            text: "Waey".to_string(),
        };

        assert!(validate_desktop_action(&action).is_ok());
    }

    #[test]
    fn rejects_secret_text_and_unsupported_entry_targets() {
        let secret = DesktopAction::SetText {
            summary: "Fill field".to_string(),
            target: target("Edit", Some("Search")),
            text: "my api key is secret".to_string(),
        };
        let unsupported_target = DesktopAction::SetText {
            summary: "Fill field".to_string(),
            target: target("Button", Some("Search")),
            text: "Waey".to_string(),
        };

        assert!(validate_desktop_action(&secret).is_err());
        assert!(validate_desktop_action(&unsupported_target).is_err());
    }

    #[test]
    fn allows_narrow_scroll_proposals() {
        let action = DesktopAction::Scroll {
            summary: "Scroll the article".to_string(),
            target: target("Document", Some("Article")),
            direction: DesktopScrollDirection::Down,
        };

        assert!(validate_desktop_action(&action).is_ok());
    }

    #[test]
    fn expanded_actions_declare_their_required_capability() {
        let text_entry = DesktopAction::SetText {
            summary: "Fill search".to_string(),
            target: target("Edit", Some("Search")),
            text: "Waey".to_string(),
        };
        let scroll = DesktopAction::Scroll {
            summary: "Scroll the article".to_string(),
            target: target("Document", Some("Article")),
            direction: DesktopScrollDirection::Down,
        };

        assert_eq!(
            text_entry.required_capability(),
            Some(DesktopActionCapability::TextEntry)
        );
        assert_eq!(
            scroll.required_capability(),
            Some(DesktopActionCapability::ScreenNavigation)
        );
    }

    #[test]
    fn requires_all_supplied_identifiers_to_match() {
        let mut action_target = target("Button", Some("Settings"));
        action_target.automation_id = Some("different".to_string());

        assert!(!matches_desktop_action_target(&candidate(), &action_target));
    }
}
