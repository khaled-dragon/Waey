//! Persistent, in-process Windows UI Automation collection.
//!
//! UI Automation objects are COM apartment-bound. Keeping all of them on one
//! dedicated STA thread avoids the process startup, assembly loading, and temp
//! file work that made the legacy PowerShell collector noticeably slow.

use super::{
    ScreenContextCollector, ScreenContextPoint, ScreenContextSnapshot, UiContextRect,
    UiElementSummary, VisibleWindowSummary,
};
use std::sync::{
    mpsc::{self, Receiver, SyncSender, TrySendError},
    Mutex, OnceLock,
};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use windows::{
    core::Interface,
    Win32::{
        Foundation::{HWND, POINT, RECT},
        System::Com::{
            CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_INPROC_SERVER,
            COINIT_APARTMENTTHREADED,
        },
        UI::{
            Accessibility::{
                CUIAutomation, IUIAutomation, IUIAutomation2, IUIAutomationCacheRequest,
                IUIAutomationElement, IUIAutomationTextPattern, IUIAutomationValuePattern,
                TreeScope_Children, TreeScope_Descendants, TreeScope_Element,
                UIA_AutomationIdPropertyId, UIA_BoundingRectanglePropertyId,
                UIA_ButtonControlTypeId, UIA_CheckBoxControlTypeId, UIA_ClassNamePropertyId,
                UIA_ComboBoxControlTypeId, UIA_ControlTypePropertyId, UIA_CustomControlTypeId,
                UIA_DocumentControlTypeId, UIA_EditControlTypeId, UIA_GroupControlTypeId,
                UIA_HasKeyboardFocusPropertyId, UIA_HyperlinkControlTypeId,
                UIA_IsEnabledPropertyId, UIA_IsOffscreenPropertyId, UIA_IsPasswordPropertyId,
                UIA_ListItemControlTypeId, UIA_MenuItemControlTypeId, UIA_NamePropertyId,
                UIA_NativeWindowHandlePropertyId, UIA_PaneControlTypeId, UIA_ProcessIdPropertyId,
                UIA_RadioButtonControlTypeId, UIA_TabItemControlTypeId, UIA_TextControlTypeId,
                UIA_TextPatternId, UIA_TreeItemControlTypeId, UIA_ValuePatternId,
                UIA_WindowControlTypeId,
            },
            WindowsAndMessaging::{GetCursorPos, GetForegroundWindow},
        },
    },
};

const MAX_CONTEXT_ELEMENTS: usize = 420;
const MAX_VISIBLE_WINDOWS: usize = 12;
const UIA_TRANSACTION_TIMEOUT_MS: u32 = 700;
const WORKER_START_TIMEOUT: Duration = Duration::from_millis(750);
const REQUEST_TIMEOUT: Duration = Duration::from_millis(1_250);

static WORKER: OnceLock<Mutex<Option<NativeUiaWorker>>> = OnceLock::new();

#[derive(Clone)]
struct NativeUiaWorker {
    sender: SyncSender<CaptureRequest>,
}

struct CaptureRequest {
    region: Option<UiContextRect>,
    target_window_handle: Option<isize>,
    response: mpsc::Sender<Result<ScreenContextSnapshot, String>>,
}

struct NativeCollector {
    automation: IUIAutomation,
    cache_request: IUIAutomationCacheRequest,
}

pub fn warm_up() {
    if let Err(error) = worker_sender() {
        crate::logger::warn(format!("native UI Automation warm-up failed: {error}"));
    }
}

pub fn capture(
    region: Option<UiContextRect>,
    target_window_handle: Option<isize>,
) -> Result<ScreenContextSnapshot, String> {
    let sender = worker_sender()?;
    let (response_sender, response_receiver) = mpsc::channel();
    let request = CaptureRequest {
        region,
        target_window_handle,
        response: response_sender,
    };

    match sender.try_send(request) {
        Ok(()) => {}
        Err(TrySendError::Full(_)) => {
            return Err("Native UI Automation is still collecting a previous snapshot.".to_string())
        }
        Err(TrySendError::Disconnected(_)) => {
            reset_worker();
            return Err("Native UI Automation worker stopped unexpectedly.".to_string());
        }
    }

    match response_receiver.recv_timeout(REQUEST_TIMEOUT) {
        Ok(result) => result,
        Err(mpsc::RecvTimeoutError::Timeout) => {
            // Releasing the sender lets a wedged worker exit as soon as Windows
            // returns. The next capture gets a clean STA and never waits behind it.
            reset_worker();
            Err(format!(
                "Native UI Automation timed out after {} ms.",
                REQUEST_TIMEOUT.as_millis()
            ))
        }
        Err(mpsc::RecvTimeoutError::Disconnected) => {
            reset_worker();
            Err("Native UI Automation worker stopped before returning data.".to_string())
        }
    }
}

fn worker_sender() -> Result<SyncSender<CaptureRequest>, String> {
    let worker = worker_slot();
    let mut current = worker
        .lock()
        .map_err(|_| "Native UI Automation worker lock was poisoned.".to_string())?;

    if current.is_none() {
        *current = Some(NativeUiaWorker::start()?);
    }

    current
        .as_ref()
        .map(|worker| worker.sender.clone())
        .ok_or_else(|| "Native UI Automation worker was not initialized.".to_string())
}

fn reset_worker() {
    if let Ok(mut worker) = worker_slot().lock() {
        *worker = None;
    }
}

fn worker_slot() -> &'static Mutex<Option<NativeUiaWorker>> {
    WORKER.get_or_init(|| Mutex::new(None))
}

impl NativeUiaWorker {
    fn start() -> Result<Self, String> {
        let (request_sender, request_receiver) = mpsc::sync_channel(1);
        let (ready_sender, ready_receiver) = mpsc::sync_channel(1);

        thread::Builder::new()
            .name("waey-native-uia".to_string())
            .spawn(move || run_worker(request_receiver, ready_sender))
            .map_err(|error| format!("Unable to start native UI Automation worker: {error}"))?;

        ready_receiver
            .recv_timeout(WORKER_START_TIMEOUT)
            .map_err(|_| "Native UI Automation worker did not start in time.".to_string())??;

        Ok(Self {
            sender: request_sender,
        })
    }
}

fn run_worker(receiver: Receiver<CaptureRequest>, ready: SyncSender<Result<(), String>>) {
    if let Err(error) = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) }.ok() {
        let _ = ready.send(Err(format!(
            "Unable to initialize the UI Automation apartment: {error}"
        )));
        return;
    }

    let collector = NativeCollector::new();
    let ready_result = collector
        .as_ref()
        .map(|_| ())
        .map_err(|error| error.clone());
    let _ = ready.send(ready_result);

    let Ok(collector) = collector else {
        unsafe { CoUninitialize() };
        return;
    };

    while let Ok(request) = receiver.recv() {
        let result = collector.capture(request.region, request.target_window_handle);
        let _ = request.response.send(result);
    }

    unsafe { CoUninitialize() };
}

impl NativeCollector {
    fn new() -> Result<Self, String> {
        let automation: IUIAutomation = unsafe {
            CoCreateInstance(&CUIAutomation, None, CLSCTX_INPROC_SERVER)
                .map_err(|error| format!("Unable to create UI Automation: {error}"))?
        };

        if let Ok(automation2) = automation.cast::<IUIAutomation2>() {
            unsafe {
                let _ = automation2.SetConnectionTimeout(UIA_TRANSACTION_TIMEOUT_MS);
                let _ = automation2.SetTransactionTimeout(UIA_TRANSACTION_TIMEOUT_MS);
            }
        }

        let cache_request = Self::cache_request(&automation)?;
        Ok(Self {
            automation,
            cache_request,
        })
    }

    fn cache_request(automation: &IUIAutomation) -> Result<IUIAutomationCacheRequest, String> {
        let cache_request = unsafe {
            automation
                .CreateCacheRequest()
                .map_err(|error| format!("Unable to create UI Automation cache request: {error}"))?
        };

        unsafe {
            cache_request
                .SetTreeScope(TreeScope_Element)
                .map_err(|error| {
                    format!("Unable to configure UI Automation cache scope: {error}")
                })?;

            for property in [
                UIA_NamePropertyId,
                UIA_ControlTypePropertyId,
                UIA_AutomationIdPropertyId,
                UIA_ClassNamePropertyId,
                UIA_BoundingRectanglePropertyId,
                UIA_HasKeyboardFocusPropertyId,
                UIA_IsEnabledPropertyId,
                UIA_IsOffscreenPropertyId,
                UIA_IsPasswordPropertyId,
                UIA_ProcessIdPropertyId,
                UIA_NativeWindowHandlePropertyId,
            ] {
                cache_request.AddProperty(property).map_err(|error| {
                    format!("Unable to configure UI Automation cached property: {error}")
                })?;
            }
        }

        Ok(cache_request)
    }

    fn capture(
        &self,
        region: Option<UiContextRect>,
        target_window_handle: Option<isize>,
    ) -> Result<ScreenContextSnapshot, String> {
        let started_at = Instant::now();
        let cursor = capture_cursor_position();
        let target_handle = target_window_handle
            .filter(|handle| *handle != 0)
            .map(HWND)
            .unwrap_or_else(|| unsafe { GetForegroundWindow() });

        let active_window = unsafe {
            self.automation
                .ElementFromHandleBuildCache(target_handle, &self.cache_request)
                .map_err(|error| {
                    format!("Unable to read the active UI Automation window: {error}")
                })?
        };
        let active_process_id = cached_process_id(&active_window);
        let focused_element = unsafe {
            self.automation
                .GetFocusedElementBuildCache(&self.cache_request)
                .ok()
        }
        .filter(|element| same_process(cached_process_id(element), active_process_id));
        let pointed_element = cursor
            .as_ref()
            .and_then(|point| unsafe {
                self.automation
                    .ElementFromPointBuildCache(
                        POINT {
                            x: point.x,
                            y: point.y,
                        },
                        &self.cache_request,
                    )
                    .ok()
            })
            .filter(|element| same_process(cached_process_id(element), active_process_id));

        let active_summary = self.element_summary(&active_window, cursor.clone(), 0, false);
        let focused_summary = focused_element
            .as_ref()
            .and_then(|element| self.element_summary(element, cursor.clone(), 0, true));
        let pointed_summary = pointed_element
            .as_ref()
            .and_then(|element| self.element_summary(element, cursor.clone(), 0, false));
        let selected_text = focused_element
            .as_ref()
            .and_then(selected_text_from_element);

        let (mut elements, elements_truncated) =
            self.window_elements(&active_window, cursor.clone())?;
        if let Some(active) = active_summary {
            elements.insert(0, active);
        }
        if let Some(focused) = focused_summary.clone() {
            insert_unique_element(&mut elements, focused);
        }
        if let Some(pointed) = pointed_summary.clone() {
            insert_unique_element(&mut elements, pointed);
        }

        Ok(ScreenContextSnapshot {
            schema_version: super::SCREEN_CONTEXT_SCHEMA_VERSION,
            platform: "windows".to_string(),
            active_window_title: cached_text(|| unsafe { active_window.CachedName() }),
            active_app_name: None,
            selected_text: selected_text.clone(),
            selected_text_source: selected_text.map(|_| "uia".to_string()),
            captured_at: timestamp_millis(),
            region,
            cursor,
            active_window_bounds: cached_rect(&active_window),
            focused_element: focused_summary,
            pointed_element: pointed_summary,
            visible_windows: self.visible_windows()?,
            elements,
            diagnostics: super::ScreenContextDiagnostics {
                collector: ScreenContextCollector::NativeUia,
                elapsed_ms: started_at.elapsed().as_millis().min(u128::from(u64::MAX)) as u64,
                truncated: elements_truncated,
                ..Default::default()
            },
        })
    }

    fn window_elements(
        &self,
        active_window: &IUIAutomationElement,
        cursor: Option<ScreenContextPoint>,
    ) -> Result<(Vec<UiElementSummary>, bool), String> {
        let condition = unsafe {
            self.automation.ControlViewCondition().map_err(|error| {
                format!("Unable to create the UI Automation control view: {error}")
            })?
        };
        let elements = unsafe {
            active_window
                .FindAllBuildCache(TreeScope_Descendants, &condition, &self.cache_request)
                .map_err(|error| format!("Unable to enumerate UI Automation controls: {error}"))?
        };
        let length = unsafe { elements.Length() }
            .map_err(|error| format!("Unable to count UI Automation controls: {error}"))?;
        let mut summaries = Vec::with_capacity((length.max(0) as usize).min(MAX_CONTEXT_ELEMENTS));

        let is_truncated = length > MAX_CONTEXT_ELEMENTS as i32;
        for index in 0..length.min(MAX_CONTEXT_ELEMENTS as i32) {
            let Ok(element) = (unsafe { elements.GetElement(index) }) else {
                continue;
            };
            if let Some(summary) = self.element_summary(&element, cursor, 1, false) {
                summaries.push(summary);
            }
        }

        Ok((summaries, is_truncated))
    }

    fn visible_windows(&self) -> Result<Vec<VisibleWindowSummary>, String> {
        let root = unsafe {
            self.automation
                .GetRootElement()
                .map_err(|error| format!("Unable to read the UI Automation desktop: {error}"))?
        };
        let condition = unsafe {
            self.automation.ControlViewCondition().map_err(|error| {
                format!("Unable to create the UI Automation window view: {error}")
            })?
        };
        let windows = unsafe {
            root.FindAllBuildCache(TreeScope_Children, &condition, &self.cache_request)
                .map_err(|error| {
                    format!("Unable to enumerate visible UI Automation windows: {error}")
                })?
        };
        let length = unsafe { windows.Length() }
            .map_err(|error| format!("Unable to count visible UI Automation windows: {error}"))?;
        let mut summaries = Vec::new();

        for index in 0..length {
            if summaries.len() >= MAX_VISIBLE_WINDOWS {
                break;
            }
            let Ok(window) = (unsafe { windows.GetElement(index) }) else {
                continue;
            };
            if cached_offscreen(&window) {
                continue;
            }
            let Some(bounds) = cached_rect(&window) else {
                continue;
            };
            let Some(title) = cached_text(|| unsafe { window.CachedName() }) else {
                continue;
            };
            if title.is_empty() || bounds.width == 0 || bounds.height == 0 {
                continue;
            }

            summaries.push(VisibleWindowSummary {
                title,
                app_name: None,
                bounds,
            });
        }

        Ok(summaries)
    }

    fn element_summary(
        &self,
        element: &IUIAutomationElement,
        cursor: Option<ScreenContextPoint>,
        depth: u16,
        force_focused: bool,
    ) -> Option<UiElementSummary> {
        if cached_password(element) {
            return None;
        }

        let bounds = cached_rect(element)?;
        if bounds.width == 0 || bounds.height == 0 {
            return None;
        }

        let focused = force_focused || cached_bool(|| unsafe { element.CachedHasKeyboardFocus() });
        let under_cursor = cursor.is_some_and(|point| point_in_rect(point, &bounds));
        let role = cached_control_type(element)
            .map(control_type_name)
            .unwrap_or("Unknown")
            .to_string();
        let name = cached_text(|| unsafe { element.CachedName() }).unwrap_or_default();
        let value = if focused || under_cursor {
            value_from_element(element)
        } else {
            None
        };
        let selected_text = if focused {
            selected_text_from_element(element)
        } else {
            None
        };

        if name.is_empty()
            && value.is_none()
            && selected_text.is_none()
            && !focused
            && !under_cursor
        {
            return None;
        }

        Some(UiElementSummary {
            role,
            name,
            value,
            selected_text,
            automation_id: cached_text(|| unsafe { element.CachedAutomationId() }),
            class_name: cached_text(|| unsafe { element.CachedClassName() }),
            bounds,
            focused,
            under_cursor,
            is_enabled: cached_bool(|| unsafe { element.CachedIsEnabled() }),
            is_offscreen: cached_offscreen(element),
            depth,
            child_count: 0,
            parent_trail: Vec::new(),
        })
    }
}

fn selected_text_from_element(element: &IUIAutomationElement) -> Option<String> {
    let pattern = unsafe {
        element
            .GetCurrentPatternAs::<IUIAutomationTextPattern>(UIA_TextPatternId)
            .ok()?
    };
    let ranges = unsafe { pattern.GetSelection().ok()? };
    let length = unsafe { ranges.Length().ok()? };
    if length <= 0 {
        return None;
    }
    let range = unsafe { ranges.GetElement(0).ok()? };
    cached_text(|| unsafe { range.GetText(2_400) })
}

fn value_from_element(element: &IUIAutomationElement) -> Option<String> {
    let pattern = unsafe {
        element
            .GetCurrentPatternAs::<IUIAutomationValuePattern>(UIA_ValuePatternId)
            .ok()?
    };
    cached_text(|| unsafe { pattern.CurrentValue() })
}

fn cached_control_type(element: &IUIAutomationElement) -> Option<i32> {
    unsafe {
        element
            .CachedControlType()
            .ok()
            .map(|control_type| control_type.0)
    }
}

fn cached_process_id(element: &IUIAutomationElement) -> Option<i32> {
    unsafe { element.CachedProcessId().ok() }
}

fn cached_password(element: &IUIAutomationElement) -> bool {
    cached_bool(|| unsafe { element.CachedIsPassword() })
}

fn cached_offscreen(element: &IUIAutomationElement) -> bool {
    cached_bool(|| unsafe { element.CachedIsOffscreen() })
}

fn cached_bool<T>(get_value: T) -> bool
where
    T: FnOnce() -> windows::core::Result<windows::core::BOOL>,
{
    get_value().map(|value| value.as_bool()).unwrap_or(false)
}

fn cached_text<T>(get_value: T) -> Option<String>
where
    T: FnOnce() -> windows::core::Result<windows::core::BSTR>,
{
    get_value()
        .ok()
        .map(|value| value.to_string())
        .filter(|value| !value.trim().is_empty())
}

fn cached_rect(element: &IUIAutomationElement) -> Option<UiContextRect> {
    let rectangle: RECT = unsafe { element.CachedBoundingRectangle().ok()? };
    let width = rectangle.right.saturating_sub(rectangle.left);
    let height = rectangle.bottom.saturating_sub(rectangle.top);

    (width > 0 && height > 0).then_some(UiContextRect {
        x: rectangle.left,
        y: rectangle.top,
        width: width as u32,
        height: height as u32,
    })
}

pub fn capture_cursor_position() -> Option<ScreenContextPoint> {
    let mut point = POINT::default();
    unsafe { GetCursorPos(&mut point).ok()? };
    Some(ScreenContextPoint {
        x: point.x,
        y: point.y,
    })
}

fn same_process(candidate: Option<i32>, active_process: Option<i32>) -> bool {
    candidate.is_some() && candidate == active_process
}

fn point_in_rect(point: ScreenContextPoint, rect: &UiContextRect) -> bool {
    let right = rect.x.saturating_add(rect.width as i32);
    let bottom = rect.y.saturating_add(rect.height as i32);
    point.x >= rect.x && point.x <= right && point.y >= rect.y && point.y <= bottom
}

fn insert_unique_element(elements: &mut Vec<UiElementSummary>, candidate: UiElementSummary) {
    let already_present = elements.iter().any(|element| {
        element.automation_id == candidate.automation_id
            && element.role == candidate.role
            && element.bounds.x == candidate.bounds.x
            && element.bounds.y == candidate.bounds.y
            && element.bounds.width == candidate.bounds.width
            && element.bounds.height == candidate.bounds.height
    });

    if !already_present {
        elements.push(candidate);
    }
}

fn control_type_name(control_type: i32) -> &'static str {
    match control_type {
        value if value == UIA_ButtonControlTypeId.0 => "Button",
        value if value == UIA_CheckBoxControlTypeId.0 => "CheckBox",
        value if value == UIA_ComboBoxControlTypeId.0 => "ComboBox",
        value if value == UIA_CustomControlTypeId.0 => "Custom",
        value if value == UIA_DocumentControlTypeId.0 => "Document",
        value if value == UIA_EditControlTypeId.0 => "Edit",
        value if value == UIA_GroupControlTypeId.0 => "Group",
        value if value == UIA_HyperlinkControlTypeId.0 => "Hyperlink",
        value if value == UIA_ListItemControlTypeId.0 => "ListItem",
        value if value == UIA_MenuItemControlTypeId.0 => "MenuItem",
        value if value == UIA_PaneControlTypeId.0 => "Pane",
        value if value == UIA_RadioButtonControlTypeId.0 => "RadioButton",
        value if value == UIA_TabItemControlTypeId.0 => "TabItem",
        value if value == UIA_TextControlTypeId.0 => "Text",
        value if value == UIA_TreeItemControlTypeId.0 => "TreeItem",
        value if value == UIA_WindowControlTypeId.0 => "Window",
        _ => "Unknown",
    }
}

fn timestamp_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::{control_type_name, point_in_rect, UiContextRect};
    use crate::screen_intelligence::ScreenContextPoint;
    use windows::Win32::UI::Accessibility::{UIA_ButtonControlTypeId, UIA_EditControlTypeId};

    #[test]
    fn control_type_names_match_the_prompt_schema() {
        assert_eq!(control_type_name(UIA_ButtonControlTypeId.0), "Button");
        assert_eq!(control_type_name(UIA_EditControlTypeId.0), "Edit");
    }

    #[test]
    fn point_matching_uses_screen_coordinates() {
        let rect = UiContextRect {
            x: -50,
            y: 100,
            width: 200,
            height: 80,
        };

        assert!(point_in_rect(ScreenContextPoint { x: 0, y: 120 }, &rect));
        assert!(!point_in_rect(ScreenContextPoint { x: 200, y: 120 }, &rect));
    }
}
