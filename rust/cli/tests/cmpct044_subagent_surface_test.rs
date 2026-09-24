//! Feature: spec/features/custom-provider-compactor-sub-agent-read-only-tool-surface.feature
//!
//! CMPCT-044/045 review follow-up (045-W1): the custom-provider dispatch
//! arm of the compactor sub-agent spawner used to call
//! `CustomProvider::create_rig_agent`, which attaches the FULL provider
//! tool surface (GenerateCompaction, InjectSummary, DeepSearch,
//! AgentManager, …) to the ephemeral compactor sub-agent — breaking the
//! 7-tool read-only surface contract (CMPCT-044 Rule [3]) and creating a
//! recursion path into compaction. This test pins the fix: the custom
//! provider builder must expose a sub-agent variant that attaches ONLY
//! the seven read-only infrastructure tools.
//!
//! Source-shape guard (established pattern — the marker strings are the
//! contract): reads the provider source via relative paths from the
//! codelet-cli manifest dir.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::PathBuf;

/// Locate a source file (crate-relative, e.g. "providers/src/custom/custom_provider.rs")
/// under the repo's `rust/` directory.
fn repo_source(rel: &str) -> String {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let rust_root = manifest.parent().expect("repo rust root");
    std::fs::read_to_string(rust_root.join(rel))
        .unwrap_or_else(|e| panic!("failed to read {rel}: {e}"))
}

/// Scenario: Custom-provider compactor sub-agent stays on the read-only tool surface
#[test]
fn custom_provider_compactor_sub_agent_stays_on_the_read_only_tool_surface() {
    // @step Given the compactor sub-agent is dispatched for a session on a registered Rhai custom provider
    // The contract is enforced by two cooperating source markers:
    //  (1) the custom provider builder accepts a sub-agent gate, and
    //  (2) the compactor spawner passes the gate on the custom-provider dispatch.
    let custom_provider_src = repo_source("providers/src/custom/custom_provider.rs");
    let compactor_src = repo_source("agent-loop/src/generate_compaction_handler.rs");

    // @step When the compactor agent is built through the custom-provider path
    // (1) `create_rig_agent` must carry a `sub_agent: bool` gate.
    assert!(
        custom_provider_src.contains("sub_agent: bool"),
        "CustomProvider::create_rig_agent must accept a `sub_agent: bool` gate \
         so the compactor sub-agent path can be built without the full surface"
    );

    // (2) the compactor spawner's custom-provider dispatch must pass `true`.
    assert!(
        compactor_src.contains(
            "true, // sub_agent: the compactor must stay on the 7-tool read-only surface"
        ) || compactor_src.contains("true, // sub_agent"),
        "the compactor spawner must pass `sub_agent: true` to \
         CustomProvider::create_rig_agent on the custom-provider dispatch arm"
    );

    // @step Then the built agent exposes exactly the seven read-only tools (Read, Grep, AstGrep, Glob, Ls, Bash, SessionSearch) and no other tool
    // The sub-agent branch must attach the seven read-only tools.
    for tool in [
        "ReadTool::new(session_id)",
        "GrepTool::new(session_id)",
        "AstGrepTool::new(session_id)",
        "GlobTool::new(session_id)",
        "LsTool::new(session_id)",
        "BashTool::new(session_id)",
        "SessionSearchTool::new(session_id)",
    ] {
        assert!(
            custom_provider_src.contains(tool),
            "the custom-provider sub-agent branch must attach {tool}"
        );
    }

    // @step And in particular it has no GenerateCompaction tool (no recursion into compaction), no InjectSummaryTool, no DeepSearchTool, no AgentManagerTool, and no Write/Edit tool
    // The sub-agent gate must guard EVERY infrastructure tool that the
    // full surface attaches — in particular GenerateCompaction (the
    // recursion risk). The full-surface branches keep the attachments;
    // the gate must be what separates them.
    assert!(
        custom_provider_src.contains("if sub_agent {"),
        "the custom-provider builder must gate the tool attachment on `sub_agent`"
    );
    assert!(
        custom_provider_src.contains("GenerateCompactionTool::new(session_id)"),
        "the full-surface branch must still attach GenerateCompactionTool \
         (parent agents keep the tool; the gate, not removal, is the fix)"
    );

    // And the sub-agent branch itself must NOT contain any of the
    // non-read-only infrastructure tools. Extract the `if sub_agent { ... }`
    // block body (brace-balanced scan) and assert their absence there.
    let gate_start = custom_provider_src
        .find("if sub_agent {")
        .expect("the `if sub_agent {` gate must exist");
    let mut depth = 0;
    let mut block_end = gate_start;
    for (offset, ch) in custom_provider_src[gate_start..].char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    block_end = gate_start + offset;
                    break;
                }
            }
            _ => {}
        }
    }
    let block_body = &custom_provider_src[gate_start..block_end];
    for forbidden in [
        "GenerateCompactionTool::new",
        "InjectSummaryTool::new",
        "DeepSearchTool::new",
        "AgentManagerTool::new",
        "WriteTool::new",
        "EditTool::new",
        "fspec_tool",
        "bridge_tool",
    ] {
        assert!(
            !block_body.contains(forbidden),
            "the custom-provider sub-agent branch must NOT attach {forbidden} — \
             the compactor sub-agent stays on the 7-tool read-only surface"
        );
    }
}
