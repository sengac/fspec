use codelet_agent_loop::persistence_retry::RecoveryRecord;
use codelet_core::persistence::transcript::{JournalEvent, JournalIdentity, JournalRecord};
use codelet_core::persistence::{create_session, reset_stores_for_tests};
use codelet_test_helpers::setup_temp_directory;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use uuid::Uuid;

pub type TestResult = Result<(), Box<dyn std::error::Error>>;

pub fn fixture() -> Result<(tempfile::TempDir, Uuid), Box<dyn std::error::Error>> {
    let directory = setup_temp_directory()?;
    codelet_common::set_data_directory(directory.path().into())?;
    reset_stores_for_tests();
    let session = create_session("recovery retirement", directory.path())?;
    Ok((directory, session.id))
}

pub fn recovery(session_id: Uuid) -> RecoveryRecord {
    let event = JournalRecord::new(
        JournalIdentity {
            session_id,
            request_id: Uuid::new_v4(),
            attempt_id: Uuid::new_v4(),
            message_id: Uuid::new_v4(),
        },
        0,
        JournalEvent::Text {
            text: "authentic pending prefix".into(),
        },
    );
    RecoveryRecord {
        version: 1,
        id: event.id,
        role: "transcript".into(),
        content: String::new(),
        metadata: HashMap::from([("_transcript".into(), serde_json::json!(event))]),
    }
}

pub fn manifest(root: &Path, session: Uuid) -> PathBuf {
    root.join("sessions").join(format!("{session}.json"))
}
