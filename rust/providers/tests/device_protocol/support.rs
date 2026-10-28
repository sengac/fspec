use base64::Engine;
use codelet_providers::codex::codex_device_auth::DeviceAuthConfig;
use codelet_providers::codex::codex_oauth::{urlencoded, CODEX_CLIENT_ID};
use serde_json::{json, Value};
use wiremock::matchers::{body_json, body_string_contains, header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

pub const DEVICE: &str = "SYNTHETIC-device-id";
pub const USER_CODE: &str = "SYNTHETIC-USER-CODE";
pub const SECRET: &str = "SYNTHETIC-SECRET-MUST-NOT-APPEAR";

pub fn usercode() -> Value {
    json!({"device_auth_id": DEVICE, "usercode": USER_CODE, "interval": "0"})
}

pub fn is_poll_request(request: &wiremock::Request) -> bool {
    request.method.as_str() == "POST"
        && request
            .headers
            .get("content-type")
            .is_some_and(|value| value == "application/json")
        && serde_json::from_slice::<Value>(&request.body).ok()
            == Some(json!({"device_auth_id": DEVICE, "user_code": USER_CODE}))
}

pub fn authorized() -> Value {
    json!({"authorization_code": "SYNTHETIC-auth-code",
        "code_verifier": "SYNTHETIC-verifier", "code_challenge": "SYNTHETIC-challenge"})
}

pub fn tokens() -> Value {
    let payload = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .encode(r#"{"chatgpt_account_id":"SYNTHETIC-account"}"#);
    json!({"id_token": format!("e30.{payload}.SYNTHETIC-signature"),
        "access_token": "SYNTHETIC-access", "refresh_token": "SYNTHETIC-refresh",
        "expires_in": 3600})
}

pub fn config(server: &MockServer, timeout_ms: u64) -> DeviceAuthConfig {
    DeviceAuthConfig {
        issuer_url: server.uri(),
        timeout_ms,
        poll_interval_override_ms: Some(5),
        slow_down_increment_override_ms: Some(5),
        display_fn: Some(Box::new(|_, _| {})),
    }
}

pub async fn official_start(server: &MockServer, response: Value) {
    Mock::given(method("POST"))
        .and(path("/api/accounts/deviceauth/usercode"))
        .respond_with(move |request: &wiremock::Request| {
            if request
                .headers
                .get("content-type")
                .is_some_and(|value| value == "application/json")
                && serde_json::from_slice::<Value>(&request.body).ok()
                    == Some(json!({"client_id": CODEX_CLIENT_ID}))
            {
                ResponseTemplate::new(200).set_body_json(&response)
            } else {
                ResponseTemplate::new(415)
            }
        })
        .mount(server)
        .await;
}

pub async fn compatible_start(server: &MockServer) {
    Mock::given(path("/api/accounts/deviceauth/usercode"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "device_auth_id": DEVICE, "user_code": USER_CODE, "interval": 0
        })))
        .mount(server)
        .await;
}

pub async fn official_poll(server: &MockServer) {
    Mock::given(method("POST"))
        .and(path("/api/accounts/deviceauth/token"))
        .and(header("content-type", "application/json"))
        .and(body_json(
            json!({"device_auth_id": DEVICE, "user_code": USER_CODE}),
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(authorized()))
        .mount(server)
        .await;
}

pub async fn official_exchange(server: &MockServer) {
    Mock::given(method("POST"))
        .and(path("/oauth/token"))
        .and(header("content-type", "application/x-www-form-urlencoded"))
        .and(body_string_contains("grant_type=authorization_code"))
        .and(body_string_contains("code=SYNTHETIC-auth-code"))
        .and(body_string_contains("code_verifier=SYNTHETIC-verifier"))
        .and(body_string_contains(format!("client_id={CODEX_CLIENT_ID}")))
        .and(body_string_contains(format!(
            "redirect_uri={}",
            urlencoded(&format!("{}/deviceauth/callback", server.uri()))
        )))
        .respond_with(ResponseTemplate::new(200).set_body_json(tokens()))
        .mount(server)
        .await;
}

pub async fn count(server: &MockServer, endpoint: &str) -> usize {
    server
        .received_requests()
        .await
        .unwrap_or_default()
        .iter()
        .filter(|request| request.url.path() == endpoint)
        .count()
}
