//! Source-shape scenarios for the turn path: durable input admission before
//! dispatch, assistant flush before tool_result, and the Done arm plus
//! per-request token settlement.

use super::*;

// ============================================================================
// Scenario: Source-shape — agent_loop body invokes persist_user_message before dispatching to the provider
// ============================================================================

#[test]
fn agent_loop_invokes_persist_user_message_before_provider_dispatch() {
    // @step Given the sources of rust/agent-loop/src/agent_loop.rs and its agent_loop/prompt.rs and agent_loop/admission.rs submodules
    let turn_loop = read_agent_loop_src("agent_loop.rs");
    let prompt = read_agent_loop_src("agent_loop/prompt.rs");
    let admission = read_agent_loop_src("agent_loop/admission.rs");

    // @step When the files are scanned

    // @step Then admission::store_original durably persists the input through crate::persist::persist_input_original
    let store_original = section(&admission, "pub(super) async fn store_original(", "\n}\n");
    assert!(
        store_original.contains("crate::persist::persist_input_original(&session_id, id, original)"),
        "admission::store_original must persist the input via crate::persist::persist_input_original"
    );

    // @step And prompt::admit_prompt calls admission::store_original before session.set_status(SessionStatus::Running) and refuses the turn when it fails
    let admit_prompt = section(&prompt, "pub(super) async fn admit_prompt(", "\n}\n");
    let store_pos = position(
        admit_prompt,
        "if let Err(error) = admission::store_original(session, pending).await {",
        "prompt::admit_prompt",
    );
    let running_pos = position(
        admit_prompt,
        "session.set_status(SessionStatus::Running);",
        "prompt::admit_prompt",
    );
    assert!(
        store_pos < running_pos,
        "admission::store_original must precede set_status(SessionStatus::Running)"
    );
    let failure_path = &admit_prompt[store_pos..running_pos];
    assert!(
        failure_path.contains("InputDisposition::Refused { reason: error }")
            && failure_path.contains("return false;"),
        "a failed store_original must refuse the input and end the prompt before Running"
    );

    // @step And the turn loop gates on prompt::admit_prompt before admission::admit_turn dispatches to providers::run_selected_provider
    let gate_pos = position(
        &turn_loop,
        "if !prompt::admit_prompt(&session, &mut pending, &mut state).await {",
        "agent_loop.rs",
    );
    let admit_pos = position(
        &turn_loop,
        "admission::admit_turn(&session, &mut inner_session, &pending).await",
        "agent_loop.rs",
    );
    let dispatch_arm = section(
        &turn_loop[admit_pos..],
        "admission::Gate::Dispatch(submission) => {",
        "admission::settle_dispatch(",
    );
    assert!(
        gate_pos < admit_pos && dispatch_arm.contains("providers::run_selected_provider("),
        "the provider is dispatched only from admit_turn's Gate::Dispatch arm, after admit_prompt"
    );
    let admit_turn = section(&admission, "pub(super) async fn admit_turn(", "\n}\n");
    let admitted_pos = position(
        admit_turn,
        "input_receipts::admit(session_id, id)",
        "admission::admit_turn",
    );
    let dispatch_pos = position(
        admit_turn,
        "Ok(()) => Gate::Dispatch(id),",
        "admission::admit_turn",
    );
    assert!(
        admitted_pos < dispatch_pos,
        "admit_turn must durably admit the input before returning Gate::Dispatch"
    );
}

// ============================================================================
// Scenario: Source-shape — BackgroundOutput's StreamEvent::ToolResult arm
// persists assistant before tool_result
// ============================================================================

#[test]
fn background_output_tool_result_arm_persists_assistant_first() {
    // @step Given the source of rust/agent-loop/src/background_output.rs
    let src = read_agent_loop_src("background_output.rs");

    // @step When the file is scanned
    let arm = section(
        &src,
        "StreamEvent::ToolResult(ref tr)",
        "StreamEvent::ToolProgress(tp)",
    );

    // @step Then the StreamEvent::ToolResult arm calls self.persist_assistant_message()
    let assistant_flush_pos = position(arm, "self.persist_assistant_message()", "ToolResult arm");

    // @step And the same arm subsequently calls persist_tool_result_identified(...)
    let tool_result_pos = position(arm, "persist_tool_result_identified(", "ToolResult arm");

    // @step And the assistant-flush call precedes the tool-result persist call textually within that arm
    assert!(
        assistant_flush_pos < tool_result_pos,
        "self.persist_assistant_message() must precede persist_tool_result_identified(...) in the ToolResult arm"
    );
}

// ============================================================================
// Scenario: Source-shape — BackgroundOutput's StreamEvent::Done arm
// persists assistant then token state
// ============================================================================

#[test]
fn background_output_done_arm_persists_assistant_then_token_state() {
    // @step Given the source of rust/agent-loop/src/background_output.rs
    let src = read_agent_loop_src("background_output.rs");

    // @step When the file is scanned
    let arm = section(
        &src,
        "StreamEvent::Done(stop_reason)",
        "StreamEvent::CompactionStarted",
    );

    // @step Then the StreamEvent::Done arm calls self.persist_assistant_message_with_stop_reason(stop_reason) before it emits StreamChunk::done()
    let assistant_pos = position(
        arm,
        "self.persist_assistant_message_with_stop_reason(stop_reason)",
        "Done arm",
    );
    let done_pos = position(arm, "StreamChunk::done()", "Done arm");
    assert!(
        assistant_pos < done_pos,
        "the assistant message must be persisted before the Done chunk is emitted"
    );

    // @step And the token state is persisted per provider request: the stream driver binds the agent with .with_settlement_session(session_id), RigAgent wraps the hook in RequestSettlementHook, and the hook calls settle_request(session_id, identity, usage)
    let driver = read_workspace_src("cli/src/interactive/stream_loop/driver.rs");
    let bind_pos = position(
        &driver,
        ".with_settlement_session(session_id);",
        "stream_loop/driver.rs",
    );
    let stream_pos = position(
        &driver,
        ".prompt_streaming_with_history_and_hook(",
        "stream_loop/driver.rs",
    );
    assert!(
        bind_pos < stream_pos,
        "the agent must be bound to the settlement session before the stream starts"
    );
    let rig_agent = read_workspace_src("core/src/rig_agent.rs");
    let streaming = section(
        &rig_agent,
        "pub async fn prompt_streaming_with_history_and_hook<P>(",
        ".with_hook(hook)",
    );
    assert!(
        streaming.contains("RequestSettlementHook::optional(")
            && streaming.contains("self.settlement_session,"),
        "every streamed request's hook must be wrapped in RequestSettlementHook with the settlement session"
    );
    let hook = read_workspace_src("core/src/request_settlement_hook.rs");
    position(
        &hook,
        "settle_request(session_id, identity, usage)",
        "request_settlement_hook.rs",
    );
    let settlement = read_workspace_src("core/src/persistence/request_settlement.rs");
    let settle = section(&settlement, "pub fn settle_request(", "\n}\n");
    for field in [
        "token_usage.cumulative_billed_input = current",
        "token_usage.cumulative_billed_output = current",
        "token_usage.reasoning_tokens = current",
    ] {
        position(settle, field, "settle_request");
    }
}
