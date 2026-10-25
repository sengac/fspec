pub const SCRIPT: &str = r#"
fn build_url(config) { config.base_url + "/inference" }
fn stored(config) {
    let response = http::get(config.base_url + "/stored", #{});
    json::parse(response.body)
}

fn build_headers(config) {
    let tokens = stored(config);
    #{ "Content-Type": "application/json", "Authorization": "Bearer " + tokens.access_token }
}
fn build_request(request) { #{ messages: request.messages, tools: request.tools } }
fn build_stream_request(request) { #{ messages: request.messages, tools: request.tools, stream: true } }
fn parse_response(raw) { #{ content: "settled", stop_reason: "end_turn", usage: #{ input_tokens: 7, output_tokens: 3 } } }
fn parse_stream_chunk(config, data) {
    if data == "text" { return #{ kind: "text_delta", text: "settled" }; }
    if data == "usage" { return #{ kind: "usage", input_tokens: 7, output_tokens: 3 }; }
    if data == "stop" { return #{ kind: "stop", reason: "end_turn" }; }
    #{ kind: "ignore" }
}
fn map_error(status, body) { #{ type: "api", message: "synthetic inference rejected" } }
fn auth_needs_refresh(tokens) { tokens.expires_at == 0 }
fn auth_refresh(config, tokens) {
    let response = http::post(config.token_url, json::stringify(tokens), #{ "Content-Type": "application/json" });
    json::parse(response.body)
}
"#;

pub fn legacy() -> String {
    SCRIPT
        .replace("auth_needs_refresh", "needs_refresh")
        .replace("auth_refresh", "refresh_token")
}

pub fn headers_owned() -> String {
    SCRIPT.replace(
        "let tokens = stored(config);",
        "let tokens = stored(config); if tokens.expires_at == 0 { let ignored = http::post(config.base_url + \"/owned\", json::stringify(tokens), #{}); tokens = stored(config); }",
    )
}

pub fn owned_provider(
    root: &std::path::Path,
    server: &wiremock::MockServer,
    source: &str,
    mode: &str,
) -> super::environment::TestResult<codelet_providers::custom::RhaiCustomProvider> {
    use codelet_providers::custom::{AuthConfig, RhaiCustomProvider, ScriptLoader};
    use std::sync::Arc;
    let mut config = (*super::support::config(root, server, source, mode == "custom")?).clone();
    match mode {
        "bearer" => {
            config.auth = AuthConfig::Bearer {
                env_var: "SYNTHETIC_UNUSED_KEY".into(),
                token_prefix: "Bearer".into(),
            }
        }
        "api_key" => {
            config.auth = AuthConfig::ApiKeyHeader {
                env_var: "SYNTHETIC_UNUSED_KEY".into(),
                header_name: "x-api-key".into(),
            }
        }
        _ => {}
    }
    Ok(RhaiCustomProvider::new(
        Arc::new(config),
        Arc::new(ScriptLoader::with_default_engine()),
        "local".into(),
    )?)
}
