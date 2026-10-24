//! Feature: spec/features/provider-state-original-separation.feature

#[path = "bug243/authority.rs"]
mod authority;
#[path = "bug243/durability.rs"]
mod durability;
#[path = "bug243/fixtures.rs"]
mod fixtures;
#[path = "bug243/legacy.rs"]
mod legacy;
#[path = "bug243/recovery.rs"]
mod recovery;

use fixtures::*;
use rig::tool::Tool;
use std::sync::Arc;

#[tokio::test]
#[serial_test::serial]
async fn persist_real_provider_checkpoints_without_conversation_amplification() -> TestResult {
    // @step Given a real provider persistence binder and structured tool dialogue
    let directory = setup()?;
    let (mut manifest, dialogue) = dialogue(directory.path())?;
    let mut session = session();
    session.bind_provider_persistence(manifest.id)?;
    let history = history(&session)?;
    // @step When repeated native checkpoints are persisted through the binder
    for size in 1..=12 {
        publish(&history, &"native-reasoning".repeat(size * 1024))?;
    }
    // @step Then bounded SessionSearch originals traverses only the unchanged dialogue
    codelet_tools::session_search::set_session_search_handler(
        manifest.id,
        Some(Arc::new(|action, _| match action {
            codelet_tools::session_search::types::SessionSearchAction::Originals { request } => {
                codelet_core::persistence::originals::retrieve_originals(request).into()
            }
            _ => codelet_tools::session_search::SearchResult::Error {
                message: "originals only".into(),
            },
        })),
    );
    let tool = codelet_tools::session_search::SessionSearchTool::new(manifest.id);
    let mut cursor = None;
    let mut found = Vec::new();
    loop {
        let result: Value = serde_json::from_str(
            &tool
                .call(codelet_tools::session_search::types::SessionSearchArgs {
                    action: codelet_tools::session_search::types::SessionSearchAction::Originals {
                        request: request(manifest.id, cursor),
                    },
                })
                .await?,
        )?;
        assert_eq!(result["result_type"], "originals", "{result}");
        assert!(serde_json::to_vec(&result)?.len() <= 61440);
        found.extend(
            result["records"]
                .as_array()
                .ok_or("records")?
                .iter()
                .map(|record| record["envelope"].clone()),
        );
        cursor = result["next_turn"].as_u64().map(|value| value as usize);
        if cursor.is_none() {
            break;
        }
    }
    codelet_tools::session_search::set_session_search_handler(manifest.id, None);
    assert_eq!(found, dialogue);
    // @step And retained state contains one latest checkpoint per provider generation
    manifest = load_session(manifest.id)?;
    assert_eq!(manifest.messages.len(), dialogue.len());
    let state: Value =
        serde_json::from_slice(&std::fs::read(state_path(directory.path(), manifest.id)?)?)?;
    assert_eq!(state["version"], 1);
    assert_eq!(
        state["checkpoints"].as_array().ok_or("checkpoints")?.len(),
        1
    );
    assert!(serde_json::to_vec(&state)?.len() < 220_000);
    assert_eq!(
        state["checkpoints"][0]["metadata"]["codexContinuation"],
        serde_json::to_value(history.snapshot()?)?
    );
    let roots = std::fs::read_dir(
        directory
            .path()
            .join("provider-state")
            .join(manifest.id.to_string()),
    )?
    .collect::<Result<Vec<_>, _>>()?;
    assert!(roots.len() <= 2);
    let retained_bytes = roots.iter().try_fold(0u64, |total, entry| {
        entry.metadata().map(|metadata| total + metadata.len())
    })?;
    assert!(retained_bytes <= 2 * serde_json::to_vec(&state)?.len() as u64);
    history.transition_generation(1)?;
    publish(&history, "new-generation")?;
    let retained: Value =
        serde_json::from_slice(&std::fs::read(state_path(directory.path(), manifest.id)?)?)?;
    assert_eq!(
        retained["checkpoints"]
            .as_array()
            .ok_or("retained generations")?
            .len(),
        2
    );
    assert_eq!(retained["checkpoints"][0], state["checkpoints"][0]);
    assert_eq!(load_session(manifest.id)?.messages.len(), dialogue.len());
    let mut runner = proptest::test_runner::TestRunner::new(proptest::test_runner::Config {
        cases: 16,
        ..Default::default()
    });
    runner.run(&".{0,128}", |secret| {
        let exercise = || -> TestResult {
            publish(&history, &secret)?;
            let mut cold = fixtures::session();
            cold.restore_provider_state(&get_session_message_envelopes(manifest.id)?)?;
            assert_eq!(history.snapshot()?, fixtures::history(&cold)?.snapshot()?);
            Ok(())
        };
        exercise().map_err(|error| proptest::test_runner::TestCaseError::fail(error.to_string()))
    })?;
    Ok(())
}
