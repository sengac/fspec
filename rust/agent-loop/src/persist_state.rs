use codelet_core::persistence::{load_session, update_session_tokens};

pub fn persist_token_state(
    session_id: &uuid::Uuid,
    input_tokens: u32,
    output_tokens: u32,
    reasoning_tokens: u32,
) -> Result<(), String> {
    let mut manifest = load_session(*session_id)?;
    update_session_tokens(
        &mut manifest,
        input_tokens as u64,
        output_tokens as u64,
        0,
        0,
        reasoning_tokens as u64,
    )
}

pub fn persist_pending_annotations(
    session_id: &uuid::Uuid,
    session: &mut codelet_cli::session::Session,
) {
    if session.annotations.is_empty() {
        return;
    }
    use codelet_cli::session::system_reminders::is_system_reminder;
    let reminder_count = session
        .messages
        .iter()
        .filter(|message| is_system_reminder(message))
        .count();
    let messages = load_session(*session_id)
        .and_then(|manifest| codelet_core::persistence::get_session_messages_full(&manifest));
    let messages = match messages {
        Ok(messages) => messages,
        Err(error) => {
            tracing::warn!(%error, "persist_pending_annotations could not load originals");
            session.annotations.clear();
            return;
        }
    };
    for (index, annotations) in session.annotations.drain() {
        let Some(index) = index.checked_sub(reminder_count) else {
            continue;
        };
        let Some(message) = messages.get(index) else {
            continue;
        };
        let annotations = match serde_json::to_value(annotations) {
            Ok(annotations) => annotations,
            Err(error) => {
                tracing::warn!(%error, "annotation encoding failed");
                continue;
            }
        };
        let mut entries = std::collections::HashMap::new();
        entries.insert("annotations".into(), annotations);
        if let Err(error) = codelet_core::persistence::update_message_metadata(message.id, entries)
        {
            tracing::warn!(%error, "annotation persistence failed");
        }
    }
}
