//! TUI-111: canonical work-unit ingress sanitization.
//!
//! Sanitizes the user-visible free-text fields of a [`WorkUnitInfo`] on
//! data ingress. Shared by [`BoardStore::replace_work_units`] (board
//! cards, details strip, search dialog) and
//! `AgentViewStore::sync_work_unit_contexts` (SessionHeader WU chip) so
//! a single definition names the field set. `work_type` / `status` are
//! closed vocabularies and are skipped.
//!
//! Feature: spec/features/sanitize-all-tui-output-at-ingress.feature

use codelet_rpc_types::WorkUnitInfo;

use crate::terminal::sanitize::sanitize_for_terminal;

/// Sanitize the user-visible free-text fields of a [`WorkUnitInfo`] on
/// ingress (id, title, description, epic, attachment paths).
pub fn sanitize_work_unit(unit: &mut WorkUnitInfo) {
    unit.id = sanitize_for_terminal(&unit.id);
    unit.title = sanitize_for_terminal(&unit.title);
    unit.description = unit.description.as_ref().map(|d| sanitize_for_terminal(d));
    unit.epic = unit.epic.as_ref().map(|e| sanitize_for_terminal(e));
    unit.attachments = unit
        .attachments
        .iter()
        .map(|a| sanitize_for_terminal(a))
        .collect();
}
