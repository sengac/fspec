use codelet_providers::codex::{history::CodexHistory, memory_policy::NativeMemoryUnsupported};
use rig::completion::CompletionError;
use serde_json::json;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn native_compaction_cannot_replace_committed_history() -> TestResult {
    let history = CodexHistory::default();
    let request = json!({"model":"test","input":[{"role":"user","content":"old"}]});
    history.commit(&request, &[])?;
    let prior = history.snapshot();
    for family in ["compaction", "compaction_summary", "context_compaction"] {
        let output = json!({"type":family,"encrypted_content":"opaque+/=秘密"});
        let error = history
            .commit(&request, &[output])
            .err()
            .ok_or("native commit accepted")?;
        assert!(matches!(error, CompletionError::RequestError(ref source)
            if source.downcast_ref::<NativeMemoryUnsupported>().is_some()));
        assert_eq!(history.snapshot(), prior);
        assert!(!format!("{error:?}").contains("opaque"));
    }
    Ok(())
}

#[test]
fn native_checkpoint_cannot_be_replayed_after_reload() -> TestResult {
    let history = CodexHistory::default();
    let checkpoint = json!({"version":1,"model":"test","items":[{
        "type":"compaction","encrypted_content":"opaque+/=秘密"}],"compacted_users":[]});
    history.restore(Some(serde_json::from_value(checkpoint)?));
    let prior = history.snapshot();
    let mut next = json!({"model":"test","input":[{"role":"user","content":"next"}]});
    let unchanged = next.clone();
    assert!(history.prepare(&mut next).is_err());
    assert_eq!(next, unchanged);
    assert_eq!(history.snapshot(), prior);
    assert!(!format!("{prior:?}").contains("opaque"));
    Ok(())
}
