//! PROV-120 — startup-model restore helper extracted from `bootstrap.rs`.
//!
//! Behavior feature: spec/features/startup-model-initialization-first-available.feature
//!
//! Reads the persisted `tui.lastUsedModel` (legacy `default-model.json`
//! back-compat), lists providers, resolves restore-persisted →
//! first-available, and commits the result via `set_default_model`.
//! Mirroring TS, the commit is UNCONDITIONAL (idempotent; corrects a
//! stale value) and there is NO hardcoded anthropic/claude fallback — a
//! `None` resolution leaves the default unset so the PROV-101 decline
//! path is preserved. Best-effort: any failure is surfaced to tracing
//! and never blocks bootstrap.
//!
//! Extracted from `app/bootstrap.rs` to keep that file under the
//! 300-LoC source-shape ceiling (mirrors bootstrap_git_state.rs /
//! bootstrap_session_removed.rs).

use tracing::debug;

use super::state::App;

pub(crate) async fn initialize_startup_model(app: &mut App) {
    let persisted = codelet_sessions::last_used_model_persistence::load_persisted_model_string();
    let sections = match app.backend.list_providers().await {
        Ok(sections) => sections,
        Err(err) => {
            debug!("bootstrap: backend.list_providers() failed: {err}");
            return;
        }
    };
    if let Some(resolved) = codelet_sessions::startup_model_resolution::resolve_startup_model(
        &sections,
        persisted.as_deref(),
    ) {
        if let Err(err) = app.backend.set_default_model(resolved.model_string).await {
            debug!("bootstrap: backend.set_default_model() failed: {err}");
        }
    }
}
