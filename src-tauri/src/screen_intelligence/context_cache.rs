//! Request-scoped screen-context reuse.
//!
//! Opening Waey starts a best-effort UIA capture before the user submits a
//! message. The resulting snapshot can be reused only for a very short window
//! and only for the same target HWND. This preserves a responsive first prompt
//! without silently falling back to an old desktop state after a failed capture.

use super::{ScreenContextPoint, ScreenContextSnapshot};
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

pub const PREFETCH_FRESHNESS_WINDOW: Duration = Duration::from_millis(750);
const IN_FLIGHT_WAIT_TIMEOUT: Duration = Duration::from_millis(1_350);
const MAX_CURSOR_REUSE_DISTANCE: i32 = 28;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ContextTarget(Option<isize>);

struct CachedSnapshot {
    target: ContextTarget,
    captured_at: Instant,
    cursor: Option<ScreenContextPoint>,
    snapshot: ScreenContextSnapshot,
}

#[derive(Default)]
struct CacheState {
    snapshot: Option<CachedSnapshot>,
    in_flight: Option<ContextTarget>,
}

struct CacheInner {
    state: Mutex<CacheState>,
    completed: Condvar,
}

/// Coordinates the overlay prefetch and the prompt-time capture.
///
/// A single native UIA worker services the application, so allowing concurrent
/// capture requests would turn a useful prefetch into a misleading "busy"
/// failure. This cache serializes the two callers and never returns a snapshot
/// once it is older than [`PREFETCH_FRESHNESS_WINDOW`].
#[derive(Clone)]
pub struct ScreenContextCache {
    inner: Arc<CacheInner>,
}

impl Default for ScreenContextCache {
    fn default() -> Self {
        Self {
            inner: Arc::new(CacheInner {
                state: Mutex::new(CacheState::default()),
                completed: Condvar::new(),
            }),
        }
    }
}

impl ScreenContextCache {
    /// Returns a request-safe cached snapshot for this exact target window.
    pub fn take_fresh(
        &self,
        target_window_handle: Option<isize>,
        current_cursor: Option<&ScreenContextPoint>,
    ) -> Option<ScreenContextSnapshot> {
        let target = ContextTarget(target_window_handle);
        let state = self.inner.state.lock().ok()?;
        let cached = state.snapshot.as_ref()?;

        (cached.target == target
            && cached.captured_at.elapsed() <= PREFETCH_FRESHNESS_WINDOW
            && cursor_is_near(cached.cursor.as_ref(), current_cursor))
        .then(|| cached.snapshot.clone())
    }

    /// Marks a capture as in flight. `false` means another capture must finish
    /// first; callers should wait rather than competing for the UIA worker.
    pub fn try_begin_capture(&self, target_window_handle: Option<isize>) -> Result<bool, String> {
        let target = ContextTarget(target_window_handle);
        let mut state = self
            .inner
            .state
            .lock()
            .map_err(|_| "Screen context cache lock was poisoned.".to_string())?;

        if state.in_flight.is_some() {
            return Ok(false);
        }

        state.in_flight = Some(target);
        Ok(true)
    }

    /// Stores a successful capture or clears a same-target cached result after
    /// an error. Clearing is intentional: the caller must see the fresh-capture
    /// failure instead of receiving a previous screen state.
    pub fn finish_capture(
        &self,
        target_window_handle: Option<isize>,
        result: &Result<Option<ScreenContextSnapshot>, String>,
    ) {
        let target = ContextTarget(target_window_handle);

        if let Ok(mut state) = self.inner.state.lock() {
            if state.in_flight == Some(target) {
                state.in_flight = None;
            }

            match result {
                Ok(Some(snapshot)) => {
                    state.snapshot = Some(CachedSnapshot {
                        target,
                        captured_at: Instant::now(),
                        cursor: snapshot.cursor.clone(),
                        snapshot: snapshot.clone(),
                    });
                }
                Ok(None) | Err(_) => {
                    if state
                        .snapshot
                        .as_ref()
                        .is_some_and(|cached| cached.target == target)
                    {
                        state.snapshot = None;
                    }
                }
            }
        }

        self.inner.completed.notify_all();
    }

    /// Uses a very recent prefetch when permitted; otherwise waits for an
    /// already-running collection or starts a new collection itself.
    pub fn capture_or_reuse<F>(
        &self,
        target_window_handle: Option<isize>,
        current_cursor: Option<ScreenContextPoint>,
        may_reuse_prefetch: bool,
        collector: F,
    ) -> Result<Option<ScreenContextSnapshot>, String>
    where
        F: FnOnce() -> Result<Option<ScreenContextSnapshot>, String>,
    {
        if may_reuse_prefetch {
            if let Some(snapshot) = self.take_fresh(target_window_handle, current_cursor.as_ref()) {
                return Ok(Some(snapshot));
            }
        }

        let mut collector = Some(collector);
        for _ in 0..2 {
            if self.try_begin_capture(target_window_handle)? {
                let result = collector
                    .take()
                    .expect("screen context collector is consumed once")(
                );
                self.finish_capture(target_window_handle, &result);
                return result;
            }

            self.wait_for_in_flight_capture()?;

            if may_reuse_prefetch {
                if let Some(snapshot) =
                    self.take_fresh(target_window_handle, current_cursor.as_ref())
                {
                    return Ok(Some(snapshot));
                }
            }
        }

        Err("UI Automation is still collecting screen context for another request.".to_string())
    }

    fn wait_for_in_flight_capture(&self) -> Result<(), String> {
        let mut state = self
            .inner
            .state
            .lock()
            .map_err(|_| "Screen context cache lock was poisoned.".to_string())?;

        while state.in_flight.is_some() {
            let (next_state, timeout) = self
                .inner
                .completed
                .wait_timeout(state, IN_FLIGHT_WAIT_TIMEOUT)
                .map_err(|_| "Screen context cache lock was poisoned.".to_string())?;
            state = next_state;

            if timeout.timed_out() && state.in_flight.is_some() {
                return Err(
                    "UI Automation is still collecting screen context; try again in a moment."
                        .to_string(),
                );
            }
        }

        Ok(())
    }
}

fn cursor_is_near(
    captured_cursor: Option<&ScreenContextPoint>,
    current_cursor: Option<&ScreenContextPoint>,
) -> bool {
    let (Some(captured), Some(current)) = (captured_cursor, current_cursor) else {
        return captured_cursor.is_none() && current_cursor.is_none();
    };

    captured.x.abs_diff(current.x) <= MAX_CURSOR_REUSE_DISTANCE as u32
        && captured.y.abs_diff(current.y) <= MAX_CURSOR_REUSE_DISTANCE as u32
}

#[cfg(test)]
mod tests {
    use super::ScreenContextCache;

    #[test]
    fn only_one_collection_can_run_at_a_time() {
        let cache = ScreenContextCache::default();

        assert!(cache
            .try_begin_capture(Some(10))
            .expect("start first capture"));
        assert!(!cache
            .try_begin_capture(Some(20))
            .expect("observe in-flight capture"));

        let failure = Err("collector failed".to_string());
        cache.finish_capture(Some(10), &failure);

        assert!(cache
            .try_begin_capture(Some(20))
            .expect("start after completion"));
    }
}
