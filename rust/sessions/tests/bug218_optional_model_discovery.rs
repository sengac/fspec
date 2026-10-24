//! Feature: spec/features/optional-profile-model-discovery.feature

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};

use codelet_rpc_types::ProviderInfo;
use codelet_sessions::profile_sections::build_local_profile_sections;
use codelet_test_helpers::setup_temp_directory;
use serde_json::{json, Value};
use serial_test::serial;
use wiremock::matchers::{body_json, header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

type TestResult<ResultValue = ()> = Result<ResultValue, Box<dyn std::error::Error>>;

struct ProfileFixture {
    _directory: tempfile::TempDir,
    previous: Option<std::ffi::OsString>,
}

impl ProfileFixture {
    fn new(base: &str, custom: Value) -> TestResult<Self> {
        let directory = setup_temp_directory()?;
        std::fs::write(
            directory.path().join("fspec-config.json"),
            json!({"providers": {"openai": {"profiles": {"fixture": {
                "baseUrl": base, "apiKey": "fixture-key", "customModels": custom
            }}}}})
            .to_string(),
        )?;
        let previous = std::env::var_os("FSPEC_USER_DIR");
        std::env::set_var("FSPEC_USER_DIR", directory.path());
        Ok(Self {
            _directory: directory,
            previous,
        })
    }
}

impl Drop for ProfileFixture {
    fn drop(&mut self) {
        match &self.previous {
            Some(previous) => std::env::set_var("FSPEC_USER_DIR", previous),
            None => std::env::remove_var("FSPEC_USER_DIR"),
        }
    }
}

fn section() -> ProviderInfo {
    let mut sections = build_local_profile_sections();
    assert_eq!(sections.len(), 1);
    sections.remove(0)
}

async fn listing(server: &MockServer, response: ResponseTemplate) {
    Mock::given(method("GET"))
        .and(path("/proxy/v1/models"))
        .and(header("Authorization", "Bearer fixture-key"))
        .respond_with(response)
        .expect(1)
        .mount(server)
        .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial]
async fn scenario_optional_discovery_preserves_an_independently_usable_configured_model(
) -> TestResult {
    // @step Given a profile has a nonblank custom model and discovery returns HTTP 404 or 405
    for status in [404, 405] {
        let server = MockServer::start().await;
        listing(&server, ResponseTemplate::new(status)).await;
        let _fixture = ProfileFixture::new(
            &format!("{}/proxy/v1/", server.uri()),
            json!([{"id": "configured-model"}]),
        )?;

        // @step And the local chat endpoint independently accepts the configured model
        let body = json!({"model": "configured-model", "messages": []}).to_string();
        Mock::given(method("POST"))
            .and(path("/proxy/v1/chat/completions"))
            .and(body_json(serde_json::from_str::<Value>(&body)?))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"choices": []})))
            .expect(1)
            .mount(&server)
            .await;
        tokio::task::block_in_place(|| -> TestResult {
            let address = server.address();
            let mut socket = TcpStream::connect(address)?;
            socket.set_read_timeout(Some(std::time::Duration::from_secs(2)))?;
            write!(socket, "POST /proxy/v1/chat/completions HTTP/1.1\r\nHost: {address}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len())
                ?;
            let mut response = String::new();
            socket.read_to_string(&mut response)?;
            assert!(response.starts_with("HTTP/1.1 200"));
            Ok(())
        })?;

        // @step When the server builds the local profile sections
        let info = section();

        // @step Then the configured model remains present and the profile is not marked unreachable
        assert!(!info.is_unreachable);
        assert_eq!(info.models.len(), 1);
        assert_eq!(info.models[0].id, "configured-model");

        // @step And discovery does not call inference or try alternate listing routes
        assert_eq!(
            server
                .received_requests()
                .await
                .ok_or("capture requests")?
                .len(),
            2
        );
        server.verify().await;
    }
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial]
async fn scenario_authentication_and_server_failures_remain_visible_with_configured_models(
) -> TestResult {
    // @step Given a profile has a custom model and discovery returns HTTP 401 403 or 500 or malformed JSON
    for response in [
        ResponseTemplate::new(401),
        ResponseTemplate::new(403),
        ResponseTemplate::new(500),
        ResponseTemplate::new(200).set_body_string("not-json"),
    ] {
        let server = MockServer::start().await;
        listing(&server, response).await;
        let _fixture = ProfileFixture::new(
            &format!("{}/proxy", server.uri()),
            json!([{"id": "configured-model"}]),
        )?;

        // @step When the server builds the local profile sections
        let info = section();

        // @step Then the profile is marked unreachable and its custom model is retained
        assert!(info.is_unreachable);
        assert_eq!(info.models[0].id, "configured-model");
        server.verify().await;
    }
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial]
async fn scenario_transport_failures_remain_visible_with_configured_models() -> TestResult {
    // @step Given a profile has a custom model and its local endpoint refuses connections
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let address = listener.local_addr()?;
    drop(listener);
    let _fixture = ProfileFixture::new(
        &format!("http://{address}/proxy"),
        json!([{"id": "configured-model"}]),
    )?;

    // @step When the server builds the local profile sections
    let info = section();

    // @step Then the profile is marked unreachable and its custom model is retained
    assert!(info.is_unreachable);
    assert_eq!(info.models[0].id, "configured-model");
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial]
async fn scenario_unsupported_discovery_without_a_usable_configured_model_remains_unavailable(
) -> TestResult {
    // @step Given discovery returns HTTP 404 or 405 and custom models are absent or blank
    for status in [404, 405] {
        for custom in [json!([]), json!([{"id": ""}, {"id": "  "}])] {
            let server = MockServer::start().await;
            listing(&server, ResponseTemplate::new(status)).await;
            let _fixture = ProfileFixture::new(&format!("{}/proxy", server.uri()), custom)?;

            // @step When the server builds the local profile sections
            let info = section();

            // @step Then the profile remains present and is not marked unreachable
            assert_eq!(info.profile_name.as_deref(), Some("fixture"));
            assert!(!info.is_unreachable);

            // @step And blank model identifiers are not offered for selection
            assert!(info.models.is_empty());
            server.verify().await;
        }
    }
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial]
async fn scenario_successful_empty_discovery_does_not_invent_configured_models() -> TestResult {
    // @step Given a profile has no custom models and discovery returns an empty model list
    let server = MockServer::start().await;
    listing(
        &server,
        ResponseTemplate::new(200).set_body_json(json!({"data": []})),
    )
    .await;
    let _fixture = ProfileFixture::new(&format!("{}/proxy", server.uri()), json!([]))?;

    // @step When the server builds the local profile sections
    let info = section();

    // @step Then the profile remains present with no models and is not marked unreachable
    assert_eq!(info.profile_name.as_deref(), Some("fixture"));
    assert!(info.models.is_empty());
    assert!(!info.is_unreachable);
    server.verify().await;
    Ok(())
}

#[test]
#[serial]
fn scenario_a_configured_model_cannot_bypass_a_skipped_probe_without_a_runtime() -> TestResult {
    // @step Given a profile has a custom model and no tokio runtime is present
    let listener = TcpListener::bind("127.0.0.1:0")?;
    listener.set_nonblocking(true)?;
    let _fixture = ProfileFixture::new(
        &format!("http://{}/proxy", listener.local_addr()?),
        json!([{"id": "configured-model"}]),
    )?;

    // @step When the server builds the local profile sections
    let info = section();

    // @step Then the profile is marked unreachable without attempting an HTTP request
    assert!(info.is_unreachable);
    assert_eq!(
        listener
            .accept()
            .err()
            .ok_or("unexpected HTTP request")?
            .kind(),
        std::io::ErrorKind::WouldBlock
    );
    Ok(())
}

#[tokio::test]
#[serial]
async fn scenario_a_configured_model_cannot_bypass_a_skipped_probe_on_a_current_thread_runtime(
) -> TestResult {
    // @step Given a profile has a custom model on a current thread tokio runtime
    let server = MockServer::start().await;
    let _fixture = ProfileFixture::new(
        &format!("{}/proxy", server.uri()),
        json!([{"id": "configured-model"}]),
    )?;

    // @step When the server builds the local profile sections
    let info = section();

    // @step Then the profile is marked unreachable without attempting an HTTP request
    assert!(info.is_unreachable);
    assert!(server
        .received_requests()
        .await
        .ok_or("capture requests")?
        .is_empty());
    Ok(())
}
