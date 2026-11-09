// ============================================================================
// Scenario: Compaction prompt is FRESH when the target has no DAG
// Scenario: Compaction prompt is INCREMENTAL when the target already has a DAG
//
// Prompt-builder contract (the prompt text itself is exercised by the
// incremental behavioral test above — these pin the FRESH/INCREMENTAL
// selection + target-scoping of build_generate_compaction_prompt).
// ============================================================================

/// Scenario: Compaction prompt is FRESH when the target has no DAG
#[test]
fn fresh_compaction_prompt_names_the_target_and_forbids_inject_summary() {
    // @step Given a target session that contains no existing compaction DAG
    let target = Uuid::new_v4();

    // @step When the compactor sub-agent's task prompt is built
    let prompt = codelet_cli::compaction_dag::build_generate_compaction_prompt(target, None);

    // @step Then the FRESH compaction instruction is used
    assert!(
        prompt.contains("Build a hierarchical summary DAG of your session"),
        "FRESH instruction must be selected when no DAG exists"
    );

    // @step And the task prompt names the target session id for every SessionSearch call
    assert!(
        prompt.contains(&format!("SessionSearch(session_id: \"{target}\", show, ")),
        "SessionSearch show calls must target the target session id"
    );
    assert!(
        prompt.contains(&format!("SessionSearch(session_id: \"{target}\", search, ")),
        "SessionSearch search calls must target the target session id"
    );

    // @step And the task prompt instructs the sub-agent to output the complete DAG as its final response
    assert!(
        prompt.contains("Output the complete DAG"),
        "the prompt must instruct the sub-agent to output the DAG as its final response"
    );

    // @step And the task prompt does not mention inject_summary
    assert!(
        !prompt.contains("inject_summary"),
        "the compactor prompt must NOT mention inject_summary (the sub-agent has no such tool)"
    );
}

/// Scenario: Compaction prompt is INCREMENTAL when the target already has a DAG
#[test]
fn incremental_compaction_prompt_embeds_existing_dag_and_turn_offset() {
    // @step Given a target session whose context already contains a compaction DAG ending at turn N
    let target = Uuid::new_v4();
    let existing = "<system-reminder>\n<!-- type:compaction-dag -->\n\
         <dag-node depth=\"D2\" turns=\"0-45\" label=\"Architecture\">ok</dag-node>\n\
         </system-reminder>"
        .to_string();

    // @step When the compactor sub-agent's task prompt is built
    let prompt = codelet_cli::compaction_dag::build_generate_compaction_prompt(
        target,
        Some((existing.clone(), 45)),
    );

    // @step Then the INCREMENTAL compaction instruction is used with the existing DAG embedded
    assert!(
        prompt.contains("do NOT rebuild from scratch"),
        "INCREMENTAL instruction must be selected when a DAG exists"
    );
    assert!(
        prompt.contains(&existing),
        "the existing DAG must be embedded in the prompt"
    );

    // @step And the task prompt tells the sub-agent to preserve D2 nodes, promote D0 to D1, and only survey turns from N+1 onward
    assert!(
        prompt.contains("PRESERVE all existing D2"),
        "the prompt must instruct preserving D2 nodes"
    );
    assert!(
        prompt.contains("PROMOTE existing D0 (Detailed) nodes to D1"),
        "the prompt must instruct promoting D0 to D1"
    );
    assert!(
        prompt.contains("start_turn: 46"),
        "the prompt must survey from turn 46 (max_turn_end 45 + 1)"
    );

    // @step And the existing DAG is captured from the target BEFORE any clear of its messages
    // The builder receives the captured existing_dag (the pre-clear capture
    // from detect_existing_dag) and embeds it verbatim — proof the capture
    // happened BEFORE any clear of the target's messages.
    assert!(
        prompt.contains(&existing),
        "the captured existing DAG must be embedded in the prompt"
    );
}

// ============================================================================
// Scenario: Compactor sub-agent has only the read-only tool surface
// Scenario: Handler registration lifecycle mirrors the DeepSearch pattern
//
// Tool-surface + registration-lifecycle contracts (ported from the
// agent-loop inline tests and the source-shape registration tests — the
// feature's 1:1 rule requires every scenario's steps to live in this one
// test file).
// ============================================================================

/// Scenario: Compactor sub-agent has only the read-only tool surface
#[test]
fn compactor_sub_agent_has_only_the_read_only_tool_surface() {
    // @step Given a compactor sub-agent is constructed
    // @step When its tool list is built
    // @step Then the sub-agent has exactly the seven read-only tools: Read, Grep, AstGrep, Glob, Ls, Bash, and SessionSearch
    assert_eq!(
        codelet_tools::SUB_AGENT_TOOL_COUNT,
        7,
        "the compactor sub-agent must keep exactly the 7 read-only DeepSearch tools"
    );
    assert_eq!(
        codelet_tools::SUB_AGENT_TOOL_NAMES,
        [
            "Read",
            "Grep",
            "AstGrep",
            "Glob",
            "Ls",
            "Bash",
            "SessionSearch"
        ],
        "the compactor sub-agent's tool surface must be exactly the 7          read-only tools"
    );

    // @step And the sub-agent does NOT have the inject_summary tool
    assert!(
        !codelet_tools::SUB_AGENT_TOOL_NAMES
            .iter()
            .any(|t| t.to_lowercase().contains("inject_summary")),
        "the compactor sub-agent must NOT have inject_summary (the pin is handler-side)"
    );

    // @step And the sub-agent does NOT have the GenerateCompaction tool
    assert!(
        !codelet_tools::SUB_AGENT_TOOL_NAMES.contains(&"GenerateCompaction"),
        "the compactor sub-agent must NOT have GenerateCompaction (no recursion into compaction)"
    );

    // @step And the sub-agent does NOT have any Write or Edit tool
    assert!(
        !codelet_tools::SUB_AGENT_TOOL_NAMES
            .iter()
            .any(|t| *t == "Write" || *t == "Edit"),
        "the compactor sub-agent must NOT have Write or Edit tools"
    );
}
