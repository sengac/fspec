//! Feature: spec/features/privacy-diagnostic-header-and-provider-errors.feature

#[allow(dead_code)]
#[path = "../src/common.rs"]
mod common;
#[path = "support/bug239_logging.rs"]
mod logging;
#[path = "support/bug239_provider_failure.rs"]
mod provider;

use logging::{child, run_child};

fn emit() {
    tracing::error!(status = 401, request_id = "headers-239",
        error = "transport rejected credentials\nAuthorization: Basic dXNlcjpwYXNzd29yZA==\nCookie: sid=abc; private_session=xyz\nroot cause: access denied",
        "header failure");
    tracing::error!(
        error = "Cookie: sid=\"abc\"; private_session=xyz\nroot cause: access denied",
        "quoted cookie pair failure"
    );
    tracing::warn!(
        error = r#"headers={"Authorization":"Basic dXNlcjpwYXNzd29yZA==","Cookie":"sid=abc; private_session=xyz"} root cause: access denied"#,
        "quoted header failure"
    );
    tracing::error!(
        error = %serde_json::json!({"error": {"message": "transport failed\nAuthorization: Basic dXNlcjpwYXNzd29yZA==\nroot cause: access denied"}, "status": 401}),
        "json header failure"
    );
    tracing::error!(
        error = %serde_json::json!({"error": {"message": "quota exceeded"},
            "system": format!("ANTHROPIC_SYSTEM_PRIVATE{}INSTRUCTION_TAIL_PRIVATE", "界".repeat(5000)),
            "instructions": format!("RESPONSES_SYSTEM_PRIVATE{}INSTRUCTION_TAIL_PRIVATE", "界".repeat(5000))}),
        "known instruction failure"
    );
}

#[tokio::test]
async fn scenario_embedded_credential_headers_redact_complete_values_in_every_privacy_mode() {
    // @step Given default and explicit local payload diagnostic modes
    if run_child().await {
        return;
    }
    for payloads in [false, true] {
        for mode in ["client", "combined", "daemon", "common"] {
            // @step When errors contain Basic authorization and cookies with multiple session pairs
            let (logs, _home) = child(
                "scenario_embedded_credential_headers_redact_complete_values_in_every_privacy_mode",
                payloads,
                mode,
            );
            // @step Then neither mode exposes the encoded credential or any cookie pair
            for marker in ["dXNlcjpwYXNzd29yZA==", "sid=abc", "private_session=xyz"] {
                assert!(
                    !logs.contains(marker),
                    "{mode} payloads={payloads} leaked {marker}"
                );
            }
            // @step And the surrounding status and actionable root cause remain readable
            for marker in ["status=401", "headers-239", "root cause: access denied"] {
                assert!(logs.contains(marker), "{mode} lost {marker}");
            }
        }
    }
}

#[tokio::test]
async fn scenario_application_provider_failures_do_not_expose_serialized_conversation_bodies(
) -> provider::TestResult {
    // @step Given a local provider returns an error body containing conversation payloads
    if std::env::var_os("BUG239_CHILD").is_some() {
        run_child().await;
        provider::fail_in_real_stream_loop(false).await?;
        tokio::time::sleep(std::time::Duration::from_millis(300)).await;
        return Ok(());
    }
    // @step When the real application stream loop reports the provider failure
    for payloads in [false, true] {
        let (logs, _home) = child(
            "scenario_application_provider_failures_do_not_expose_serialized_conversation_bodies",
            payloads,
            "client",
        );
        assert!(
            logs.contains("codelet_cli::interactive::stream_loop"),
            "real application target missing"
        );
        assert!(
            logs.contains("API error received from provider"),
            "terminal provider emitter not exercised"
        );
        // @step Then default diagnostics omit the serialized conversation markers
        for marker in [
            "APP_SYSTEM_PRIVATE",
            "APP_PROMPT_PRIVATE",
            "APP_COMPLETION_PRIVATE",
            "APP_ARGS_PRIVATE",
            "APP_RESULT_PRIVATE",
        ] {
            assert_eq!(
                logs.contains(marker),
                payloads,
                "payloads={payloads} marker={marker}"
            );
        }
        // @step And the provider error message and application correlation remain readable
        for marker in [
            "quota exceeded",
            "provider-fixture-239",
            "request-fixture-239",
        ] {
            assert!(
                logs.contains(marker),
                "missing actionable metadata {marker}"
            );
        }
    }
    Ok(())
}

#[tokio::test]
async fn scenario_streaming_provider_http_failures_do_not_expose_debug_wrapped_conversation_bodies(
) -> provider::TestResult {
    // @step Given a streaming provider returns an HTTP failure with conversation JSON
    if std::env::var_os("BUG239_CHILD").is_some() {
        run_child().await;
        provider::fail_in_real_stream_loop(true).await?;
        tokio::time::sleep(std::time::Duration::from_millis(300)).await;
        return Ok(());
    }
    // @step When the real provider adapter emits the streaming error through diagnostic sinks
    for payloads in [false, true] {
        for mode in ["client", "combined", "daemon", "common"] {
            let (logs, _home) = child(
                "scenario_streaming_provider_http_failures_do_not_expose_debug_wrapped_conversation_bodies",
                payloads,
                mode,
            );
            let record = logs
                .lines()
                .find(|line| {
                    line.contains("rig::providers::openai::completion::streaming")
                        && line.contains("SSE error")
                })
                .expect("actual streaming provider error record");
            // @step Then default sinks omit conversation markers from the debug wrapped body
            for marker in [
                "APP_SYSTEM_PRIVATE",
                "APP_PROMPT_PRIVATE",
                "APP_COMPLETION_PRIVATE",
                "APP_ARGS_PRIVATE",
                "APP_RESULT_PRIVATE",
            ] {
                assert_eq!(
                    record.contains(marker),
                    payloads && matches!(mode, "client" | "combined"),
                    "{mode} payloads={payloads} marker={marker}"
                );
            }
            for marker in ["\u{200b}", "\u{e0001}", r"\\\\u{200b}"] {
                assert_eq!(
                    record.contains(marker),
                    payloads && matches!(mode, "client" | "combined"),
                    "{mode} payloads={payloads} lost Unicode or escaped content"
                );
            }
            // @step And status correlation and the provider root cause remain readable
            for marker in [
                "400",
                "quota exceeded",
                "provider-fixture-239",
                "request-fixture-239",
            ] {
                assert!(record.contains(marker), "{mode} lost {marker}");
            }
        }
    }
    Ok(())
}

#[tokio::test]
async fn scenario_known_provider_instruction_fields_follow_payload_authorization_and_bounds() {
    // @step Given diagnostics contain Anthropic system and Responses instructions fields
    if run_child().await {
        return;
    }
    // @step When default and explicit local payload diagnostic modes format the JSON
    for payloads in [false, true] {
        for mode in ["client", "combined", "daemon", "common"] {
            let (logs, _home) = child(
                "scenario_known_provider_instruction_fields_follow_payload_authorization_and_bounds",
                payloads,
                mode,
            );
            let record = logs
                .lines()
                .find(|line| line.contains("known instruction failure"))
                .expect("instruction diagnostic");
            // @step Then default logs omit both instruction markers and local opt in bounds each field
            let authorized = payloads && matches!(mode, "client" | "combined");
            for marker in ["ANTHROPIC_SYSTEM_PRIVATE", "RESPONSES_SYSTEM_PRIVATE"] {
                assert_eq!(
                    record.contains(marker),
                    authorized,
                    "{mode} payloads={payloads} marker={marker}"
                );
            }
            assert!(
                !record.contains("INSTRUCTION_TAIL_PRIVATE"),
                "instruction field must be bounded"
            );
            assert_eq!(
                record.matches("[TRUNCATED]").count(),
                if authorized { 2 } else { 0 }
            );
            if authorized {
                let (_, body) = record.split_once("error=").expect("JSON error field");
                let body: serde_json::Value =
                    serde_json::from_str(body).expect("valid diagnostic JSON");
                for name in ["system", "instructions"] {
                    let value = body[name].as_str().expect("bounded instruction field");
                    assert_eq!(value.chars().count(), 4096 + "[TRUNCATED]".len());
                }
            }
            // @step And actionable error metadata remains readable
            assert!(record.contains("quota exceeded"));
        }
    }
}

#[tokio::test]
async fn scenario_credential_headers_inside_json_preserve_trailing_error_metadata() {
    // @step Given a JSON error string contains a credential header between newline separated causes
    if run_child().await {
        return;
    }
    // @step When default and explicit local payload diagnostic modes format the error
    for payloads in [false, true] {
        for mode in ["client", "combined", "daemon", "common"] {
            let (logs, _home) = child(
                "scenario_credential_headers_inside_json_preserve_trailing_error_metadata",
                payloads,
                mode,
            );
            let record = logs
                .lines()
                .find(|line| line.contains("json header failure"))
                .expect("JSON header diagnostic");
            // @step Then credentials are absent from the JSON diagnostic record
            assert!(!record.contains("dXNlcjpwYXNzd29yZA=="));
            // @step And the same record retains the final root cause and status
            for marker in ["root cause: access denied", "401"] {
                assert!(
                    record.contains(marker),
                    "{mode} payloads={payloads} lost {marker}"
                );
            }
        }
    }
}
