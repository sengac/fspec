//! Feature: spec/features/provider-facade-parity.feature

use codelet_core::persistence::{
    append_message_with_metadata, create_session_with_provider, get_session_message_envelopes,
    reset_stores_for_tests,
};
use codelet_test_helpers::setup_temp_directory;
use serde_json::{json, Value};

#[test]
#[serial_test::serial]
fn unknown_provider_state_survives_persistence_and_blob_rehydration(
) -> Result<(), Box<dyn std::error::Error>> {
    let directory = setup_temp_directory()?;
    codelet_common::set_data_directory(directory.path().to_path_buf())?;
    reset_stores_for_tests();
    let mut manifest =
        create_session_with_provider("extension-test", directory.path(), "independent")?;
    let session_id = manifest.id;
    for state in [json!({"opaque": [1, "secret"]}), Value::Null] {
        append_message_with_metadata(
            &mut manifest,
            "assistant",
            "",
            [
                ("provider".into(), json!("independent")),
                ("message".into(), json!({"role":"assistant","content":[]})),
                ("independentContinuation".into(), state),
                ("uuid".into(), json!("must-not-override")),
            ]
            .into_iter()
            .collect(),
        )?;
    }
    let envelopes = get_session_message_envelopes(session_id)?;
    let first: Value = serde_json::from_str(&envelopes[0])?;
    let last: Value = serde_json::from_str(&envelopes[1])?;
    assert_eq!(
        first["independentContinuation"],
        json!({"opaque": [1, "secret"]})
    );
    assert_eq!(last.get("independentContinuation"), Some(&Value::Null));
    assert_ne!(first["uuid"], "must-not-override");
    assert_eq!(first["message"]["content"], json!([]));
    Ok(())
}
