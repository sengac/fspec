//! Feature: spec/features/codex-rejection-diagnostics.feature
#[path = "fixtures/cmpct076.rs"]
mod fixture;

use codelet_providers::codex::diagnostics::{EvidenceState, RejectionClass, MAX_DIAGNOSTIC_BYTES};
use codelet_providers::codex::refreshing_client::RefreshingCodexClient;
use fixture::{client_error, diagnostic, model_diagnostic, model_error, TestResult};
use rig::completion::CompletionError;
use rig::http_client::{Error, HttpClientExt};
use serde_json::json;
use std::time::Duration;
use wiremock::matchers::method;
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn scenario_http_context_rejection_retains_typed_evidence_through_the_model_stream(
) -> TestResult {
    // @step Given Codex HTTP rejection responses with declared context codes
    for status in [400, 413] {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(status)
                .insert_header("x-request-id", "request-context")
                .set_body_json(json!({"error":{"code":"context_length_exceeded","message":"reduce input"}})))
            .expect(1).mount(&server).await;
        // @step When the real model consumes each rejected stream
        let error = model_error(&server.uri(), Duration::from_secs(1)).await?;
        // @step Then status code message and request identity survive in a typed error without success or replay
        assert!(matches!(error, CompletionError::HttpError(_)), "{error}");
        let rejection = model_diagnostic(&error)?;
        assert_eq!(rejection.status, Some(status));
        assert_eq!(rejection.class, RejectionClass::ContextLengthExceeded);
        assert_eq!(rejection.state, EvidenceState::Complete);
        assert_eq!(rejection.code.as_deref(), Some("context_length_exceeded"));
        assert_eq!(rejection.attempts, Some(1));
        let message = error.to_string();
        for expected in [
            status.to_string().as_str(),
            "context_length_exceeded",
            "reduce input",
            "request-context",
        ] {
            assert!(message.contains(expected), "{message}");
        }
        server.verify().await;
    }
    Ok(())
}

#[tokio::test]
async fn scenario_sensitive_header_credentials_stay_private_throughout_rejection_evidence(
) -> TestResult {
    // @step Given credentials in opaque proxy and explicitly sensitive request headers
    for (name, secret, sensitive) in [
        ("x-api-key", "ascii-credential", false),
        ("authorization", "opaque-é-credential", false),
        ("proxy-authorization", "proxy-credential", false),
        ("x-private-token", "marked-sensitive-credential", true),
    ] {
        let server = MockServer::start().await;
        let escaped: String = secret
            .chars()
            .map(|character| format!("\\u{:04x}", u32::from(character)))
            .collect();
        Mock::given(method("POST"))
            .respond_with(
                ResponseTemplate::new(400)
                    .insert_header("x-request-id", escaped.as_str())
                    .insert_header("retry-after", escaped.as_str())
                    .set_body_json(json!({"error":{"code":"bad_request","message":secret}})),
            )
            .expect(1)
            .mount(&server)
            .await;
        let padded = format!(" \t{secret}\t ");
        let mut value = http::HeaderValue::from_bytes(padded.as_bytes())?;
        value.set_sensitive(sensitive);
        let request = http::Request::builder()
            .method("POST")
            .uri(server.uri())
            .header(name, value)
            .body(b"{}".to_vec())?;
        // @step When a rejection echoes those credentials in its body and allowlisted metadata
        let result = RefreshingCodexClient::new_api_key()
            .send_streaming(request)
            .await;
        let error = result.err().ok_or("rejection unexpectedly succeeded")?;
        let rejection = diagnostic(&error)?;
        // @step Then neither typed fields nor formatted diagnostics disclose the credential echoes
        assert_eq!(rejection.message.as_deref(), Some("[redacted]"), "{name}");
        assert_eq!(
            rejection.request_id.as_deref(),
            Some("[redacted]"),
            "{name}"
        );
        assert_eq!(
            rejection.retry_after.as_deref(),
            Some("[redacted]"),
            "{name}"
        );
        let evidence = format!("{error} {error:?}");
        assert!(!evidence.contains(secret), "{name}");
        assert!(!evidence.contains(&escaped), "{name}");
        server.verify().await;
    }
    Ok(())
}

#[tokio::test]
async fn scenario_a_broken_rejection_body_preserves_typed_unknown_evidence() -> TestResult {
    // @step Given a context rejection body followed by a transport read failure
    let server = fixture::StalledRejection::broken().await?;
    // @step When the real model consumes the broken rejection
    let error = tokio::time::timeout(
        Duration::from_secs(2),
        model_error(&server.url, Duration::from_secs(1)),
    )
    .await??;
    // @step Then status identity and bounded evidence survive but incomplete evidence cannot classify overflow
    let rejection = model_diagnostic(&error)?;
    assert_eq!(rejection.status, Some(400));
    assert_eq!(rejection.request_id.as_deref(), Some("broken-request"));
    assert_eq!(rejection.state, EvidenceState::ReadFailed);
    assert_eq!(rejection.class, RejectionClass::Unknown);
    assert_eq!(rejection.code, None);
    assert!(rejection.body.contains("context_length_exceeded"));
    assert!(rejection.body.len() <= MAX_DIAGNOSTIC_BYTES);
    Ok(())
}

#[tokio::test]
async fn scenario_unrelated_and_malformed_rejection_evidence_never_implies_context_overflow(
) -> TestResult {
    // @step Given unrelated malformed and oversized Codex rejection bodies
    for (body, class, state) in [
        (json!({"error":{"code":"invalid_request","message":"token context is not a recognized option"}}).to_string(), RejectionClass::Other, EvidenceState::Complete),
        ("not JSON: context length exceeded".to_owned(), RejectionClass::Unknown, EvidenceState::Complete),
        ("invalid rejection\n\u{1b}]52;c;clipboard-injection\u{7}".to_owned(), RejectionClass::Unknown, EvidenceState::Complete),
        (format!("{}{}", json!({"error":{"code":"context_length_exceeded","message":"untrusted prefix"}}), " ".repeat(20_000)), RejectionClass::Unknown, EvidenceState::Truncated),
        (json!({"error":{"code":17,"message":"context_length_exceeded"}}).to_string(), RejectionClass::Unknown, EvidenceState::Complete),
    ] {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(413).set_body_string(body))
            .expect(1).mount(&server).await;
        // @step When the real streaming client receives each rejection
        let error = client_error(&server, false).await?;
        // @step Then diagnostics remain bounded and only complete declared codes classify the failure
        assert!(matches!(error, Error::Instance(_)), "{error}");
        let rejection = diagnostic(&error)?;
        assert_eq!(rejection.class, class);
        assert_eq!(rejection.state, state);
        assert!(rejection.body.len() <= MAX_DIAGNOSTIC_BYTES);
        assert!(error.to_string().len() < 18_000);
        // @step And rendered errors cannot execute terminal control sequences
        assert!(!error.to_string().chars().any(char::is_control));
        server.verify().await;
    }
    Ok(())
}

#[tokio::test]
async fn scenario_exhausted_setup_retries_preserve_final_rejection_evidence() -> TestResult {
    // @step Given retryable Codex rejection responses with retry advice
    for status in [429, 503] {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(
                ResponseTemplate::new(status)
                    .insert_header("retry-after", "0")
                    .insert_header("x-request-id", "request-final")
                    .set_body_json(json!({"error":{"code":"service_busy","message":"try later"}})),
            )
            .expect(3)
            .mount(&server)
            .await;
        // @step When the declared setup retry budget is exhausted
        let error = client_error(&server, false).await?;
        // @step Then the final typed error retains status retry advice identity and attempt count
        assert!(matches!(error, Error::Instance(_)), "{error}");
        assert!(error.to_string().contains("request-final"));
        assert!(error.to_string().contains("service_busy"));
        let rejection = diagnostic(&error)?;
        assert_eq!(rejection.status, Some(status));
        assert_eq!(rejection.retry_after.as_deref(), Some("0"));
        assert_eq!(rejection.request_id.as_deref(), Some("request-final"));
        assert_eq!(rejection.attempts, Some(3));
        assert_eq!(rejection.class, RejectionClass::Other);
        server.verify().await;
    }
    Ok(())
}

#[tokio::test]
async fn scenario_sse_failures_retain_typed_bounded_diagnostics_without_completion() -> TestResult {
    // @step Given failed and top-level Codex SSE error events
    for (code, class) in [
        (
            json!("context_length_exceeded"),
            RejectionClass::ContextLengthExceeded,
        ),
        (json!("bad_request"), RejectionClass::Other),
        (json!(17), RejectionClass::Unknown),
    ] {
        for terminal in [false, true] {
            let server = MockServer::start().await;
            let failure =
                json!({"code":code,"message":format!("Bearer test-key {}", "x".repeat(20_000))});
            let event = if terminal {
                json!({"type":"response.failed","response":{"id":"resp-failed","status":"failed","output":[],"error":failure}})
            } else {
                json!({"type":"error","error":failure})
            };
            Mock::given(method("POST"))
                .respond_with(
                    ResponseTemplate::new(200)
                        .insert_header("content-type", "text/event-stream")
                        .set_body_string(format!("data: {event}\n\n")),
                )
                .expect(1)
                .mount(&server)
                .await;
            // @step When the real model consumes each terminal failure
            let error = model_error(&server.uri(), Duration::from_secs(1)).await?;
            // @step Then declared codes and bounded messages survive without completion or tool dispatch
            assert!(
                matches!(error, CompletionError::HttpError(_)),
                "expected typed backend diagnostic"
            );
            assert!(error.to_string().len() < 18_000);
            let rejection = model_diagnostic(&error)?;
            assert_eq!(rejection.code.as_deref(), code.as_str());
            assert_eq!(rejection.class, class);
            assert_eq!(rejection.status, None);
            assert_eq!(rejection.attempts, None);
            assert!(!format!("{error:?}").contains("test-key"));
            server.verify().await;
        }
    }
    Ok(())
}

#[tokio::test]
async fn scenario_rejection_diagnostics_do_not_disclose_credentials_or_private_headers(
) -> TestResult {
    // @step Given a rejection echoing request credentials and private response headers
    for body in [
        json!({"error":{"code":"bad_request","message":"Bearer secret-credential private-account cookie-secret quoted-secret"}}).to_string(),
        r#"{"error":{"code":"bad_request","message":"\u0073ecret-credential \u0063ookie-secret \u0071uoted-secret"}}"#.to_owned(),
        r#"malformed "\u0073ecret-credential \u0063ookie-secret \u0071uoted-secret""#.to_owned(),
        r#"{"error":{"message":"secret-cred"#.to_owned(),
        format!("{}{}", " ".repeat(MAX_DIAGNOSTIC_BYTES - 21), r#""\u0073ecret-cred\u0065ntial""#),
        format!("{}{}", " ".repeat(MAX_DIAGNOSTIC_BYTES - 20), r#""\u0073ecret-credential""#),
    ] {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(400)
                .insert_header("set-cookie", "private-cookie")
                .set_body_string(body))
            .expect(1).mount(&server).await;
        // @step When the real streaming client captures diagnostic evidence
        let error = client_error(&server, true).await?;
        // @step Then credential values are redacted and private headers are absent
        assert!(matches!(error, Error::Instance(_)), "{error}");
        let evidence = format!("{error} {error:?}");
        assert!(evidence.contains("[redacted]"));
        for secret in ["secret-credential", "private-account", "private-cookie", "cookie-secret", "quoted-secret", "ecret-credential", "ookie-secret", "uoted-secret", "secret-cred"] {
            assert!(!evidence.contains(secret), "credential echo was retained");
        }
        server.verify().await;
    }
    Ok(())
}

#[tokio::test]
async fn scenario_a_stalled_rejection_body_terminates_within_its_diagnostic_deadline() -> TestResult
{
    // @step Given an HTTP rejection whose body never finishes
    let server = fixture::StalledRejection::start().await?;
    // @step When the real model consumes the rejected stream
    let error = tokio::time::timeout(
        Duration::from_secs(2),
        model_error(&server.url, Duration::from_millis(100)),
    )
    .await??;
    // @step Then typed incomplete evidence returns before the harness deadline without replay
    assert!(matches!(error, CompletionError::HttpError(_)), "{error}");
    assert!(error.to_string().contains("DeadlineExceeded"));
    let rejection = model_diagnostic(&error)?;
    assert_eq!(rejection.state, EvidenceState::DeadlineExceeded);
    assert_eq!(rejection.class, RejectionClass::Unknown);
    assert_eq!(rejection.status, Some(400));
    Ok(())
}
