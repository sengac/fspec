use std::io::Write;
use std::sync::{Arc, Mutex};

use codelet_rpc_types::ProviderInfo;
use codelet_sessions::profile_sections::build_local_profile_sections;
use codelet_test_helpers::setup_temp_directory;
use serde_json::{json, Value};
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

pub type TestResult<ResultValue = ()> = Result<ResultValue, Box<dyn std::error::Error>>;
pub const PROFILE: &str = "credential-profile-secret";
pub const MODEL: &str = "explicit-model";

pub struct Fixture {
    pub directory: tempfile::TempDir,
    previous: Option<std::ffi::OsString>,
}

pub async fn infer(server: &MockServer) -> TestResult {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let mut stream = tokio::net::TcpStream::connect(server.address()).await?;
    let body = json!({"model": MODEL, "messages": []}).to_string();
    let request = format!("POST /private-endpoint/v1/chat/completions HTTP/1.1\r\nHost: {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", server.address(), body.len());
    stream.write_all(request.as_bytes()).await?;
    let mut response = String::new();
    tokio::time::timeout(
        std::time::Duration::from_secs(2),
        stream.read_to_string(&mut response),
    )
    .await??;
    assert!(response.starts_with("HTTP/1.1 200"));
    Ok(())
}

impl Fixture {
    pub fn new(base: &str, custom: Value) -> TestResult<Self> {
        let directory = setup_temp_directory()?;
        std::fs::write(
            directory.path().join("fspec-config.json"),
            json!({"providers": {"openai": {"profiles": {(PROFILE): {
                "baseUrl": base, "apiKey": "api-secret", "customModels": custom
            }}}}})
            .to_string(),
        )?;
        let previous = std::env::var_os("FSPEC_USER_DIR");
        std::env::set_var("FSPEC_USER_DIR", directory.path());
        Ok(Self {
            directory,
            previous,
        })
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        match &self.previous {
            Some(previous) => std::env::set_var("FSPEC_USER_DIR", previous),
            None => std::env::remove_var("FSPEC_USER_DIR"),
        }
    }
}

#[derive(Clone, Default)]
struct LogBuffer(Arc<Mutex<Vec<u8>>>);

impl Write for LogBuffer {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0
            .lock()
            .map_err(|_| std::io::Error::other("log lock"))?
            .extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

pub fn enumerate() -> TestResult<(ProviderInfo, String)> {
    let buffer = LogBuffer::default();
    let writer = buffer.clone();
    let subscriber = tracing_subscriber::fmt()
        .with_ansi(false)
        .without_time()
        .with_max_level(tracing::Level::INFO)
        .with_writer(move || writer.clone())
        .finish();
    let mut sections = tracing::subscriber::with_default(subscriber, build_local_profile_sections);
    assert_eq!(sections.len(), 1);
    let log = String::from_utf8(buffer.0.lock().map_err(|_| "log lock")?.clone())?;
    Ok((sections.remove(0), log))
}

pub async fn listing(server: &MockServer, response: ResponseTemplate, calls: u64) {
    Mock::given(method("GET"))
        .and(path("/private-endpoint/v1/models"))
        .and(header("Authorization", "Bearer api-secret"))
        .respond_with(response)
        .expect(calls)
        .mount(server)
        .await;
}

pub fn diagnostic(log: &str, discovery: &str) {
    assert!(log.contains(&format!("discovery=\"{discovery}\"")), "{log}");
    assert!(log.contains("inference=\"unknown\""), "{log}");
    for secret in ["api-secret", "payload-secret", "private-endpoint", PROFILE] {
        assert!(
            !log.contains(secret),
            "diagnostics must omit {secret}: {log}"
        );
    }
    assert!(!log.contains("marking profile unreachable"), "{log}");
}

pub fn blocked(info: &ProviderInfo) {
    assert!(info.is_unreachable);
    assert_eq!(info.models[0].id, MODEL);
    assert!(
        codelet_sessions::startup_model_resolution::resolve_startup_model(
            std::slice::from_ref(info),
            None
        )
        .is_none()
    );
}
