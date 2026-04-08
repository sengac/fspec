# AST Research — PROV-054 Copilot OAuth Patterns

This document captures the existing Rust patterns in `codelet/providers/` that PROV-054 will mirror for GitHub Copilot OAuth device flow implementation.

## Reference Modules Analyzed

| File | Lines | Purpose |
|------|------:|---------|
| `codelet/providers/src/claude_auth.rs` | 87 | Credential persistence (read/write claude_auth.json) |
| `codelet/providers/src/claude_oauth.rs` | 335 | OAuth primitives (URL build, token exchange, refresh) |
| `codelet/providers/src/codex/codex_device_auth.rs` | 308 | RFC 8628 device flow (request, poll, slow_down/pending handling) |
| `codelet/providers/tests/codex_device_auth_test.rs` | 621 | Wiremock-based integration tests for device flow |
| `codelet/providers/tests/fixtures/mod.rs` | 98 | Shared test helpers (env guards, JWT builder, token JSON) |

---

## Pattern 1: Credential Persistence (mirror `claude_auth.rs`)

### Key Decisions to Mirror

1. **Single struct** with `Serialize + Deserialize` representing the on-disk JSON
2. **Both async and sync readers** — async for NAPI bindings, sync for `manager.rs::get_<provider>()`
3. **Path resolution via `FSPEC_HOME` env var** with fallback to `$HOME/.fspec/credentials`
4. **`tokio::fs::create_dir_all` then `tokio::fs::write`** for the writer
5. **`anyhow::Result`** for all error returns

### Public API Surface (to mirror for Copilot)

```rust
// codelet/providers/src/claude_auth.rs:22-27
pub struct ClaudeAuthJson {
    pub access_token: String,
    pub refresh_token: String,
    pub expires: u64,  // ms since Unix epoch
}

pub fn get_claude_auth_path() -> PathBuf;                              // line 41
pub async fn read_claude_auth() -> Result<Option<ClaudeAuthJson>>;     // line 46
pub fn read_claude_auth_sync() -> Result<Option<ClaudeAuthJson>>;      // line 63
pub async fn write_claude_auth(auth: &ClaudeAuthJson) -> Result<()>;   // line 76
```

### Copilot-Specific Differences

- **Filename**: `copilot_auth.json` (not `claude_auth.json`)
- **File mode**: 0600 (must be enforced explicitly — claude_auth.rs does NOT set mode, so we add this)
- **Schema additions**: `enterprise_url: Option<String>` for Enterprise deployments
- **Token expiry**: GitHub Copilot device flow returns tokens with `expires=0` (never expires) per Slice 1 memo

### Action: New file `codelet/providers/src/copilot/auth.rs`

Mirror exact API but rename: `CopilotAuthJson`, `get_copilot_auth_path()`, `read_copilot_auth_sync()`, `write_copilot_auth()`.

---

## Pattern 2: Device Flow Orchestration (mirror `codex_device_auth.rs`)

### Key Functions

```rust
// src/codex/codex_device_auth.rs:99-125
pub async fn request_device_code(issuer_url: &str) -> Result<DeviceCodeResponse>;

// src/codex/codex_device_auth.rs:131-230
pub async fn poll_device_token(
    config: &PollConfig<'_>,
    device_code: &DeviceCodeResponse,
) -> Result<PollResult>;

// src/codex/codex_device_auth.rs:241-308
pub async fn device_auth_login(config: DeviceAuthConfig) -> Result<CodexTokens>;
```

### Critical Patterns to Mirror

1. **`SLOW_DOWN_INCREMENT_MS = 5_000`** constant (RFC 8628 §3.5)
2. **`PollConfig<'a>`** struct with `poll_interval_override_ms` and `slow_down_increment_override_ms` for testability
3. **`DeviceAuthConfig`** with `display_fn: Option<DisplayCallback>` callback for showing user_code + URL
4. **`PollResult` enum** with `Success { authorization_code, code_verifier }` and `TerminalError { error }` variants
5. **`tokio::time::timeout` wrapping the polling loop** for overall flow timeout

### Critical Difference Per Slice 1 Memo

GitHub Copilot device flow does NOT return `code_verifier` (no PKCE). Token exchange uses just the authorization_code (which is already an access_token in Copilot's flow). Different shape from Codex.

### Polling Wait Calculation (per PROV-054 Rule 5)

- On `authorization_pending`: sleep `(interval + 3 second safety margin)`
- On `slow_down`: server-provided interval + 5s increment per RFC 8628 §3.5
- This differs from Codex which uses just `interval` without the 3s safety margin

### Action: New file `codelet/providers/src/copilot/oauth.rs`

Adapt the `request_device_code` / `poll_device_token` / `device_auth_login` functions for GitHub's endpoints:
- `POST {host}/login/device/code` (replaces `/api/accounts/deviceauth/usercode`)
- `POST {host}/login/oauth/access_token` (replaces `/api/accounts/deviceauth/token`)
- `host` parameterized: `https://github.com` or `https://<enterprise-domain>`

---

## Pattern 3: Test Fixture Reuse

### Existing Helpers (extend `tests/fixtures/mod.rs`)

```rust
// tests/fixtures/mod.rs:64-71
pub fn setup_codex_home() -> (tempfile::TempDir, CodexHomeGuard);

// tests/fixtures/mod.rs:91-98
pub fn setup_fspec_home() -> (tempfile::TempDir, FspecHomeGuard);
```

`setup_fspec_home()` is ALREADY suitable for Copilot tests — both Claude and Copilot store under `FSPEC_HOME`. No new fixture needed.

### Test Pattern Template

```rust
#[tokio::test]
#[serial]
async fn test_login_to_github_com_via_device_flow() {
    // @step Given I have an active GitHub Copilot subscription on github.com
    let (_temp_dir, _guard) = setup_fspec_home();

    let mock_server = MockServer::start().await;

    // Mock device code endpoint
    Mock::given(method("POST"))
        .and(path("/login/device/code"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "device_code": "...",
            "user_code": "ABCD-1234",
            "verification_uri": "https://github.com/login/device",
            "interval": 5
        })))
        .mount(&mock_server)
        .await;

    // Mock polling endpoint
    Mock::given(method("POST"))
        .and(path("/login/oauth/access_token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "access_token": "ghu_test_token",
            "token_type": "bearer",
            "scope": "read:user"
        })))
        .mount(&mock_server)
        .await;

    let config = CopilotDeviceAuthConfig {
        host_url: mock_server.uri(),
        deployment_type: CopilotDeploymentType::GitHubCom,
        timeout_ms: 30_000,
        poll_interval_override_ms: Some(50),
        ..Default::default()
    };
    let result = copilot_device_auth_login(config).await;

    // @step Then a credential should be persisted at "~/.fspec/credentials/copilot_auth.json"
    assert!(result.is_ok());
    let auth_path = get_copilot_auth_path();
    assert!(auth_path.exists());

    // @step And the credential file permissions should be 0600
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(&auth_path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
    }
}
```

---

## Pattern 4: URL Normalization (Enterprise)

Per Rule 5 (mirroring opencode `copilot.ts:15` `normalizeDomain`):
- Strip `https://` / `http://` scheme
- Strip trailing slash
- Result: bare domain like `ghe.example.com`

This is a pure-string utility function — should live in `copilot/oauth.rs` as `normalize_enterprise_domain(input: &str) -> String`.

---

## Test Mapping — PROV-054 5 Scenarios

| Scenario | Test Name | Key Mocks |
|----------|-----------|-----------|
| Login github.com via device flow | `test_login_github_com_via_device_flow` | `/login/device/code`, `/login/oauth/access_token` |
| Login Enterprise with URL normalization | `test_login_enterprise_with_url_normalization` | Enterprise host mocks + normalization assertion |
| Polling: authorization_pending sleep+3s | `test_polling_handles_authorization_pending_with_safety_margin` | `error: authorization_pending` then success |
| Polling: slow_down increases interval | `test_polling_handles_slow_down_per_rfc8628` | `error: slow_down`, `interval: 10` |
| Logout deletes credential file | `test_logout_deletes_copilot_credential_file` | Setup auth.json → call logout → assert deleted |

---

## Module Layout (PROV-054 scope only)

```
codelet/providers/src/copilot/
  mod.rs              # Module exports
  auth.rs             # CopilotAuthJson + read/write/get_path
  oauth.rs            # request_device_code, poll_device_token, copilot_device_auth_login,
                      # normalize_enterprise_domain, CopilotDeviceAuthConfig, PollConfig, etc.
```

PROV-055/056 will add: `refreshing_client.rs`, `behavior_facade.rs`, `header_facade.rs`, `classifier.rs`, `endpoint.rs`, `models.rs`.

---

## Cargo Dependencies

`codelet/providers/Cargo.toml` already has:
- `anyhow` ✓
- `serde` + `serde_json` ✓
- `tokio` ✓
- `reqwest` ✓
- `tracing` ✓
- `tempfile` (dev) ✓
- `wiremock` (dev) ✓
- `serial_test` (dev) ✓
- `base64` (dev, used by fixtures for JWT) ✓

**No new dependencies needed for PROV-054.**

---

## Anti-Patterns Discovered (DO NOT mirror)

1. **`codex_device_auth.rs` uses PKCE-style code_verifier** — Copilot does NOT (no PKCE)
2. **`claude_oauth.rs::REQUIRED_BETA_HEADERS`** — Copilot has its own header set (PROV-055 scope)
3. **`extract_account_id` from JWT** — Copilot tokens are opaque `ghu_*` strings, not JWTs
