//! Feature: spec/features/provider-state-durability.feature

use super::fixtures::*;
use codelet_core::persistence::messages::durability::observe;

#[test]
#[serial_test::serial]
fn acknowledge_checkpoint_state_only_after_ordered_durability_barriers() -> TestResult {
    // @step Given a real provider checkpoint writer with an existing native checkpoint
    let directory = setup()?;
    let (manifest, _) = dialogue(directory.path())?;
    let mut live = session();
    live.bind_provider_persistence(manifest.id)?;
    publish(&history(&live)?, "prior")?;
    // @step When checkpoint publication reaches each durable promotion boundary
    let (publication, completed) = observe(None, || publish(&history(&live)?, "ordered"));
    publication?;
    // @step Then data sync precedes atomic replacement and directory acknowledgement
    let boundaries = [
        "provider_state.write",
        "provider_state.sync",
        "provider_state.rename",
        "provider_state.directory",
        "manifest.sync",
        "manifest.rename",
        "manifest.directory",
    ];
    let positions = boundaries
        .iter()
        .map(|boundary| {
            completed
                .iter()
                .position(|event| event == boundary)
                .ok_or_else(|| format!("missing {boundary}: {completed:?}"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    assert!(positions.windows(2).all(|pair| pair[0] < pair[1]));
    // @step And failures before replacement preserve the previous checkpoint
    for boundary in boundaries {
        let path = state_path(directory.path(), manifest.id)?;
        let previous = std::fs::read(&path)?;
        let mut cold = session();
        cold.restore_provider_state(&get_session_message_envelopes(manifest.id)?)?;
        cold.bind_provider_persistence(manifest.id)?;
        let (failed, reached) = observe(Some(boundary), || publish(&history(&cold)?, boundary));
        assert!(failed.is_err(), "{boundary}: {reached:?}");
        if boundary != "manifest.directory" {
            assert_eq!(std::fs::read(&path)?, previous);
        } else {
            // @step And failure after replacement reports promoted state explicitly and retry is idempotent
            let error = failed.err().ok_or("publication failure")?.to_string();
            assert!(error.contains("promoted"), "{error}");
            let promoted = std::fs::read(state_path(directory.path(), manifest.id)?)?;
            let mut retry = session();
            retry.restore_provider_state(&get_session_message_envelopes(manifest.id)?)?;
            retry.bind_provider_persistence(manifest.id)?;
            publish(&history(&retry)?, boundary)?;
            assert_eq!(
                std::fs::read(state_path(directory.path(), manifest.id)?)?,
                promoted
            );
        }
    }
    assert_eq!(load_session(manifest.id)?.messages.len(), 3);
    Ok(())
}
