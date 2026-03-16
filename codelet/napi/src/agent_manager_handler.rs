//! AgentManager handler implementation — bridges codelet-tools AgentManagerTool
//! to the SessionManager in codelet-napi.
//!
//! Feature: spec/features/agent-manager-core.feature
//! Feature: spec/features/agent-manager-messaging.feature
//!
//! Creates an `AgentManagerHandler` closure that accesses SessionManager
//! directly to execute spawn/list/get_status/close/message actions.

use codelet_tools::agent_manager::{
    AgentManagerAction, AgentManagerHandler, AgentManagerResult,
    SessionEntry, SessionStatus,
};
use codelet_tools::agent_manager::types::ContextReference;
use std::sync::Arc;
use uuid::Uuid;

use crate::session_manager::{IncomingMessage, SessionManager};

/// Create an AgentManagerHandler closure for a specific session.
///
/// The handler has access to the SessionManager singleton for session
/// creation, destruction, ChainOfCommand queries, status reporting,
/// and inter-session messaging.
///
/// # Arguments
/// * `project` - Project path for spawned sessions
/// * `spawner_provider_id` - Provider ID to inherit (e.g. "anthropic")
/// * `spawner_model_id` - Model ID to inherit (e.g. "claude-sonnet-4")
pub fn create_handler(
    project: String,
    spawner_provider_id: Option<String>,
    spawner_model_id: Option<String>,
) -> AgentManagerHandler {
    Arc::new(move |action: AgentManagerAction, calling_session_id: Uuid| {
        let session_manager = SessionManager::instance();

        match action {
            AgentManagerAction::Spawn { role } => {
                handle_spawn(
                    session_manager,
                    calling_session_id,
                    &project,
                    spawner_provider_id.as_deref(),
                    spawner_model_id.as_deref(),
                    role,
                )
            }
            AgentManagerAction::List => {
                handle_list(session_manager)
            }
            AgentManagerAction::GetStatus { session_id } => {
                handle_get_status(session_manager, &session_id)
            }
            AgentManagerAction::Close { session_id } => {
                handle_close(session_manager, calling_session_id, &session_id)
            }
            AgentManagerAction::Message { session_id, message, context } => {
                handle_message(session_manager, calling_session_id, &session_id, &message, context)
            }
            AgentManagerAction::SetRole { session_id, role } => {
                handle_set_role(session_manager, calling_session_id, session_id.as_deref(), &role)
            }
        }
    })
}

/// Handle the `spawn` action — create a subordinate session
fn handle_spawn(
    session_manager: &SessionManager,
    spawner_id: Uuid,
    project: &str,
    provider_id: Option<&str>,
    model_id: Option<&str>,
    role: Option<String>,
) -> AgentManagerResult {
    let subordinate_id = Uuid::new_v4();
    let name = format!("Agent {}", &subordinate_id.to_string()[..8]);

    // Build the model string in "provider/model" format
    let model_string = match (provider_id, model_id) {
        (Some(provider), Some(model)) => format!("{provider}/{model}"),
        (Some(provider), None) => provider.to_string(),
        (None, Some(model)) => model.to_string(),
        (None, None) => {
            return AgentManagerResult::invalid_parameter(
                "Cannot spawn: no model configured on spawner session",
            );
        }
    };

    // Create the session using tokio runtime (handler is called from sync context)
    let create_result = {
        let rt = match tokio::runtime::Handle::try_current() {
            Ok(h) => h,
            Err(_) => {
                return AgentManagerResult::Error {
                    error: true,
                    code: "internal_error".to_string(),
                    message: "No tokio runtime available for session creation".to_string(),
                };
            }
        };

        // Use block_in_place since we're in an async context already
        tokio::task::block_in_place(|| {
            rt.block_on(session_manager.create_session_with_id(
                &subordinate_id.to_string(),
                &model_string,
                project,
                &name,
            ))
        })
    };

    if let Err(e) = create_result {
        return AgentManagerResult::Error {
            error: true,
            code: "internal_error".to_string(),
            message: format!("Failed to create subordinate session: {e}"),
        };
    }

    // Register the spawner→subordinate relationship in ChainOfCommand
    if let Err(e) = session_manager.add_supervisor(subordinate_id, spawner_id) {
        // Clean up the created session since we can't establish the relationship
        let _ = session_manager.destroy_session(&subordinate_id.to_string());
        return AgentManagerResult::Error {
            error: true,
            code: "internal_error".to_string(),
            message: format!("Failed to register spawner relationship: {e}"),
        };
    }

    // Set the role on the subordinate if provided
    if let Some(role_str) = role {
        if let Ok(session) = session_manager.get_session(&subordinate_id.to_string()) {
            session.set_role(role_str);
        }
    }

    AgentManagerResult::Spawned {
        session_id: subordinate_id.to_string(),
    }
}

/// Handle the `list` action — return all sessions with relationships
fn handle_list(session_manager: &SessionManager) -> AgentManagerResult {
    let sessions_info = session_manager.list_sessions();
    let mut entries = Vec::with_capacity(sessions_info.len());

    for info in &sessions_info {
        let uuid = match Uuid::parse_str(&info.id) {
            Ok(u) => u,
            Err(_) => continue,
        };

        // Get role from the session
        let role = session_manager
            .get_session(&info.id)
            .ok()
            .and_then(|s| s.get_role());

        // Get the spawner (who spawned this session?)
        // A session's spawner is any session that has this session as its subordinate
        let spawner_id = session_manager
            .get_supervisors(uuid)
            .first()
            .map(|id| id.to_string());

        // Get subordinates (sessions spawned by this session)
        let subordinate = session_manager.get_subordinate(uuid);
        let subordinate_ids = subordinate
            .map(|id| vec![id.to_string()])
            .unwrap_or_default();

        entries.push(SessionEntry {
            session_id: info.id.clone(),
            name: info.name.clone(),
            role,
            status: info.status.clone(),
            spawner_id,
            subordinate_ids,
        });
    }

    AgentManagerResult::Listed { sessions: entries }
}

/// Handle the `get_status` action — return detailed status for a session
fn handle_get_status(
    session_manager: &SessionManager,
    session_id: &str,
) -> AgentManagerResult {
    let session = match session_manager.get_session(session_id) {
        Ok(s) => s,
        Err(_) => return AgentManagerResult::session_not_found(session_id),
    };

    let uuid = match Uuid::parse_str(session_id) {
        Ok(u) => u,
        Err(_) => return AgentManagerResult::session_not_found(session_id),
    };

    let role = session.get_role();
    let status = session.get_status().as_str().to_string();
    let model = session.get_model_id();

    // Get spawner (supervisors of this session)
    let spawner_id = session_manager
        .get_supervisors(uuid)
        .first()
        .map(|id| id.to_string());

    // Get subordinates
    let subordinate = session_manager.get_subordinate(uuid);
    let subordinate_ids = subordinate
        .map(|id| vec![id.to_string()])
        .unwrap_or_default();

    // Count pending incoming messages
    let pending_messages = session.pending_incoming_message_count();

    AgentManagerResult::Status(SessionStatus {
        session_id: session_id.to_string(),
        role,
        status,
        model,
        spawner_id,
        subordinate_ids,
        pending_messages,
    })
}

/// Handle the `close` action — terminate a subordinate session
fn handle_close(
    session_manager: &SessionManager,
    calling_session_id: Uuid,
    target_session_id: &str,
) -> AgentManagerResult {
    let target_uuid = match Uuid::parse_str(target_session_id) {
        Ok(u) => u,
        Err(_) => return AgentManagerResult::session_not_found(target_session_id),
    };

    // Verify the session exists
    if session_manager.get_session(target_session_id).is_err() {
        return AgentManagerResult::session_not_found(target_session_id);
    }

    // Permission check: only the spawner can close
    // The spawner is registered as a supervisor of the subordinate
    let supervisors = session_manager.get_supervisors(target_uuid);
    if !supervisors.contains(&calling_session_id) {
        return AgentManagerResult::permission_denied(
            "Only the spawner (supervisor) can close a subordinate session",
        );
    }

    // Destroy the session
    match session_manager.destroy_session(target_session_id) {
        Ok(()) => AgentManagerResult::Closed {
            closed: true,
            session_id: target_session_id.to_string(),
        },
        Err(e) => AgentManagerResult::Error {
            error: true,
            code: "internal_error".to_string(),
            message: format!("Failed to close session: {e}"),
        },
    }
}

/// Handle the `set_role` action — set or clear a role on a session (AMGR-012)
///
/// Sets the role (system prompt overlay) on a target session. If no session_id
/// is provided, defaults to the caller's own session. Empty role string clears it.
fn handle_set_role(
    session_manager: &SessionManager,
    calling_session_id: Uuid,
    session_id: Option<&str>,
    role: &str,
) -> AgentManagerResult {
    // Resolve target session ID — default to caller's own session
    let target_id = session_id
        .map(|s| s.to_string())
        .unwrap_or_else(|| calling_session_id.to_string());

    // Get the session
    let session = match session_manager.get_session(&target_id) {
        Ok(s) => s,
        Err(_) => return AgentManagerResult::session_not_found(&target_id),
    };

    // Set or clear the role
    if role.is_empty() {
        session.clear_role();
        AgentManagerResult::RoleSet {
            session_id: target_id,
            role: None,
        }
    } else {
        session.set_role(role.to_string());
        AgentManagerResult::RoleSet {
            session_id: target_id,
            role: Some(role.to_string()),
        }
    }
}

/// Handle the `message` action — send a message to any session (AMGR-010 + AMGR-011)
///
/// Any session can send a message to any other session by ID. No access control
/// on sending — supervisor→subordinate, subordinate→supervisor, and peer-to-peer
/// are all allowed. Self-messaging is also allowed.
///
/// When context references are provided (AMGR-011), they are resolved at send time
/// using the persistence layer. Resolved content is appended after the sender's
/// message text as XML-style <quoted-context> blocks.
///
/// Messages are delivered through the target session's existing incoming_message
/// channel (mpsc, capacity 16). If the channel is full, returns delivery_failed.
fn handle_message(
    session_manager: &SessionManager,
    calling_session_id: Uuid,
    target_session_id: &str,
    message: &str,
    context: Option<Vec<ContextReference>>,
) -> AgentManagerResult {
    // Look up the target session
    let target_session = match session_manager.get_session(target_session_id) {
        Ok(s) => s,
        Err(_) => return AgentManagerResult::session_not_found(target_session_id),
    };

    // Get the sender's role from the calling session (empty string if no role)
    let sender_role = session_manager
        .get_session(&calling_session_id.to_string())
        .ok()
        .and_then(|s| s.get_role())
        .unwrap_or_default();

    // Resolve context references if provided (AMGR-011)
    let (final_message, context_info) = match &context {
        Some(refs) if !refs.is_empty() => {
            let (resolved_text, resolved_count) = resolve_context(refs);
            let full_message = format!("{message}\n\n{resolved_text}");
            (full_message, Some(resolved_count))
        }
        _ => (message.to_string(), None),
    };

    // Construct the incoming message
    let incoming = IncomingMessage {
        source_session_id: calling_session_id.to_string(),
        role_name: sender_role,
        message: final_message,
        images: None,
    };

    // Deliver via the target's incoming_message channel (non-blocking try_send)
    match target_session.receive_incoming_message(incoming) {
        Ok(()) => {
            match context_info {
                Some(resolved_count) => AgentManagerResult::MessageDeliveredWithContext {
                    delivered: true,
                    session_id: target_session_id.to_string(),
                    context_resolved: resolved_count,
                },
                None => AgentManagerResult::MessageDelivered {
                    delivered: true,
                    session_id: target_session_id.to_string(),
                },
            }
        }
        Err(_) => AgentManagerResult::delivery_failed(
            &format!("Incoming message channel full for session {target_session_id}"),
        ),
    }
}

/// Resolve context references into XML-style quoted-context blocks (AMGR-011)
///
/// Each reference is resolved independently:
/// - Turns: fetch specific turn indices from session history
/// - TurnRange: fetch a contiguous range of turns
/// - Query: search session history using ripgrep regex matching
///
/// Returns (resolved_text, success_count) where success_count is the number
/// of references that resolved without degradation.
fn resolve_context(refs: &[ContextReference]) -> (String, usize) {
    let mut blocks = Vec::new();
    let mut success_count = 0;

    for reference in refs {
        let (block, resolved) = resolve_single_context(reference);
        blocks.push(block);
        if resolved {
            success_count += 1;
        }
    }

    let resolved_text = format!("<quoted-context>\n{}\n</quoted-context>", blocks.join("\n"));
    (resolved_text, success_count)
}

/// Resolve a single context reference into a <from> block
///
/// Returns (xml_block, was_successful) where was_successful indicates
/// whether the reference resolved without degradation.
fn resolve_single_context(reference: &ContextReference) -> (String, bool) {
    match reference {
        ContextReference::Turns { session_id, turns } => {
            resolve_turns_context(session_id, turns)
        }
        ContextReference::TurnRange { session_id, start_turn, end_turn } => {
            let turn_indices: Vec<usize> = (*start_turn..=*end_turn).collect();
            resolve_turns_context(session_id, &turn_indices)
        }
        ContextReference::Query { session_id, query } => {
            resolve_query_context(session_id, query)
        }
    }
}

/// Resolve specific turn indices from a session's history
fn resolve_turns_context(session_id: &str, turns: &[usize]) -> (String, bool) {
    use crate::persistence;

    // Load the session
    let session_uuid = match Uuid::parse_str(session_id) {
        Ok(u) => u,
        Err(_) => {
            return (
                format!("<from session=\"{session_id}\">⚠ Session {session_id} not found</from>"),
                false,
            );
        }
    };

    let session = match persistence::load_session(session_uuid) {
        Ok(s) => s,
        Err(_) => {
            return (
                format!("<from session=\"{session_id}\">⚠ Session {session_id} not found</from>"),
                false,
            );
        }
    };

    // Get all messages
    let messages = match persistence::get_session_messages_full(&session) {
        Ok(m) => m,
        Err(_) => {
            return (
                format!("<from session=\"{session_id}\">⚠ Session {session_id} not found</from>"),
                false,
            );
        }
    };

    // Collect valid turns
    let mut lines = Vec::new();
    let mut valid_turns = Vec::new();
    for &idx in turns {
        if idx < messages.len() {
            let msg = &messages[idx];
            let content = resolve_stored_message_content(msg);
            // Truncate very long messages in context
            let truncated = if content.len() > 2000 {
                format!("{}... [truncated]", &content[..2000])
            } else {
                content
            };
            lines.push(format!("[{idx}] {}: {truncated}", msg.role));
            valid_turns.push(idx);
        }
    }

    if lines.is_empty() {
        return (
            format!("<from session=\"{session_id}\">⚠ No valid turns found</from>"),
            false,
        );
    }

    let turns_label = format_turns_label(&valid_turns);
    let block = format!(
        "<from session=\"{session_id}\" turns=\"{turns_label}\">\n{}\n</from>",
        lines.join("\n")
    );
    (block, true)
}

/// Resolve a search query against a session's history
fn resolve_query_context(session_id: &str, query: &str) -> (String, bool) {
    use crate::persistence;
    use crate::session_search_handler::{build_ripgrep_matcher, ripgrep_is_match};

    // Build the regex matcher
    let matcher = match build_ripgrep_matcher(query) {
        Ok(m) => m,
        Err(_) => {
            return (
                format!(
                    "<from session=\"{session_id}\" query=\"{query}\">⚠ Invalid query pattern \"{query}\"</from>"
                ),
                false,
            );
        }
    };

    // Load the session
    let session_uuid = match Uuid::parse_str(session_id) {
        Ok(u) => u,
        Err(_) => {
            return (
                format!("<from session=\"{session_id}\">⚠ Session {session_id} not found</from>"),
                false,
            );
        }
    };

    let session = match persistence::load_session(session_uuid) {
        Ok(s) => s,
        Err(_) => {
            return (
                format!("<from session=\"{session_id}\">⚠ Session {session_id} not found</from>"),
                false,
            );
        }
    };

    // Get all messages
    let messages = match persistence::get_session_messages_full(&session) {
        Ok(m) => m,
        Err(_) => {
            return (
                format!("<from session=\"{session_id}\">⚠ Session {session_id} not found</from>"),
                false,
            );
        }
    };

    // Find matching turns
    let mut lines = Vec::new();
    let mut matched_turns = Vec::new();
    for (idx, msg) in messages.iter().enumerate() {
        let content = resolve_stored_message_content(msg);
        if ripgrep_is_match(&matcher, &content) {
            let truncated = if content.len() > 2000 {
                format!("{}... [truncated]", &content[..2000])
            } else {
                content
            };
            lines.push(format!("[{idx}] {}: {truncated}", msg.role));
            matched_turns.push(idx);
        }
    }

    if lines.is_empty() {
        return (
            format!(
                "<from session=\"{session_id}\" query=\"{query}\">⚠ No matches for query \"{query}\"</from>"
            ),
            false,
        );
    }

    let turns_label = format_turns_label(&matched_turns);
    let block = format!(
        "<from session=\"{session_id}\" turns=\"{turns_label}\" query=\"{query}\">\n{}\n</from>",
        lines.join("\n")
    );
    (block, true)
}

/// Resolve a StoredMessage's content, handling blob references
///
/// Same logic as session_search_handler::resolve_message_content but
/// kept as a separate function to avoid making that private function public.
fn resolve_stored_message_content(msg: &crate::persistence::StoredMessage) -> String {
    use crate::persistence;
    use crate::persistence::{extract_blob_hash, is_blob_reference};

    // Check if the content itself is a blob reference
    if is_blob_reference(&msg.content) {
        if let Some(hash) = extract_blob_hash(&msg.content) {
            if let Ok(bytes) = persistence::get_blob(hash) {
                return String::from_utf8_lossy(&bytes).to_string();
            }
        }
    }

    // Check for additional blob refs
    if !msg.blob_refs.is_empty() {
        let mut parts = vec![msg.content.clone()];
        for blob_ref in &msg.blob_refs {
            if let Ok(bytes) = persistence::get_blob(blob_ref) {
                parts.push(String::from_utf8_lossy(&bytes).to_string());
            }
        }
        return parts.join("\n");
    }

    msg.content.clone()
}

/// Format turn indices as a compact label (e.g., "1-3" or "1,3,5")
fn format_turns_label(turns: &[usize]) -> String {
    if turns.is_empty() {
        return String::new();
    }
    if turns.len() == 1 {
        return turns[0].to_string();
    }

    // Check if contiguous
    let min = turns[0];
    let max = turns[turns.len() - 1];
    let is_contiguous = turns.len() == (max - min + 1)
        && turns.windows(2).all(|w| w[1] == w[0] + 1);

    if is_contiguous {
        format!("{min}-{max}")
    } else {
        turns.iter().map(|t| t.to_string()).collect::<Vec<_>>().join(",")
    }
}
