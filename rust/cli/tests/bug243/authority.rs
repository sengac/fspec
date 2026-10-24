//! Feature: spec/features/provider-state-authority.feature

use super::fixtures::*;

#[test]
#[serial_test::serial]
fn select_authoritative_provider_state_independently_from_original_metadata() -> TestResult {
    // @step Given original dialogue carrying an older native checkpoint and a newer separate checkpoint
    let directory = setup()?;
    let (mut manifest, _) = dialogue(directory.path())?;
    let alias = CodexHistory::default();
    publish(&alias, "original-checkpoint-alias")?;
    let mut original = json!({"uuid":Uuid::new_v4(),"provider":"codex",
        "message":{"role":"assistant","content":[{"type":"text","text":"authentic dialogue"}]},
        "codexContinuation":serde_json::to_value(alias.snapshot()?)?});
    append_message_with_metadata(
        &mut manifest,
        "assistant",
        "",
        serde_json::from_value(original.clone())?,
    )?;
    let mut live = session();
    live.bind_provider_persistence(manifest.id)?;
    publish(&history(&live)?, "authoritative-separate-state")?;
    // @step When cold replay restores the committed provider state
    let mut cold = session();
    cold.restore_provider_state(&get_session_message_envelopes(manifest.id)?)?;
    // @step Then the separate checkpoint is authoritative and original dialogue remains unchanged
    assert_eq!(history(&cold)?.snapshot()?, history(&live)?.snapshot()?);
    let mut selected = request(manifest.id, Some(3));
    selected.limit = Some(1);
    assert_eq!(
        originals::retrieve_originals(selected)?.records[0].envelope,
        original
    );
    // @step And original metadata cannot replace or invalidate the committed native checkpoint
    original["codexContinuation"] = json!({"version":999,"invalid":"not native state"});
    append_message_with_metadata(
        &mut manifest,
        "assistant",
        "",
        serde_json::from_value(original)?,
    )?;
    let mut again = session();
    again.restore_provider_state(&get_session_message_envelopes(manifest.id)?)?;
    assert_eq!(history(&again)?.snapshot()?, history(&live)?.snapshot()?);
    Ok(())
}
