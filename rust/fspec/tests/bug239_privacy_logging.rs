//! Feature: spec/features/privacy-conscious-diagnostic-logging.feature

#[allow(dead_code)]
#[path = "../src/common.rs"]
mod common;

#[path = "support/bug239_logging.rs"]
mod logging;
use logging::{child, run_child};
use tracing_subscriber::layer::SubscriberExt;

const MARKERS: &[&str] = &[
    "SYSTEM_PRIVATE",
    "PROMPT_PRIVATE",
    "COMPLETION_PRIVATE",
    "ARGS_PRIVATE",
    "RESULT_PRIVATE",
];

fn emit() {
    use rig::telemetry::SpanCombinator;
    let system = format!("SYSTEM_PRIVATE{}", "s".repeat(20000));
    let span = tracing::info_span!(
        "invoke_agent",
        session_id = "session-239",
        request_id = "request-239",
        gen_ai.system_instructions = %system,
        gen_ai.usage.input_tokens = 43,
        gen_ai.prompt = "PROMPT_PRIVATE",
        gen_ai.completion = "COMPLETION_PRIVATE",
        gen_ai.input.messages = tracing::field::Empty,
        gen_ai.output.messages = tracing::field::Empty
    );
    span.record_model_input(&serde_json::json!({"text": "PROMPT_PRIVATE"}));
    span.record_model_output(&serde_json::json!({"text": "COMPLETION_PRIVATE"}));
    let _agent = span.enter();
    let tool = tracing::info_span!(
        "execute_tool",
        gen_ai.tool.name = "synthetic",
        gen_ai.tool.call.id = "call-239",
        gen_ai.tool.call.arguments = "ARGS_PRIVATE api_key=KEY_SECRET",
        gen_ai.tool.call.result = "RESULT_PRIVATE"
    );
    let _tool = tool.enter();
    tracing::error!(
        elapsed_ms = 12,
        error = "transport failed: connection reset: peer closed",
        "SSE failure"
    );
    tracing::info!(
        authorization = "Bearer BEARER_SECRET",
        api_key = "KEY_SECRET",
        "credential field test"
    );
    tracing::info!("auth failed: Bearer BEARER_SECRET api_key=KEY_SECRET");
    tracing::info!(
        input = r#"{"api_key":"JSON_SECRET","password":"QUOTED_SECRET","refresh_token":"REFRESH_SECRET"}"#,
        "credential assignment test"
    );
    tracing::info!(gen_ai.tool.call.result = %format!("RESULT_PRIVATE{}TAIL_PRIVATE", "x".repeat(12000)), "tool finished");
    tracing::trace!(target: "rig::completions", "completion request: PROMPT_PRIVATE");
}

#[tokio::test]
async fn scenario_daemon_stderr_never_inherits_local_payload_authorization() {
    // @step Given explicit local payload diagnostics are enabled
    if run_child().await {
        return;
    }
    // @step When nested agent and tool spans emit synthetic payloads to daemon stderr
    let (logs, _home) = child(
        "scenario_daemon_stderr_never_inherits_local_payload_authorization",
        true,
        "daemon",
    );
    // @step Then stderr omits payload and credential markers
    for marker in MARKERS {
        assert!(!logs.contains(marker), "leaked {marker}");
    }
    for marker in [
        "KEY_SECRET",
        "BEARER_SECRET",
        "JSON_SECRET",
        "QUOTED_SECRET",
        "REFRESH_SECRET",
    ] {
        assert!(!logs.contains(marker), "leaked {marker}");
    }
    // @step And stderr retains correlation metadata and actionable errors
    for marker in [
        "session-239",
        "request-239",
        "transport failed: connection reset: peer closed",
    ] {
        assert!(logs.contains(marker), "missing {marker}");
    }
}

#[tokio::test]
async fn scenario_default_local_logs_preserve_errors_without_conversation_payloads() {
    // @step Given local diagnostic logging with ordinary trace verbosity
    if run_child().await {
        return;
    }
    // @step When nested agent and tool spans emit synthetic payloads and an SSE failure
    let (client, _client_home) = child(
        "scenario_default_local_logs_preserve_errors_without_conversation_payloads",
        false,
        "client",
    );
    let (combined, _combined_home) = child(
        "scenario_default_local_logs_preserve_errors_without_conversation_payloads",
        false,
        "combined",
    );
    let (shared, _shared_home) = child(
        "scenario_default_local_logs_preserve_errors_without_conversation_payloads",
        false,
        "common",
    );
    let (invalid, _invalid_home) = child(
        "scenario_default_local_logs_preserve_errors_without_conversation_payloads",
        false,
        "invalid",
    );
    let (legacy, _legacy_home) = child(
        "scenario_default_local_logs_preserve_errors_without_conversation_payloads",
        false,
        "legacy",
    );
    eprintln!("BUG-239 synthetic legacy_bytes={} protected_client_bytes={} legacy_max_line={} protected_max_line={}", legacy.len(), client.len(), legacy.lines().map(str::len).max().unwrap_or(0), client.lines().map(str::len).max().unwrap_or(0));
    assert!(
        legacy.len() > client.len() * 10,
        "default logging still expands payloads"
    );
    let logs = format!("{client}{combined}{shared}{invalid}");
    // @step Then local logs omit all system prompt completion and tool payload markers
    for marker in MARKERS {
        assert!(!logs.contains(marker), "leaked {marker}");
    }
    // @step And correlation metadata and the complete actionable error chain remain readable
    for metadata in [
        "session-239",
        "request-239",
        "call-239",
        "elapsed_ms=12",
        "gen_ai.usage.input_tokens=43",
        "transport failed: connection reset: peer closed",
    ] {
        assert!(logs.contains(metadata), "missing {metadata}");
    }
}

#[tokio::test]
async fn scenario_explicit_local_payload_diagnostics_are_bounded_and_credential_safe() {
    // @step Given explicit local payload diagnostics are enabled
    if run_child().await {
        return;
    }
    // @step When nested agent and tool spans emit synthetic payloads and credentials
    let (logs, home) = child(
        "scenario_explicit_local_payload_diagnostics_are_bounded_and_credential_safe",
        true,
        "client",
    );
    // @step Then local logs include bounded payload markers but omit credential values
    for marker in MARKERS {
        assert!(logs.contains(marker), "missing {marker}");
    }
    assert_eq!(
        logs.matches("SYSTEM_PRIVATE").count(),
        1,
        "span payload repeats across nested events"
    );
    for marker in [
        "KEY_SECRET",
        "BEARER_SECRET",
        "TAIL_PRIVATE",
        "JSON_SECRET",
        "QUOTED_SECRET",
        "REFRESH_SECRET",
    ] {
        assert!(!logs.contains(marker), "leaked {marker}");
    }
    assert!(
        logs.lines().all(|line| line.len() < 10000),
        "unbounded log record"
    );
    // @step And local diagnostic storage is private with five file rotation retention
    let directory = home.path().join(".fspec/logs/private");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            std::fs::metadata(&directory)
                .expect("private log directory")
                .permissions()
                .mode()
                & 0o777,
            0o700
        );
    }
    assert!(directory.is_dir());
    assert!(std::fs::read_dir(directory).expect("private logs").count() <= 5);
}

#[test]
fn scenario_broadcast_never_inherits_local_payload_authorization() {
    // @step Given explicit local payload diagnostics are enabled
    if std::env::var_os("BUG239_CHILD").is_none() {
        let _result = child(
            "scenario_broadcast_never_inherits_local_payload_authorization",
            true,
            "broadcast",
        );
        return;
    }
    // @step When a tool event with payload fields and credentials reaches RPC broadcast
    let (sender, mut receiver) = tokio::sync::broadcast::channel(32);
    let subscriber =
        tracing_subscriber::registry().with(codelet_rpc::BroadcastLogLayer::new(sender));
    tracing::subscriber::with_default(subscriber, || {
        let span = tracing::info_span!(
            "invoke_agent",
            session_id = "session-239",
            request_id = tracing::field::Empty,
            gen_ai.prompt = "PROMPT_PRIVATE"
        );
        span.record("request_id", "request-239");
        let _entered = span.enter();
        tracing::error!(
            gen_ai.tool.call.arguments = "ARGS_PRIVATE",
            authorization = "Bearer BEARER_SECRET",
            error = "transport failed: connection reset: peer closed",
            "tool failed: api_key=KEY_SECRET"
        );
    });
    let record = receiver.try_recv().expect("broadcast diagnostic");
    // @step Then broadcast omits payload and credential markers
    for marker in [
        "ARGS_PRIVATE",
        "PROMPT_PRIVATE",
        "KEY_SECRET",
        "BEARER_SECRET",
    ] {
        assert!(!record.message.contains(marker), "leaked {marker}");
    }
    // @step And broadcast retains correlation metadata and actionable errors
    for marker in [
        "session-239",
        "request-239",
        "transport failed: connection reset: peer closed",
    ] {
        assert!(record.message.contains(marker), "missing {marker}");
    }
}
