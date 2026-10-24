//! Feature: spec/features/profile-discovery-classification.feature

#[path = "bug252/support.rs"]
mod support;

use std::net::TcpListener;
use std::time::Duration;

use codelet_sessions::last_used_model_persistence::{
    load_persisted_model_string_from, save_persisted_model_string_to,
};
use codelet_sessions::profile_sections::load_local_server_profiles;
use codelet_sessions::provider_settings::list_provider_settings_at;
use codelet_sessions::startup_model_resolution::resolve_startup_model;
use serde_json::json;
use serial_test::serial;
use support::{blocked, diagnostic, enumerate, listing, Fixture, TestResult, MODEL, PROFILE};
use wiremock::matchers::{body_partial_json, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial]
async fn scenario_unsupported_discovery_without_configured_models_does_not_claim_failed_inference(
) -> TestResult {
    // @step Given listing returns HTTP 404 or 405 with absent or blank configured models
    for status in [404, 405] {
        for custom in [json!([]), json!([{"id": ""}, {"id": "  "}])] {
            let server = MockServer::start().await;
            listing(&server, ResponseTemplate::new(status), 1).await;
            let _fixture = Fixture::new(&format!("{}/private-endpoint", server.uri()), custom)?;
            // @step When the profile list is enumerated
            let (info, log) = enumerate()?;
            // @step Then no models are offered and the profile is not marked unreachable
            assert!(!info.is_unreachable);
            assert!(info.models.is_empty());
            assert!(resolve_startup_model(&[info], None).is_none());
            // @step And diagnostics report unsupported discovery and unknown inference without secrets
            diagnostic(&log, "unsupported");
            server.verify().await;
            assert_eq!(server.received_requests().await.ok_or("requests")?.len(), 1);
        }
    }
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial]
async fn scenario_configured_model_survives_cold_startup_and_switching_with_unsupported_discovery(
) -> TestResult {
    // @step Given a persisted configured model has unsupported listing and independently working inference
    for status in [404, 405] {
        let server = MockServer::start().await;
        listing(&server, ResponseTemplate::new(status), 2).await;
        let fixture = Fixture::new(
            &format!("{}/private-endpoint", server.uri()),
            json!([{"id": MODEL}, {"id": "second-model"}]),
        )?;
        Mock::given(method("POST"))
            .and(path("/private-endpoint/v1/chat/completions"))
            .and(body_partial_json(json!({"model": MODEL})))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"choices": []})))
            .expect(1)
            .mount(&server)
            .await;
        support::infer(&server).await?;
        // @step When profiles are enumerated at cold startup and after switching the persisted selection
        for model in [MODEL, "second-model"] {
            let selected = format!("openai:{PROFILE}/{model}");
            save_persisted_model_string_to(fixture.directory.path(), &selected)?;
            let persisted = load_persisted_model_string_from(fixture.directory.path());
            let (info, log) = enumerate()?;
            // @step Then startup restores the configured selection and settings retain the custom model
            assert_eq!(
                resolve_startup_model(&[info], persisted.as_deref())
                    .ok_or("startup model")?
                    .model_string,
                selected
            );
            let settings =
                list_provider_settings_at(&fixture.directory.path().join("fspec-config.json"))?;
            assert!(settings
                .iter()
                .any(|provider| provider.provider_id == "openai"
                    && provider
                        .profiles
                        .iter()
                        .any(|profile| profile.name == PROFILE)));
            assert_eq!(load_local_server_profiles()[0].custom_models[0].id, MODEL);
            // @step And diagnostics report unsupported discovery and unknown inference without secrets
            diagnostic(&log, "unsupported");
        }
        server.verify().await;
        assert_eq!(server.received_requests().await.ok_or("requests")?.len(), 3);
    }
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial]
async fn scenario_successful_discovery_distinguishes_populated_and_empty_listings() -> TestResult {
    // @step Given listing returns valid populated or empty model data
    for (data, expected, state) in [
        (json!([]), 0, "empty"),
        (json!([{"id": MODEL}]), 1, "supported"),
    ] {
        let server = MockServer::start().await;
        listing(
            &server,
            ResponseTemplate::new(200).set_body_json(json!({"data": data})),
            1,
        )
        .await;
        let _fixture = Fixture::new(&format!("{}/private-endpoint", server.uri()), json!([]))?;
        // @step When the profile list is enumerated
        let (info, log) = enumerate()?;
        // @step Then only returned models are offered without invented capabilities
        assert!(!info.is_unreachable);
        assert_eq!(info.models.len(), expected);
        for model in info.models {
            assert_eq!(model.id, MODEL);
            assert!(!model.supports_reasoning && !model.supports_vision);
        }
        // @step And diagnostics distinguish supported and empty discovery with unknown inference
        diagnostic(&log, state);
        server.verify().await;
    }
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial]
async fn scenario_authentication_server_and_protocol_failures_block_selection_without_leaking_payloads(
) -> TestResult {
    // @step Given a configured model has authentication server or malformed listing failures
    for (response, category) in [
        (
            ResponseTemplate::new(401).set_body_string("payload-secret"),
            "authentication_failed",
        ),
        (ResponseTemplate::new(403), "authentication_failed"),
        (
            ResponseTemplate::new(500).set_body_string("payload-secret"),
            "http_failed",
        ),
        (
            ResponseTemplate::new(200).set_body_string("payload-secret"),
            "protocol_failed",
        ),
        (
            ResponseTemplate::new(200).set_body_json(json!({"error": "payload-secret"})),
            "protocol_failed",
        ),
        (
            ResponseTemplate::new(200).set_body_json(json!({"data": [{"id": 7}]})),
            "protocol_failed",
        ),
        (
            ResponseTemplate::new(200).set_body_json(json!({"data": [{"id": "  "}]})),
            "protocol_failed",
        ),
    ] {
        let server = MockServer::start().await;
        listing(&server, response, 1).await;
        let _fixture = Fixture::new(
            &format!("{}/private-endpoint", server.uri()),
            json!([{"id": MODEL}]),
        )?;
        // @step When the profile list is enumerated
        let (info, log) = enumerate()?;
        // @step Then the configured model is retained but unavailable for startup selection
        blocked(&info);
        // @step And diagnostics identify the failure category with unknown inference and no secrets
        diagnostic(&log, category);
        server.verify().await;
        assert_eq!(server.received_requests().await.ok_or("requests")?.len(), 1);
    }
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial]
async fn scenario_transport_failures_remain_visible_without_optimistic_fallback() -> TestResult {
    // @step Given a configured model endpoint refuses connections or times out
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let address = listener.local_addr()?;
    drop(listener);
    let server = MockServer::start().await;
    listing(
        &server,
        ResponseTemplate::new(200).set_delay(Duration::from_secs(6)),
        1,
    )
    .await;
    for base in [
        format!("http://{address}/private-endpoint"),
        format!("{}/private-endpoint", server.uri()),
    ] {
        let _fixture = Fixture::new(&base, json!([{"id": MODEL}]))?;
        // @step When the profile list is enumerated
        let (info, log) = enumerate()?;
        // @step Then the configured model is retained but unavailable for startup selection
        blocked(&info);
        // @step And diagnostics report transport failure and unknown inference without secrets
        diagnostic(&log, "transport_failed");
    }
    server.verify().await;
    Ok(())
}

#[test]
#[serial]
fn scenario_skipped_probes_cannot_authorize_configured_models() -> TestResult {
    // @step Given a configured profile is enumerated outside a Tokio runtime
    let listener = TcpListener::bind("127.0.0.1:0")?;
    listener.set_nonblocking(true)?;
    let _fixture = Fixture::new(
        &format!("http://{}/private-endpoint", listener.local_addr()?),
        json!([{"id": MODEL}]),
    )?;
    // @step When the profile list is enumerated
    let (info, log) = enumerate()?;
    // @step Then the configured model is retained but unavailable for startup selection
    blocked(&info);
    assert_eq!(
        listener
            .accept()
            .err()
            .ok_or("no requests expected")?
            .kind(),
        std::io::ErrorKind::WouldBlock
    );
    // @step And diagnostics report skipped discovery and unknown inference without secrets
    diagnostic(&log, "skipped");
    Ok(())
}
