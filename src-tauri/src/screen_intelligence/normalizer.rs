use super::{collection_diagnostics, ScreenContextSnapshot, UiContextRect, UiElementSummary};

const MAX_CONTEXT_ELEMENTS: usize = 600;
const MAX_NAME_LENGTH: usize = 240;
const MAX_VALUE_LENGTH: usize = 8_000;
const MAX_SELECTED_TEXT_LENGTH: usize = 2_400;
const MAX_PARENT_TRAIL_ITEMS: usize = 6;
const MAX_PARENT_TRAIL_ITEM_LENGTH: usize = 120;

pub fn normalize_snapshot(
    mut snapshot: ScreenContextSnapshot,
    region: Option<&UiContextRect>,
    elapsed_ms: u64,
) -> ScreenContextSnapshot {
    snapshot.active_window_title = clean_optional(snapshot.active_window_title, MAX_NAME_LENGTH);
    snapshot.active_app_name = clean_optional(snapshot.active_app_name, 120);
    snapshot.selected_text = clean_optional(snapshot.selected_text, MAX_SELECTED_TEXT_LENGTH);
    snapshot.selected_text_source = clean_optional(snapshot.selected_text_source, 48);
    snapshot.focused_element = snapshot.focused_element.and_then(normalize_element);
    snapshot.pointed_element = snapshot.pointed_element.and_then(normalize_element);
    snapshot.visible_windows = snapshot
        .visible_windows
        .into_iter()
        .filter_map(|mut window| {
            window.title = clean_text(&window.title, MAX_NAME_LENGTH);
            window.app_name = clean_optional(window.app_name, 120);
            (!window.title.is_empty() && has_area(&window.bounds)).then_some(window)
        })
        .take(12)
        .collect();

    let mut elements = snapshot
        .elements
        .into_iter()
        .filter_map(normalize_element)
        .filter(|element| has_area(&element.bounds))
        .filter(|element| {
            region
                .map(|target| intersects(&element.bounds, target))
                .unwrap_or(true)
        })
        .filter(|element| !is_empty_scaffold(element))
        .collect::<Vec<_>>();

    elements.sort_by_key(element_priority);
    let truncated = elements.len() > MAX_CONTEXT_ELEMENTS;
    elements.truncate(MAX_CONTEXT_ELEMENTS);

    snapshot.diagnostics = collection_diagnostics(elapsed_ms, elements.len(), truncated);
    snapshot.elements = elements;
    snapshot
}

fn normalize_element(mut element: UiElementSummary) -> Option<UiElementSummary> {
    if looks_sensitive(&element) {
        return None;
    }

    element.role = clean_text(&element.role, 64);
    element.name = clean_text(&element.name, MAX_NAME_LENGTH);
    element.value = clean_optional(element.value, MAX_VALUE_LENGTH);
    element.selected_text = clean_optional(element.selected_text, MAX_SELECTED_TEXT_LENGTH);
    element.automation_id = clean_optional(element.automation_id, 160);
    element.class_name = clean_optional(element.class_name, 160);
    element.parent_trail = element
        .parent_trail
        .into_iter()
        .map(|part| clean_text(&part, MAX_PARENT_TRAIL_ITEM_LENGTH))
        .filter(|part| !part.is_empty())
        .take(MAX_PARENT_TRAIL_ITEMS)
        .collect();

    let has_content = !element.name.is_empty()
        || element
            .value
            .as_deref()
            .is_some_and(|value| !value.is_empty())
        || element
            .selected_text
            .as_deref()
            .is_some_and(|value| !value.is_empty())
        || element.focused
        || element.under_cursor;

    has_content.then_some(element)
}

fn looks_sensitive(element: &UiElementSummary) -> bool {
    let fingerprint = format!(
        "{} {} {} {}",
        element.role,
        element.name,
        element.automation_id.as_deref().unwrap_or_default(),
        element.class_name.as_deref().unwrap_or_default()
    )
    .to_ascii_lowercase();

    [
        "password",
        "passcode",
        "one-time",
        "otp",
        "security code",
        "cvv",
        "credit card",
    ]
    .iter()
    .any(|needle| fingerprint.contains(needle))
}

fn is_empty_scaffold(element: &UiElementSummary) -> bool {
    matches!(element.role.as_str(), "Pane" | "Group" | "Custom")
        && element.value.is_none()
        && element.selected_text.is_none()
        && element.name.is_empty()
        && !element.focused
        && !element.under_cursor
}

fn element_priority(element: &UiElementSummary) -> (u8, u16, String) {
    let priority = if element.under_cursor {
        0
    } else if element.focused {
        1
    } else if matches!(
        element.role.as_str(),
        "Button"
            | "Edit"
            | "ComboBox"
            | "CheckBox"
            | "RadioButton"
            | "MenuItem"
            | "TabItem"
            | "Hyperlink"
    ) {
        2
    } else if matches!(element.role.as_str(), "Document" | "Text") {
        3
    } else {
        4
    };

    (priority, element.depth, element.name.clone())
}

fn clean_optional(value: Option<String>, max_length: usize) -> Option<String> {
    value
        .map(|item| clean_text(&item, max_length))
        .filter(|item| !item.is_empty())
}

fn clean_text(value: &str, max_length: usize) -> String {
    let normalized = value.split_whitespace().collect::<Vec<_>>().join(" ");
    if normalized.chars().count() <= max_length {
        return normalized;
    }

    let shortened = normalized
        .chars()
        .take(max_length.saturating_sub(1))
        .collect::<String>();
    format!("{shortened}…")
}

fn has_area(rect: &UiContextRect) -> bool {
    rect.width > 0 && rect.height > 0
}

fn intersects(a: &UiContextRect, b: &UiContextRect) -> bool {
    let a_right = a.x.saturating_add(a.width as i32);
    let a_bottom = a.y.saturating_add(a.height as i32);
    let b_right = b.x.saturating_add(b.width as i32);
    let b_bottom = b.y.saturating_add(b.height as i32);

    a.x < b_right && a_right > b.x && a.y < b_bottom && a_bottom > b.y
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::screen_intelligence::{ScreenContextDiagnostics, ScreenContextPoint};

    fn element(role: &str, name: &str) -> UiElementSummary {
        UiElementSummary {
            role: role.to_string(),
            name: name.to_string(),
            value: None,
            selected_text: None,
            automation_id: None,
            class_name: None,
            bounds: UiContextRect {
                x: 0,
                y: 0,
                width: 20,
                height: 20,
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

    fn snapshot(elements: Vec<UiElementSummary>) -> ScreenContextSnapshot {
        ScreenContextSnapshot {
            schema_version: 2,
            platform: "windows".to_string(),
            active_window_title: Some("Test".to_string()),
            active_app_name: None,
            selected_text: None,
            selected_text_source: None,
            captured_at: 0,
            region: None,
            cursor: Some(ScreenContextPoint { x: 1, y: 1 }),
            active_window_bounds: None,
            focused_element: None,
            pointed_element: None,
            visible_windows: Vec::new(),
            elements,
            diagnostics: ScreenContextDiagnostics::default(),
        }
    }

    #[test]
    fn sensitive_controls_are_removed_before_prompt_routing() {
        let normal = element("Edit", "Email");
        let secret = element("Edit", "Password");
        let result = normalize_snapshot(snapshot(vec![normal, secret]), None, 20);

        assert_eq!(result.elements.len(), 1);
        assert_eq!(result.elements[0].name, "Email");
    }

    #[test]
    fn pointed_elements_are_prioritized() {
        let regular = element("Button", "Later");
        let mut pointed = element("Text", "Current");
        pointed.under_cursor = true;
        let result = normalize_snapshot(snapshot(vec![regular, pointed]), None, 20);

        assert_eq!(result.elements[0].name, "Current");
    }
}
