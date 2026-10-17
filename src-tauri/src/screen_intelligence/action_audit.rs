//! Privacy-preserving audit outcomes for approved desktop-action attempts.
//!
//! Do not add screen labels, model summaries, bounds, or entered text here.
//! Those values can contain personal data and are not needed to diagnose the
//! action pipeline's safety decisions.

use super::DesktopActionKind;
use crate::storage::open_app_database;
use rusqlite::params;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::AppHandle;
use uuid::Uuid;

#[derive(Clone, Copy, Debug)]
pub enum DesktopActionAuditOutcome {
    Applied,
    Blocked,
    Failed,
}

pub fn record_desktop_action_outcome(
    app: &AppHandle,
    action_kind: DesktopActionKind,
    outcome: DesktopActionAuditOutcome,
    reason_code: &'static str,
) -> Result<(), String> {
    let connection = open_app_database(app)?;
    connection
        .execute(
            "insert into desktop_action_audit (id, action_kind, outcome, reason_code, created_at)
             values (?1, ?2, ?3, ?4, ?5)",
            params![
                Uuid::new_v4().to_string(),
                action_kind_name(action_kind),
                outcome_name(outcome),
                reason_code,
                timestamp_seconds(),
            ],
        )
        .map_err(|error| error.to_string())?;
    Ok(())
}

fn action_kind_name(kind: DesktopActionKind) -> &'static str {
    match kind {
        DesktopActionKind::Click => "click",
        DesktopActionKind::SetText => "setText",
        DesktopActionKind::Scroll => "scroll",
    }
}

fn outcome_name(outcome: DesktopActionAuditOutcome) -> &'static str {
    match outcome {
        DesktopActionAuditOutcome::Applied => "applied",
        DesktopActionAuditOutcome::Blocked => "blocked",
        DesktopActionAuditOutcome::Failed => "failed",
    }
}

fn timestamp_seconds() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::{action_kind_name, outcome_name, DesktopActionAuditOutcome};
    use crate::screen_intelligence::DesktopActionKind;

    #[test]
    fn audit_labels_are_fixed_and_do_not_use_user_content() {
        assert_eq!(action_kind_name(DesktopActionKind::SetText), "setText");
        assert_eq!(outcome_name(DesktopActionAuditOutcome::Blocked), "blocked");
    }
}
