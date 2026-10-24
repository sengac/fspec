//! Feature: spec/features/transient-provider-waiting.feature

use codelet_rpc_types::StreamChunk;
use serde_json::json;

#[test]
fn waiting_changes_chrome_without_entering_the_transcript() -> Result<(), serde_json::Error> {
    for message in [Some("Codex buffering"), None] {
        let wire = json!({"ProviderWaiting": {"message": message}});
        let chunk: StreamChunk = serde_json::from_value(wire.clone())?;
        assert_eq!(serde_json::to_value(chunk)?, wire);
    }
    Ok(())
}
