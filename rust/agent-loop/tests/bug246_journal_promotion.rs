#![doc = "Feature: spec/features/persistence-journal-promotion.feature"]

use codelet_agent_loop::persist::{
    persist_assistant_message_identified, persist_tool_result_identified, retry_pending_persistence,
};
use codelet_agent_loop::persistence_retry::{PersistenceSpool, RecoveryRecord};
use codelet_core::persistence::transcript::{
    JournalEvent, JournalIdentity, JournalRecord, JournalStore,
};
use codelet_core::persistence::AssistantContent;
use serde_json::json;
use std::collections::HashMap;
use uuid::Uuid;

#[path = "support/bug246.rs"]
mod support;
use support::{fixture, read, TestResult};

fn journal_record(session_id: Uuid) -> JournalRecord {
    JournalRecord::new(
        JournalIdentity {
            session_id,
            request_id: Uuid::new_v4(),
            attempt_id: Uuid::new_v4(),
            message_id: Uuid::new_v4(),
        },
        0,
        JournalEvent::Text {
            text: "authentic prefix".into(),
        },
    )
}

fn recovery(record: &JournalRecord) -> RecoveryRecord {
    RecoveryRecord {
        version: 1,
        id: record.id,
        role: "transcript".into(),
        content: String::new(),
        metadata: HashMap::from([("_transcript".into(), json!(record))]),
    }
}

#[tokio::test]
#[serial_test::serial]
async fn journal_identities_remain_physical_assistant_and_result_identities() -> TestResult {
    // @step Given an acknowledged journal invocation and its captured result identities
    let (directory, session, _) = fixture()?;
    std::fs::remove_dir(directory.path().join("messages/messages.jsonl"))?;
    let assistant = Uuid::new_v4();
    let result = Uuid::new_v4();
    let journal = JournalStore::configured(session.id)?;
    let mut identity = JournalIdentity {
        session_id: session.id,
        request_id: Uuid::new_v4(),
        attempt_id: Uuid::new_v4(),
        message_id: assistant,
    };
    journal.append(&JournalRecord::new(
        identity.clone(),
        0,
        JournalEvent::ToolRequested {
            id: "call".into(),
            call_id: Some("native".into()),
            name: "inspect".into(),
            input: json!({}),
        },
    ))?;
    journal.append(&JournalRecord::new(
        identity.clone(),
        1,
        JournalEvent::ToolStarted { id: "call".into() },
    ))?;
    identity.message_id = result;
    journal.append(&JournalRecord::new(
        identity,
        2,
        JournalEvent::ToolCompleted {
            id: "call".into(),
            content: "authentic result".into(),
            is_error: false,
        },
    ))?;
    // @step When assistant and result originals are promoted using those identities
    persist_assistant_message_identified(
        &session.id,
        "codex",
        assistant,
        vec![AssistantContent::ToolUse {
            id: "call".into(),
            call_id: Some("native".into()),
            name: "inspect".into(),
            input: json!({}),
        }],
        Some("tool_use".into()),
        None,
    )?;
    persist_tool_result_identified(
        &session.id,
        result,
        "call",
        Some("native"),
        "authentic result",
        false,
    )?;
    retry_pending_persistence(&session.id)?;
    // @step Then cold reopen contains each original once with its journal identity
    let originals = read(session.id)?;
    assert_eq!(originals.len(), 2);
    assert_eq!(originals[0]["uuid"], assistant.to_string());
    assert_eq!(originals[1]["uuid"], result.to_string());
    assert_eq!(originals[1]["message"]["content"][0]["call_id"], "native");
    assert!(journal
        .project(&[assistant, result].into())?
        .records
        .is_empty());
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn transcript_recovery_never_creates_a_conversation_original() -> TestResult {
    // @step Given a retained transcript event for a real session
    let (_directory, session, _) = fixture()?;
    let record = journal_record(session.id);
    PersistenceSpool::open(session.id)?.enqueue(&recovery(&record))?;
    // @step When persistence recovery promotes the event
    retry_pending_persistence(&session.id)?;
    retry_pending_persistence(&session.id)?;
    // @step Then the journal contains the event and the conversation archive remains empty
    assert_eq!(JournalStore::configured(session.id)?.read()?, vec![record]);
    assert!(read(session.id)?.is_empty());
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn transcript_recovery_rejects_mismatched_ownership() -> TestResult {
    // @step Given a transcript event with a mismatched session or record identity
    for mismatch_session in [true, false] {
        let (directory, session, _) = fixture()?;
        std::fs::remove_dir(directory.path().join("messages/messages.jsonl"))?;
        let mut record = journal_record(session.id);
        if mismatch_session {
            record.identity.session_id = Uuid::new_v4();
        }
        let mut pending = recovery(&record);
        if !mismatch_session {
            pending.id = Uuid::new_v4();
        }
        let spool = PersistenceSpool::open(session.id)?;
        spool.enqueue(&pending)?;
        // @step When persistence recovery attempts to promote the event
        let result = retry_pending_persistence(&session.id);
        // @step Then recovery fails without writing the event or a conversation original
        assert!(result.is_err());
        assert!(JournalStore::configured(session.id)?.read()?.is_empty());
        assert!(read(session.id)?.is_empty());
        assert_eq!(spool.pending_count()?, 1);
    }
    Ok(())
}
