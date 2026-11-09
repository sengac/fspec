#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Feature: spec/features/completion-registration-layout-guards.feature
//!
//! CONT-011 guard: armed done() registration stays guarded across the provider
//! builder chains. Its own guard test file per the product owner (2026-11-05,
//! COV-075); the checks are the same source-shape sweep as CONT-002's
//! done_is_registered_only_while_auto_continue_is_armed in
//! auto_continue_engine_test.rs.

/// The seven production builder chains (doc §3 registration list).
const PROVIDER_CHAINS: &[&str] = &[
    "../providers/src/claude.rs",
    "../providers/src/openai.rs",
    "../providers/src/gemini.rs",
    "../providers/src/zai.rs",
    "../providers/src/codex/agent.rs",
    "../providers/src/copilot/rig_agent.rs",
    "../providers/src/custom/custom_provider.rs",
];

fn chain_source(rel: &str) -> String {
    if rel == "../providers/src/codex/agent.rs" {
        assert!(std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../providers/src/codex/mod.rs"
        ))
        .is_ok_and(|module| module.contains("mod agent;")));
    }
    let path = format!("{}/{}", env!("CARGO_MANIFEST_DIR"), rel);
    // BUG-448: a provider's module tree (openai.rs + openai/agent.rs).
    codelet_test_helpers::source_composition::read_tree_or_file(std::path::Path::new(&path))
        .unwrap_or_else(|e| panic!("cannot read {path}: {e}"))
}

#[test]
fn armed_done_registration_stays_guarded_across_provider_builders() {
    // @step Given a provider agent is built for a session marked armed
    let session_id = uuid::Uuid::new_v4();
    codelet_tools::set_continue_armed(session_id, true);
    assert!(
        codelet_tools::is_continue_armed(session_id),
        "armed registry must report the session as armed"
    );

    // @step Then the agent tool set includes the done tool
    // (source-shape: every chain conditionally adds DoneTool when armed)
    for chain in PROVIDER_CHAINS {
        let source = chain_source(chain);
        assert!(
            source.contains("DoneTool"),
            "{chain} must register DoneTool in its create_rig_agent chain"
        );
    }

    // @step And an agent built for an unarmed session does not include the done tool
    codelet_tools::set_continue_armed(session_id, false);
    assert!(
        !codelet_tools::is_continue_armed(session_id),
        "disarming must clear the armed registry entry"
    );

    // @step And all 7 provider builder chains register the done tool conditionally on the armed state
    for chain in PROVIDER_CHAINS {
        let source = chain_source(chain);
        assert!(
            source.contains("is_continue_armed"),
            "{chain} must gate DoneTool registration on is_continue_armed(session_id)"
        );
    }

    // @step And the DeepSearch sub-agent toolset never includes the done tool
    let deep_search = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../agent-loop/src/deep_search_handler.rs"
    ))
    .expect("deep_search_handler.rs must be readable");
    assert!(
        !deep_search.contains("DoneTool"),
        "DeepSearch sub-agent toolset must never include done()"
    );
}
