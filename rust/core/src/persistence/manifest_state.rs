/// Update session token usage (ADDS to existing).
///
/// TOKEN-003: `reasoning_tokens` is STORED (not added) — the caller passes
/// the session-cumulative 🧠 value each turn, so adding would double-count.
/// Input/output remain per-turn deltas that add to `cumulative_billed_*`.
pub fn update_session_tokens(
    session: &mut SessionManifest,
    input: u64,
    output: u64,
    cache_read: u64,
    cache_create: u64,
    reasoning_tokens: u64,
) -> Result<(), String> {
    super::manifest_updates::update_tokens(
        session,
        input,
        output,
        cache_read,
        cache_create,
        reasoning_tokens,
    )
}

/// Set session token usage (REPLACES existing — restore scenario).
///
/// Takes the full [`TokenUsage`] so the /resume restore path can re-seed
/// every field (including the session-cumulative reasoning value for the
/// SessionHeader 🧠 counter) in one call.
pub fn set_session_tokens(session: &mut SessionManifest, usage: &TokenUsage) -> Result<(), String> {
    super::manifest_updates::set_tokens(session, usage)
}

/// Set compaction state for a session.
pub fn set_compaction_state(
    session: &mut SessionManifest,
    summary: String,
    compacted_before_index: usize,
) -> Result<(), String> {
    super::manifest_updates::set_compaction(session, summary, compacted_before_index)
}

/// Clear compaction state for a session.
pub fn clear_compaction_state(session: &mut SessionManifest) -> Result<(), String> {
    super::manifest_updates::clear_compaction(session)
}

/// Get session lineage information.
pub fn get_session_lineage(session: &SessionManifest) -> SessionLineage {
    SessionLineage {
        session_id: session.id,
        forked_from: session.forked_from.clone(),
        merged_from: session.merged_from.clone(),
    }
}

/// List all sessions for a specific project (owned values).
pub fn list_sessions_for_project(project: &Path) -> Result<Vec<SessionManifest>, String> {
    init_session_store()?;
    let store = SESSION_STORE.lock().map_err(|e| e.to_string())?;
    Ok(store
        .as_ref()
        .ok_or("Session store not initialized")?
        .list_for_project(project)
        .into_iter()
        .cloned()
        .collect())
}

/// List all sessions across all projects.
pub fn list_all_sessions() -> Result<Vec<SessionManifest>, String> {
    init_session_store()?;
    let store = SESSION_STORE.lock().map_err(|e| e.to_string())?;
    Ok(store
        .as_ref()
        .ok_or("Session store not initialized")?
        .list_all()
        .into_iter()
        .cloned()
        .collect())
}
