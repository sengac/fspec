//! Feature: spec/features/codex-auxiliary-request-admission.feature

use codelet_agent_loop::{
    deep_search_handler::execute_deep_search,
    generate_compaction_handler::execute_compaction_subagent,
};
use codelet_test_helpers::setup_temp_directory;
use serial_test::serial;
use std::{
    ffi::OsString,
    path::Path,
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc,
    },
    time::Duration,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

type TestResult = Result<(), Box<dyn std::error::Error>>;

struct Fixture {
    directory: tempfile::TempDir,
    environment: Vec<(&'static str, Option<OsString>)>,
    connections: Arc<AtomicUsize>,
    proxy: tokio::task::JoinHandle<()>,
}

impl Fixture {
    async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let directory = setup_temp_directory()?;
        std::fs::write(
            directory.path().join("auth.json"),
            r#"{"OPENAI_API_KEY":"sk-auxiliary-test-only"}"#,
        )?;
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
        let address = format!("http://{}", listener.local_addr()?);
        let connections = Arc::new(AtomicUsize::new(0));
        let observed = connections.clone();
        let proxy = tokio::spawn(async move {
            while let Ok((mut socket, _)) = listener.accept().await {
                observed.fetch_add(1, Ordering::SeqCst);
                let mut buffer = [0; 4096];
                let _ = socket.read(&mut buffer).await;
                let _ = socket
                    .write_all(b"HTTP/1.1 403 Forbidden\r\nContent-Length: 0\r\n\r\n")
                    .await;
            }
        });
        let mut environment = Vec::new();
        for (name, value) in [
            ("CODEX_HOME", directory.path().as_os_str().to_os_string()),
            ("CODEX_MODEL", OsString::from("auxiliary-unknown-model")),
            ("HTTPS_PROXY", OsString::from(&address)),
            ("https_proxy", OsString::from(&address)),
            ("ALL_PROXY", OsString::from(&address)),
            ("all_proxy", OsString::from(&address)),
            ("NO_PROXY", OsString::new()),
            ("no_proxy", OsString::new()),
        ] {
            environment.push((name, std::env::var_os(name)));
            std::env::set_var(name, value);
        }
        Ok(Self {
            directory,
            environment,
            connections,
            proxy,
        })
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.proxy.abort();
        for (name, value) in &self.environment {
            match value {
                Some(value) => std::env::set_var(name, value),
                None => std::env::remove_var(name),
            }
        }
    }
}

async fn run_auxiliary(
    compaction: bool,
    project: &Path,
    capacity: Option<usize>,
    output: Option<usize>,
) -> Result<String, String> {
    let request = async {
        if compaction {
            codelet_common::set_data_directory(project.to_path_buf())?;
            codelet_core::persistence::reset_stores_for_tests();
            let manifest = codelet_core::persistence::create_session_with_provider(
                "auxiliary admission",
                project,
                "codex",
            )?;
            execute_compaction_subagent(
                project,
                manifest.id,
                None,
                &Arc::new(AtomicBool::new(false)),
                "codex",
                Some("auxiliary-unknown-model"),
                capacity,
                output,
            )
            .await
        } else {
            execute_deep_search(
                project,
                "Inspect the project without changing files",
                None,
                1,
                "codex",
                Some("auxiliary-unknown-model"),
                0,
                0,
                capacity,
                output,
            )
            .await
        }
    };
    tokio::time::timeout(Duration::from_secs(5), request)
        .await
        .map_err(|_| "auxiliary request failed to stop before transport".to_string())?
}

async fn missing_limits(compaction: bool) -> TestResult {
    let fixture = Fixture::new().await?;
    for (capacity, output) in [(None, None), (Some(100_000), None), (None, Some(100))] {
        let result = run_auxiliary(compaction, fixture.directory.path(), capacity, output).await;
        let error = result
            .err()
            .ok_or("missing limits unexpectedly completed")?;
        assert!(
            error.contains("request admission requires an explicit positive"),
            "missing limits did not produce admission failure: {error}"
        );
        assert_eq!(fixture.connections.load(Ordering::SeqCst), 0);
    }
    Ok(())
}

async fn oversized_request(compaction: bool) -> TestResult {
    let fixture = Fixture::new().await?;
    let result = run_auxiliary(compaction, fixture.directory.path(), Some(200), Some(100)).await;
    let error = result
        .err()
        .ok_or("oversized request unexpectedly completed")?;
    assert!(
        error.contains("complete request input") && error.contains("exceeds capacity 200"),
        "oversized request did not produce capacity failure: {error}"
    );
    assert_eq!(fixture.connections.load(Ordering::SeqCst), 0);
    Ok(())
}

#[tokio::test]
#[serial]
async fn deep_search_rejects_missing_codex_limits_before_transport() -> TestResult {
    missing_limits(false).await
}

#[tokio::test]
#[serial]
async fn compaction_rejects_missing_codex_limits_before_transport() -> TestResult {
    missing_limits(true).await
}

#[tokio::test]
#[serial]
async fn deep_search_rejects_an_oversized_codex_request_before_transport() -> TestResult {
    oversized_request(false).await
}

#[tokio::test]
#[serial]
async fn compaction_rejects_an_oversized_codex_request_before_transport() -> TestResult {
    oversized_request(true).await
}
