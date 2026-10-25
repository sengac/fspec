use super::native::TestResult;
use codelet_tools::rlcd::{
    config::{load_rlcd_config, save_rlcd_url_at},
    security_config::RlcdSecurityConfig,
};
use serde_json::{json, Value};
use std::path::Path;
use wiremock::{
    matchers::{method, path},
    Mock, MockServer, ResponseTemplate,
};

pub async fn start(effect: &Path, denied: bool) -> TestResult<MockServer> {
    let directory = std::path::PathBuf::from(
        std::env::var_os("FSPEC_USER_DIR").ok_or("isolated user directory absent")?,
    );
    let home = std::path::PathBuf::from(std::env::var_os("HOME").ok_or("isolated HOME absent")?);
    assert_eq!(directory, home.join(".fspec"));
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/health"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({"status":"ready","model":"synthetic-security-fixture"})),
        )
        .mount(&server)
        .await;
    let expected = effect.display().to_string();
    Mock::given(method("POST")).and(path("/v1/classifier"))
        .respond_with(move |request: &wiremock::Request| {
            let body: Value = serde_json::from_slice(&request.body).unwrap_or(Value::Null);
            assert_eq!(body["model"], "synthetic-security-fixture");
            assert_eq!(body["questions"]["security"]["type"], "noul");
            let known = body["state"].as_str().is_some_and(|state| state.contains(&format!("path={expected}; operation=write")));
            let risk = if known && !denied { 0.01 } else { 0.99 };
            ResponseTemplate::new(200).set_body_json(json!({"model":"synthetic-security-fixture","answers":{"security":{"type":"noul","noul":risk}},"usage":{"input_tokens":1,"output_tokens":1}}))
        }).mount(&server).await;
    save_rlcd_url_at(&directory.join("fspec-config.json"), &server.uri())?;
    let config = load_rlcd_config();
    assert_eq!(config.url, server.uri());
    assert!(config.enabled);
    assert_eq!(config.security, RlcdSecurityConfig::default());
    Ok(server)
}
