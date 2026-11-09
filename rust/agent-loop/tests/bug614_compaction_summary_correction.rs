//! Feature: spec/features/compaction-summary-correction.feature
//!
//! BUG-614: incident 21b391d2. The main model called inject_summary in an
//! ordinary turn with a markdown handoff note and no dag-node. The rejection
//! ended the turn, the model never saw the error, and every later input (an
//! AgentManager report) failed before dispatch. These tests drive the
//! production agent loop with scripted provider replies.

#[path = "support/bug614_scripted.rs"]
pub mod scripted;

use scripted::{errors, messages, notifications, texts, Candidate, Reply, Scripted};

/// The shape of the incident's candidate: a complete markdown handoff note.
const PROSE: &str = "# MarketPulse continuation orchestration\n\n## User requests / hard constraints (most recent highest priority)\n- Rust only. DO NOT USE OTHER LANGUAGES.\n\n## Next orchestration steps\n1. Relay the Rust-only rule to every worker.\n7. Final concise user update. Do not imply full UI migration finished.\n";
const NO_NODES: &str = "invalid DAG candidate: missing or unclosed dag-node structure";
const VALID: &str =
    "<dag-node depth=\"D1\" turns=\"0-{end}\" label=\"Work\">Completed evidence</dag-node>";

/// The last tool result the model received in an ordinary request, as JSON.
fn last_tool_result(body: &serde_json::Value) -> serde_json::Value {
    let text = messages(body)
        .into_iter()
        .rev()
        .find(|(role, _)| role == "tool")
        .map(|(_, text)| text)
        .unwrap_or_default();
    let value: serde_json::Value = serde_json::from_str(&text).unwrap_or(serde_json::Value::Null);
    match value.as_str() {
        Some(encoded) => serde_json::from_str(encoded).unwrap_or(serde_json::Value::Null),
        None => value,
    }
}

fn assert_syntax(result: &serde_json::Value) {
    let syntax = result["syntax"].as_str().unwrap_or_default();
    assert!(
        syntax.contains("Close every <dag-node> with </dag-node>"),
        "{result}"
    );
    assert!(
        syntax.contains("Same-depth peers must be disjoint"),
        "{result}"
    );
    assert!(syntax.contains("<dag-files>"), "{result}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_a_prose_summary_in_an_ordinary_turn_returns_the_exact_error_to_the_model() {
    // @step Given a main model in an ordinary turn with the production summary handler
    let mut harness = Scripted::new().await;
    harness.script(vec![
        Reply::inject_summary(PROSE),
        Reply::text("Continuing the orchestration without a summary."),
    ]);
    // @step When it calls inject_summary with a markdown handoff note that has no dag-node
    harness.send("please continue where you left off last session");
    let chunks = harness.until_done().await;
    let requests = harness.ordinary_requests().await;
    // The incident's second input: a sub-agent's AgentManager report, which
    // failed before dispatch while the rejection blocked the session.
    harness.script(vec![Reply::text("Worker report handled.")]);
    harness.report("AgentManager report: the worker finished its Rust migration slice");
    let report = harness.until_done().await;
    assert_eq!(
        (requests.len(), errors(&chunks), errors(&report)),
        (2, vec![], vec![]),
        "the rejection must go back to the model in the same turn and must not block the next input"
    );
    // @step Then the tool result is rejected attempt 1 of 3 with the exact validator error and the DAG syntax
    let result = last_tool_result(&requests[1]);
    assert_eq!(result["status"], "rejected", "{result}");
    assert_eq!(result["attempt"], 1, "{result}");
    assert_eq!(result["max_attempts"], 3, "{result}");
    assert_eq!(result["error"], NO_NODES, "{result}");
    assert_eq!(result["installed"], false, "{result}");
    assert_syntax(&result);
    // @step And the model's next request in the same turn carries that result
    assert!(texts(&chunks).contains("Continuing the orchestration without a summary."));
    // @step And nothing is installed and ordinary inference is not blocked
    assert!(harness.inference_ready().await);
    assert!(harness.committed_summary().is_none());
    // @step And a later background AgentManager report runs ordinary inference
    assert!(texts(&report).contains("Worker report handled."));
    assert_eq!(harness.ordinary_requests().await.len(), 3);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_a_corrected_resubmission_in_the_same_turn_is_committed() {
    // @step Given a main model whose first inject_summary candidate was rejected
    let mut harness = Scripted::new().await;
    harness.script(vec![
        Reply::inject_summary(PROSE),
        Reply::inject_summary(VALID),
        Reply::text("Resumed after the summary."),
    ]);
    // @step When it resubmits a valid DAG in the same turn
    harness.send("summarize and continue");
    let chunks = harness.until_done().await;
    let requests = harness.ordinary_requests().await;
    assert!(requests.len() >= 2, "errors: {:?}", errors(&chunks));
    assert_eq!(last_tool_result(&requests[1])["status"], "rejected");
    // @step Then the second submission is queued and the turn stops at the tool boundary
    assert!(errors(&chunks).is_empty(), "{:?}", errors(&chunks));
    // @step And the turn's reconciliation commits the corrected DAG
    assert!(chunks.iter().any(|chunk| matches!(
        chunk,
        codelet_rpc_types::StreamChunk::CompactionComplete { .. }
    )));
    let summary = harness
        .committed_summary()
        .expect("committed corrected DAG");
    assert!(
        summary.starts_with("<dag-node depth=\"D1\" turns=\"0-")
            && summary.contains("Completed evidence"),
        "{summary}"
    );
    assert!(harness.inference_ready().await);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_three_rejected_submissions_leave_the_summary_uninstalled_and_the_turn_continues()
{
    // @step Given a main model that submits an invalid DAG three times in one turn
    let mut harness = Scripted::new().await;
    harness.script(vec![
        Reply::inject_summary(PROSE),
        Reply::inject_summary(PROSE),
        Reply::inject_summary(PROSE),
        Reply::text("Answer without a summary."),
    ]);
    // @step When the third submission is rejected
    harness.send("summarize and continue");
    let chunks = harness.until_done().await;
    let requests = harness.ordinary_requests().await;
    assert_eq!(requests.len(), 4, "errors: {:?}", errors(&chunks));
    assert_eq!(last_tool_result(&requests[2])["attempt"], 2);
    // @step Then the result is rejected_final with installed false
    let result = last_tool_result(&requests[3]);
    assert_eq!(result["status"], "rejected_final", "{result}");
    assert_eq!(result["attempt"], 3, "{result}");
    assert_eq!(result["installed"], false, "{result}");
    assert_eq!(result["error"], NO_NODES, "{result}");
    // @step And the UI shows Summary not installed with the exact validator error
    assert!(
        notifications(&chunks).contains(&format!("Summary not installed: {NO_NODES}")),
        "{:?}",
        notifications(&chunks)
    );
    // @step And committed memory is unchanged and the model finishes its answer
    assert!(texts(&chunks).contains("Answer without a summary."));
    assert!(errors(&chunks).is_empty(), "{:?}", errors(&chunks));
    let manifest = codelet_core::persistence::load_session(harness.session.id).expect("manifest");
    assert!(manifest.compaction.is_none() && manifest.compaction_commit.is_none());
    // @step And the next input runs ordinary inference
    harness.script(vec![Reply::text("Next answer.")]);
    harness.send("next task");
    let chunks = harness.until_done().await;
    assert!(errors(&chunks).is_empty(), "{:?}", errors(&chunks));
    assert!(texts(&chunks).contains("Next answer."));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_a_dag_cut_off_mid_node_asks_for_a_whole_shorter_dag() {
    // @step Given a main model whose inject_summary content ends inside an unclosed dag-node after 2 complete nodes
    let truncated = "<dag-node depth=\"D0\" turns=\"0-0\" label=\"First\">evidence</dag-node>\n<dag-node depth=\"D0\" turns=\"1-1\" label=\"Second\">evidence</dag-node>\n<dag-node depth=\"D0\" turns=\"2-";
    let mut harness = Scripted::new().await;
    harness.script(vec![Reply::inject_summary(truncated), Reply::text("ok")]);
    // @step When the submission is validated
    harness.send("summarize");
    let chunks = harness.until_done().await;
    let requests = harness.ordinary_requests().await;
    assert_eq!(requests.len(), 2, "errors: {:?}", errors(&chunks));
    let result = last_tool_result(&requests[1]);
    assert_eq!(result["status"], "rejected", "{result}");
    // @step Then the result says the DAG was truncated after 2 complete nodes
    let instruction = result["instruction"].as_str().unwrap_or_default();
    assert!(
        instruction.contains("truncated after 2 complete nodes"),
        "{result}"
    );
    // @step And it asks for a whole new shorter DAG
    assert!(instruction.contains("whole new, shorter DAG"), "{result}");
}

#[test]
fn scenario_a_truncated_inject_summary_tool_call_gets_dag_truncation_feedback() {
    // @step Given an inject_summary tool call cut off by the output limit after 3 complete nodes
    let node = |turn: usize| {
        format!("<dag-node depth=\"D0\" turns=\"{turn}-{turn}\" label=\"Work\">evidence</dag-node>")
    };
    let content = format!(
        "{}{}{}<dag-node depth=\"D0\" turns=\"3-",
        node(0),
        node(1),
        node(2)
    );
    let arguments = serde_json::json!({ "content": content }).to_string();
    let partial = arguments.trim_end_matches("\"}");
    let error = format!(
        "Tool call truncated due to output token limit. Tool 'inject_summary' received incomplete JSON arguments. Partial arguments: {partial}"
    );
    // @step When the truncated tool call recovery message is built
    let message = codelet_cli::interactive::build_truncation_recovery_message(&error);
    // @step Then it says the DAG was truncated after 3 complete nodes and asks for a whole new shorter DAG
    assert!(
        message.contains("truncated after 3 complete nodes"),
        "{message}"
    );
    assert!(message.contains("whole new, shorter DAG"), "{message}");
    // @step And it does not give the Bash heredoc advice for file writes
    assert!(!message.contains("heredoc"), "{message}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_exhausted_self_target_generation_leaves_the_callers_turn_running() {
    // @step Given a main model that calls GenerateCompaction for its own session
    let mut harness = Scripted::new().await;
    harness.script(vec![
        Reply::generate_compaction(),
        Reply::text("Continuing on the current context."),
    ]);
    // @step When the compactor's three candidates are all malformed
    harness.candidates(vec![
        Candidate::stop(PROSE),
        Candidate::stop(PROSE),
        Candidate::stop(PROSE),
    ]);
    harness.send("compact yourself and continue");
    let chunks = harness.until_done().await;
    assert_eq!(harness.builder_requests().await.len(), 3);
    // @step Then the caller receives a rejected_final result and its turn continues
    let requests = harness.ordinary_requests().await;
    assert_eq!(requests.len(), 2, "errors: {:?}", errors(&chunks));
    let result = last_tool_result(&requests[1]);
    assert_eq!(result["status"], "rejected_final", "{result}");
    assert_eq!(result["installed"], false, "{result}");
    let error = result["error"].as_str().unwrap_or_default().to_owned();
    assert!(error.contains(NO_NODES), "{result}");
    assert!(texts(&chunks).contains("Continuing on the current context."));
    // @step And nothing is installed and the UI shows Summary not installed
    assert!(harness.committed_summary().is_none());
    assert!(
        notifications(&chunks).contains(&format!("Summary not installed: {error}")),
        "{:?}",
        notifications(&chunks)
    );
    assert!(errors(&chunks).is_empty(), "{:?}", errors(&chunks));
    // @step And the next input runs ordinary inference
    harness.script(vec![Reply::text("Next answer.")]);
    harness.send("next task");
    let chunks = harness.until_done().await;
    assert!(errors(&chunks).is_empty(), "{:?}", errors(&chunks));
    assert!(texts(&chunks).contains("Next answer."));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_a_durable_authority_block_stays_reachable_by_restore() {
    // @step Given ordinary inference is blocked for an unresolved publication failure
    let mut harness = Scripted::new().await;
    harness
        .inner()
        .await
        .block_compaction_inference("publication requires explicit resolution".into());
    harness.send("blocked input");
    let chunks = harness.until_done().await;
    assert!(errors(&chunks)
        .iter()
        .any(|error| error.contains("publication requires explicit resolution")));
    assert!(harness.ordinary_requests().await.is_empty());
    // @step When the committed session is restored
    {
        let manifest = codelet_core::persistence::load_session(harness.session.id)
            .expect("committed manifest");
        harness
            .inner()
            .await
            .restore_compaction_boundary(&manifest)
            .expect("restore the committed session");
    }
    // @step Then the block is cleared
    assert!(harness.inference_ready().await);
    // @step And the next input runs ordinary inference
    harness.script(vec![Reply::text("Restored answer.")]);
    harness.send("after restore");
    let chunks = harness.until_done().await;
    assert!(errors(&chunks).is_empty(), "{:?}", errors(&chunks));
    assert!(texts(&chunks).contains("Restored answer."));
}

/// Index of the first user message carrying the compactor task.
fn task_index(messages: &[(String, String)]) -> Option<usize> {
    messages
        .iter()
        .position(|(role, text)| role == "user" && text.contains("Target session:"))
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_compactor_correction_continues_the_same_synthesis_conversation() {
    // @step Given a compactor whose synthesized candidate has no dag-node
    let mut harness = Scripted::new().await;
    harness.script(vec![
        Reply::generate_compaction(),
        Reply::text("Continuing."),
    ]);
    harness.candidates(vec![
        Candidate::stop("summary without nodes"),
        Candidate::stop(VALID),
    ]);
    // @step When the candidate is rejected
    harness.send("compact yourself");
    let chunks = harness.until_done().await;
    let builder = harness.builder_requests().await;
    assert_eq!(builder.len(), 2, "errors: {:?}", errors(&chunks));
    // @step Then the correction request carries the rejected candidate as the assistant turn followed by the exact error
    let (first, second) = (messages(&builder[0]), messages(&builder[1]));
    let task = task_index(&first).expect("compactor task");
    assert_eq!(
        task_index(&second),
        Some(task),
        "same conversation: {second:?}"
    );
    assert_eq!(second[task], first[task]);
    let rejected = second
        .iter()
        .position(|(role, text)| role == "assistant" && text == "summary without nodes")
        .expect("rejected candidate as the assistant turn");
    let (role, correction) = &second[rejected + 1];
    assert_eq!(role, "user");
    assert!(correction.contains(NO_NODES), "{correction}");
    // @step And a valid corrected candidate becomes committed memory
    let summary = harness.committed_summary().expect("committed correction");
    assert!(summary.contains("Completed evidence"), "{summary}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_a_malformed_node_in_a_truncated_compactor_chunk_is_corrected_at_once() {
    // @step Given a compactor chunk that stops at the output limit after a closed but malformed dag-node
    let chunk = "<dag-node depth=\"D9\" turns=\"0-0\" label=\"Bad\">unsupported depth</dag-node>\n<dag-node depth=\"D1\" turns=\"1-";
    let mut harness = Scripted::new().await;
    harness.script(vec![
        Reply::generate_compaction(),
        Reply::text("Continuing."),
    ]);
    harness.candidates(vec![Candidate::length(chunk), Candidate::stop(VALID)]);
    // @step When the chunk ends
    harness.send("compact yourself");
    let chunks = harness.until_done().await;
    let builder = harness.builder_requests().await;
    assert_eq!(builder.len(), 2, "errors: {:?}", errors(&chunks));
    // @step Then the next request is a correction carrying the exact error rather than a continuation
    let second = messages(&builder[1]);
    let (role, correction) = second.last().expect("correction turn");
    assert_eq!(role, "user");
    assert!(correction.contains("invalid DAG candidate"), "{correction}");
    assert!(
        !correction.contains("Continue the SAME DAG"),
        "{correction}"
    );
    assert!(second
        .iter()
        .any(|(role, text)| role == "assistant" && text.contains("unsupported depth")));
    assert!(harness.committed_summary().is_some());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_a_compactor_candidate_ending_mid_node_is_continued() {
    // @step Given a compactor candidate that ends inside an unclosed dag-node with a normal stop reason
    let whole =
        "<dag-node depth=\"D2\" turns=\"0-{end}\" label=\"Whole\">Completed evidence</dag-node>";
    let mut harness = Scripted::new().await;
    harness.script(vec![
        Reply::generate_compaction(),
        Reply::text("Continuing."),
    ]);
    harness.candidates(vec![
        Candidate::stop(&format!("{whole}\n<dag-node depth=\"D1\" turns=\"0-")),
        Candidate::stop(VALID),
    ]);
    // @step When the response ends
    harness.send("compact yourself");
    let chunks = harness.until_done().await;
    let builder = harness.builder_requests().await;
    assert_eq!(builder.len(), 2, "errors: {:?}", errors(&chunks));
    // @step Then the next request continues from its complete nodes rather than restarting
    let second = messages(&builder[1]);
    let (role, instruction) = second.last().expect("continuation turn");
    assert_eq!(role, "user");
    assert!(
        instruction.contains("Continue the SAME DAG"),
        "{instruction}"
    );
    let (role, kept) = &second[second.len() - 2];
    assert_eq!(role, "assistant");
    assert!(
        kept.starts_with("<dag-node depth=\"D2\"") && kept.ends_with("</dag-node>"),
        "{kept}"
    );
    // @step And the assembled DAG is validated before it is returned
    let summary = harness
        .committed_summary()
        .expect("committed continued DAG");
    assert!(
        summary.contains("label=\"Whole\"") && summary.contains("label=\"Work\""),
        "{summary}"
    );
}
