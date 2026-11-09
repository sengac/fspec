//! Feature: spec/features/anthropic-client-browser-authorization.feature

use codelet_providers::claude_oauth_server::ClaudeBrowserAuthorization;
use codelet_test_helpers::{OrFail, TestResult};
use std::io::Write;
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tracing::instrument::WithSubscriber;

#[derive(Clone)]
struct LogWriter(Arc<Mutex<Vec<u8>>>);

impl Write for LogWriter {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0
            .lock()
            .map_err(|error| std::io::Error::other(error.to_string()))?
            .extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[tokio::test]
async fn failed_http_connections_do_not_terminate_authorization() -> TestResult {
    for request in [
        b"GET / HTTP/1.1\r\nHost:".as_slice(),
        b"INVALID HTTP REQUEST\r\n\r\n",
    ] {
        // @step Given a client-local Anthropic authorization listener
        let flow = ClaudeBrowserAuthorization::bind().await?;
        let browser_url = flow.browser_url().to_owned();
        let state = url::Url::parse(flow.authorize_url())?
            .query_pairs()
            .find(|(name, _)| name == "state")
            .or_fail("state parameter")?
            .1
            .into_owned();
        let log = Arc::new(Mutex::new(Vec::new()));
        let writer = LogWriter(Arc::clone(&log));
        let subscriber = tracing_subscriber::fmt()
            .without_time()
            .with_ansi(false)
            .with_max_level(tracing::Level::ERROR)
            .with_writer(move || writer.clone())
            .finish();
        let completion = tokio::spawn(flow.complete().with_subscriber(subscriber));
        // @step When an incomplete or malformed HTTP request disconnects
        let address = browser_url
            .trim_start_matches("http://")
            .trim_end_matches('/');
        let mut connection = tokio::net::TcpStream::connect(address).await?;
        connection.write_all(request).await?;
        connection.shutdown().await?;
        let mut response = Vec::new();
        tokio::time::timeout(
            std::time::Duration::from_secs(2),
            connection.read_to_end(&mut response),
        )
        .await??;
        // @step Then the failed connection is rejected and its real error is logged
        tokio::task::yield_now().await;
        assert!(
            !completion.is_finished(),
            "failed connection must not terminate authorization"
        );
        let logs = String::from_utf8(
            log.lock()
                .map_err(|error| std::io::Error::other(error.to_string()))?
                .clone(),
        )?;
        assert!(
            logs.contains("Rejected Claude OAuth HTTP connection"),
            "{logs}"
        );
        assert!(
            logs.contains("connection closed before message completed") || logs.contains("invalid"),
            "real HTTP cause: {logs}"
        );
        assert!(!String::from_utf8_lossy(&response).contains("200 OK"));
        // @step And a subsequent browser GET and valid code submission complete authorization
        let client = reqwest::Client::new();
        let form = client.get(&browser_url).send().await?;
        assert!(form.status().is_success());
        assert!(form.text().await?.contains("Submit Code"));
        let raw_code = format!("valid-code#{state}");
        let response = client
            .post(format!("{browser_url}submit"))
            .form(&[("code", &raw_code)])
            .send()
            .await?;
        assert!(response.status().is_success());
        assert_eq!(completion.await??, (raw_code, state));
    }
    Ok(())
}
