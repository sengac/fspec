#![doc = "Feature: spec/features/committed-tail-recovery-evidence.feature"]

#[path = "bug248/support.rs"]
mod support;

use codelet_core::persistence::{MessageSource, MessageStore, SessionStore};
use serial_test::serial;
use support::{append, fixture, quarantines, rebuild, record, TestResult};

#[test]
#[serial]
fn scenario_missing_committed_reference_evidence_refuses_tail_removal() -> TestResult {
    // @step Given a valid prefix and torn tail with unavailable committed session evidence
    for evidence in ["missing", "malformed", "unreadable"] {
        let (directory, _, prefix) = fixture()?;
        assert!(rebuild(directory.path())?.get(prefix).is_some());
        let sessions = directory.path().join("sessions");
        match evidence {
            "missing" => std::fs::remove_dir_all(&sessions)?,
            "malformed" => std::fs::write(sessions.join("committed.json"), b"{invalid")?,
            "unreadable" => std::fs::create_dir(sessions.join("committed.json"))?,
            _ => return Err("unknown evidence case".into()),
        }
        let mut torn = serde_json::to_vec(&record("uncommitted tail"))?;
        torn.truncate(torn.len() / 2);
        append(directory.path(), &torn)?;
        let path = directory.path().join("messages/messages.jsonl");
        let original = std::fs::read(&path)?;
        // @step When recovery encounters missing malformed or unreadable session evidence
        assert!(MessageStore::new().is_err(), "{evidence} evidence accepted");
        // @step Then recovery fails without modifying history and retains the quarantined tail
        assert_eq!(std::fs::read(path)?, original);
        assert_eq!(quarantines(directory.path())?, vec![torn.as_slice()]);
    }
    Ok(())
}

#[test]
#[serial]
fn scenario_referenced_metadata_versions_refuse_destructive_recovery() -> TestResult {
    // @step Given a valid referenced message followed by its torn metadata replacement
    for encoding in ["whitespace", "escape", "escaped-value"] {
        let (directory, _, id) = fixture()?;
        let mut sessions = SessionStore::new()?;
        let mut session = sessions.create("metadata replacement", directory.path())?;
        session.add_message(id, MessageSource::Native);
        sessions.commit(&session)?;
        // @step When the replacement identity is encoded with valid JSON whitespace or escapes
        let tail = if encoding == "whitespace" {
            format!("{{ \"id\" : \"{id}\", \"metadata\" : {{")
        } else if encoding == "escape" {
            format!("{{\"\\u0069d\":\"{id}\",\"metadata\":{{")
        } else {
            let encoded_id = id
                .to_string()
                .bytes()
                .map(|byte| format!("\\u{byte:04x}"))
                .collect::<String>();
            format!("{{\"id\":\"{encoded_id}\",\"metadata\":{{")
        };
        append(directory.path(), tail.as_bytes())?;
        let path = directory.path().join("messages/messages.jsonl");
        let original = std::fs::read(&path)?;
        // @step Then recovery fails without removing the potentially referenced replacement
        assert!(
            MessageStore::new().is_err(),
            "{encoding} replacement discarded"
        );
        assert_eq!(std::fs::read(path)?, original);
        assert_eq!(quarantines(directory.path())?, vec![tail.as_bytes()]);
    }
    Ok(())
}
