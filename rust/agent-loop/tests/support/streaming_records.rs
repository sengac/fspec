use codelet_core::persistence::transcript::{JournalEvent, JournalIdentity, JournalRecord};
use uuid::Uuid;

pub fn records(count: usize) -> Vec<JournalRecord> {
    let identity = JournalIdentity {
        session_id: Uuid::new_v4(),
        request_id: Uuid::new_v4(),
        attempt_id: Uuid::new_v4(),
        message_id: Uuid::new_v4(),
    };
    (0..count)
        .map(|sequence| {
            JournalRecord::new(
                identity.clone(),
                sequence as u64,
                JournalEvent::Text {
                    text: format!("fixture-{sequence:08} α"),
                },
            )
        })
        .collect()
}
