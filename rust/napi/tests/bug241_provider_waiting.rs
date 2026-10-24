//! Supporting wire parity for spec/features/transient-provider-waiting.feature.

use codelet_rpc_types::StreamChunk;
use serde_json::json;

#[test]
fn waiting_remains_state_only_in_the_napi_wire_codec() -> Result<(), serde_json::Error> {
    for message in [Some("Giving this request a little extra thought"), None] {
        let chunk: StreamChunk =
            serde_json::from_value(json!({"ProviderWaiting":{"message":message}}))?;
        let wire = codelet_napi::stream_chunk_to_json_value(&chunk);
        assert_eq!(wire, json!({"type":"providerWaiting", "message":message}));
    }
    Ok(())
}
