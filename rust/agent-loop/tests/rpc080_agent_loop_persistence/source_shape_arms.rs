//! Source-shape scenarios for the terminal stream arms (Error, Interrupted)
//! and the crate::persist boundary.

use super::*;

// ============================================================================
// Scenario: Source-shape — BackgroundOutput's StreamEvent::Error arm
// flushes accumulated assistant content
// ============================================================================

#[test]
fn background_output_error_arm_flushes_assistant() {
    // @step Given the source of rust/agent-loop/src/background_output.rs
    let src = read_agent_loop_src("background_output.rs");

    // @step When the file is scanned
    let arm = section(
        &src,
        "StreamEvent::Error(error)",
        "StreamEvent::Interrupted(queued)",
    );

    // @step Then the StreamEvent::Error arm calls self.persist_assistant_message()
    assert!(
        arm.contains("self.persist_assistant_message()"),
        "Error arm must call self.persist_assistant_message() to flush partial content"
    );
}

// ============================================================================
// Scenario: Source-shape — BackgroundOutput's StreamEvent::Interrupted arm
// flushes accumulated assistant content
// ============================================================================

#[test]
fn background_output_interrupted_arm_flushes_assistant() {
    // @step Given the source of rust/agent-loop/src/background_output.rs
    let src = read_agent_loop_src("background_output.rs");

    // @step When the file is scanned
    let arm = section(
        &src,
        "StreamEvent::Interrupted(queued)",
        "StreamEvent::Done(stop_reason)",
    );

    // @step Then the StreamEvent::Interrupted arm calls self.persist_assistant_outcome(None, reason) with the session's interruption reason before it emits the interrupted chunk
    let reason_pos = position(
        arm,
        "let reason = self.session.interruption_reason();",
        "Interrupted arm",
    );
    let flush_pos = position(
        arm,
        "self.persist_assistant_outcome(None, reason);",
        "Interrupted arm",
    );
    let chunk_pos = position(arm, "StreamChunk::interrupted(queued)", "Interrupted arm");
    assert!(
        reason_pos < flush_pos && flush_pos < chunk_pos,
        "the Interrupted arm must flush the assistant with the interruption reason before emitting the chunk"
    );

    // @step And persist_assistant_outcome flushes the accumulated assistant content through crate::persist::persist_assistant_message_identified
    let outcome = section(&src, "fn persist_assistant_outcome(", "\n    }\n");
    let take_pos = position(
        outcome,
        "let content = self.take_assistant_content();",
        "persist_assistant_outcome",
    );
    let persist_pos = position(
        outcome,
        "crate::persist::persist_assistant_message_identified(",
        "persist_assistant_outcome",
    );
    assert!(
        take_pos < persist_pos,
        "persist_assistant_outcome must persist the content it takes from the accumulator"
    );
}

// ============================================================================
// Scenario: Boundary — persistence calls in codelet-agent-loop import from
// crate::persist (not codelet_napi)
// ============================================================================

#[test]
fn persistence_calls_in_agent_loop_use_crate_persist_not_napi() {
    // @step Given the codelet-agent-loop crate
    let src_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src");

    // @step When its source tree is scanned
    let mut violations: Vec<String> = Vec::new();
    for entry in walk_rs_files(&src_dir) {
        let body = read_raw(&entry);

        // @step Then no .rs file under rust/agent-loop/src/ references codelet_napi::persist
        if body.contains("codelet_napi::persist") || body.contains("codelet_napi :: persist") {
            violations.push(format!(
                "{}: references codelet_napi::persist",
                entry.display()
            ));
        }
    }
    assert!(
        violations.is_empty(),
        "codelet_napi::persist must not be referenced from rust/agent-loop/src/. \
         Violations:\n{}",
        violations.join("\n")
    );

    // @step And persist.rs lives at rust/agent-loop/src/persist.rs
    let persist_path = src_dir.join("persist.rs");
    assert!(
        persist_path.is_file(),
        "rust/agent-loop/src/persist.rs must exist"
    );

    // @step And it exports persist_user_message, persist_assistant_message_internal, persist_tool_result_internal, and persist_token_state as pub
    // persist.rs is read as its module tree (it keeps persist_token_state in
    // the `#[path = "persist_state.rs"] mod state;` child); a fn defined in a
    // private child counts as exported only when persist.rs re-exports it.
    let persist_root = read_raw(&persist_path);
    let persist_tree = codelet_test_helpers::source_composition::read_module_tree(&persist_path)
        .unwrap_or_else(|e| panic!("must read persist.rs module tree: {e}"));
    for name in [
        "persist_user_message",
        "persist_assistant_message_internal",
        "persist_tool_result_internal",
        "persist_token_state",
    ] {
        let definition = format!("pub fn {name}(");
        assert!(
            persist_tree.contains(&definition),
            "the persist.rs module tree must define `{definition}`"
        );
        let re_exported = persist_root.split("pub use ").skip(1).any(|statement| {
            statement
                .split(';')
                .next()
                .is_some_and(|items| items.contains(name))
        });
        assert!(
            persist_root.contains(&definition) || re_exported,
            "persist.rs must export `{name}` as pub (defined there or re-exported via `pub use`)"
        );
    }
}
