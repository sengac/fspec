# Review: PROV-054 — GitHub Copilot OAuth device flow & token storage

## Status: WARN

Tests pass, build is clean, all 5 scenarios are covered with @step comments matching exactly. However, there are significant DRY/SOLID concerns (massive copy-paste from `codex_device_auth.rs`) and several Gherkin step-ordering / coverage gaps that should be addressed.

---

## 🔴 Critical Issues (Must Fix)

**None.** Build passes, all 5 tests pass, scenarios 1:1 mapped, no `unwrap()`/`expect()`/`todo!()`/`panic!()` in production paths (only `unwrap_or` / `unwrap_or_default` / `unwrap_or_else`, all of which are safe fallbacks).

---

## 🟡 Warnings (Should Fix)

### W1. Gherkin step ordering — Scenario 2 mixes assertion with action under `When`
**File:** `spec/features/github-copilot-oauth-device-flow-token-storage.feature:55-64` (Scenario: Login to GitHub Enterprise)

- Line 60: `And I enter "https://ghe.example.com/" at the enterpriseUrl prompt` — this is a `When` action (user input), then line 61 jumps to `Then the enterprise URL should be normalized…`. The action of typing the URL is interleaved with prompts in a `When` block — that's fine — but lines 62–63 (`And the device code flow should POST to…` / `And the polling loop should POST to…`) describe HTTP routing behavior which is technically a `Then` and is correctly chained, so this scenario is borderline OK.
- **However**, the test (lines 156–212) only verifies normalization in isolation (`assert_eq!(normalized, "ghe.example.com")`) and never actually exercises the routing inside `copilot_device_auth_login` with the normalized host as the base URL — the test passes the *mock_server URI* as `host_url`, not `https://ghe.example.com`. The test does NOT verify that the deployment_type's normalized host is what gets used to construct the URL — only that `enterprise_url` is correctly persisted. This is a coverage gap on rules 2 & 3.

### W2. Scenario 3 contains an untestable open-ended assertion
**File:** `spec/features/github-copilot-oauth-device-flow-token-storage.feature:66-72`

The scenario reads `Given... And... When the polling endpoint returns "authorization_pending" Then the polling loop should sleep...`. This is structurally fine, but line 72 (`And polling should continue until the user approves the device code or the code expires`) is **untestable as written** — it's an open-ended assertion with no terminal state. The test merely verifies that *one* `authorization_pending` response is followed by *one* `success` response, not "until approval or expiry." Either tighten the Gherkin to a finite assertion or add a second test variant covering expiry.

### W3. Massive duplication with `codex_device_auth.rs` (DRY violation — single biggest concern)
**Files:** `codelet/providers/src/copilot/oauth.rs` vs `codelet/providers/src/codex/codex_device_auth.rs`

The two files implement the same RFC 8628 state machine with **byte-identical error strings** and structurally identical scaffolding. Concrete duplicates:

| Item | codex_device_auth.rs | copilot/oauth.rs |
|---|---|---|
| `SLOW_DOWN_INCREMENT_MS` constant | line 79 | line 43 |
| `DisplayCallback` type alias `Box<dyn Fn(&str, &str) + Send + Sync>` | line 42 | line 78 |
| `DevicePollResponse` struct (same name!) | lines 87–94 | lines 122–129 |
| `request_device_code()` skeleton | lines 99–125 | lines 158–187 |
| `poll_device_token()` polling loop + match block | lines 131–230 | lines 197–306 |
| Error string `"Device code has expired. Please restart the login flow."` | line 187 | line 267 |
| Error string `"User denied authorization (access_denied)."` | line 193 | line 273 |
| Error string `"Device auth polling timed out after {}ms"` | line 226 | line 302 |
| `format!("Device auth polling returned error: {other}")` | line 198 | line 278 |
| Login orchestrator step structure | lines 241–308 | lines 315–364 |

The architecture note in PROV-054 explicitly states *"codex_device_auth.rs is NOT reusable (different OAuth dialect)"* — but inspection shows the dialect difference is essentially **only the URL paths, the success-payload field names, and an extra `safety_margin_ms` constant**. The error-classification match block, the timeout wrapper, the override-vs-server-interval logic, and the Display callback type alias are reusable verbatim.

**Recommendation:** Extract a generic `oauth_device_flow.rs` module owning:

- `pub trait DeviceFlowDialect` with associated `DeviceCodeResponse: DeserializeOwned`, `Success`, `client_id() -> &'static str`, `device_code_path() -> &'static str`, `token_path() -> &'static str`, `extract_success(response) -> Option<Self::Success>`
- `pub async fn poll_device_token<D: DeviceFlowDialect>(...)` containing the loop + match block (~100 lines deduplicated)
- `pub const SLOW_DOWN_INCREMENT_MS: u64 = 5_000;`
- Centralized terminal-error strings as constants

This alone would remove ~150 lines of copy-paste and ensure RFC 8628 fixes apply to both providers.

### W4. `claude_auth.rs` and `copilot/auth.rs` are 80% byte-identical
**Files:** `codelet/providers/src/claude_auth.rs` vs `codelet/providers/src/copilot/auth.rs`

- `get_fspec_home()` (claude_auth.rs:31-38) and (copilot/auth.rs:40-47) are **byte-for-byte identical**
- `read_claude_auth()` / `read_copilot_auth()` differ only in struct type
- `read_claude_auth_sync()` / `read_copilot_auth_sync()` differ only in struct type
- `write_claude_auth()` / `write_copilot_auth()` differ only in the post-write `chmod 0600` block (Copilot adds it)

**Recommendation:** Extract a `codelet/providers/src/auth_storage.rs` module with a `CredentialFile` trait:

```rust
pub trait CredentialFile: Serialize + DeserializeOwned {
    const FILENAME: &'static str;
    const ENFORCE_MODE_0600: bool = false;
}
```

Then `claude_auth.rs` and `copilot/auth.rs` collapse to ~20-line trait impls plus thin re-exports preserving the public API.

### W5. `claude_auth.rs` is missing mode-0600 enforcement
**File:** `codelet/providers/src/claude_auth.rs:76-87`

This isn't a PROV-054 issue per se, but PROV-054 has now established that OAuth credential files MUST be 0600 (Rule 9). `claude_auth.rs` writes refresh tokens with default umask permissions (typically 0644). If you extract a shared `auth_storage` module with `ENFORCE_MODE_0600 = true` for both providers, you fix this gap as a side effect.

### W6. `copilot/oauth.rs` is over the 300-line guideline
**File:** `codelet/providers/src/copilot/oauth.rs` — 364 lines

The CLAUDE.md guideline is "Keep files under 300 lines — refactor when approaching this limit." After extracting the shared device-flow module (W3), `oauth.rs` would shrink to ~150 lines (constants + types + dialect impl + `copilot_device_auth_login` orchestrator), bringing it well under the limit.

### W7. Scenario 2 is structurally split between unit-level and integration-level assertions
**File:** `codelet/providers/tests/copilot_oauth_device_flow_test.rs:143-213`

The test calls `normalize_enterprise_domain()` directly (line 156) to satisfy the "Then enterprise URL should be normalized" step, then hands the *mock server URI* (not the normalized domain) to `copilot_device_auth_login`. The mock server happens to satisfy the device-code path because path matching is relative — but **the integration of "user enters https://ghe.example.com/ → orchestrator routes to https://ghe.example.com/login/device/code"** is never end-to-end exercised. The persistence assertion at line 211 (`parsed["enterprise_url"] == "ghe.example.com"`) only verifies the field, not the routing.

**Recommendation:** Add an explicit assertion that the mock server received exactly the path `/login/device/code` with form-encoded `client_id=Ov23li8tweQw6odWQebz` and `scope=read:user`, OR use `wiremock::matchers::body_string_contains` to prove the request payload matches PROV-054 Rule 1. As written, the test would still pass if `copilot_device_auth_login` somehow stripped the `client_id` or `scope` form fields.

### W8. `slow_down` test does not actually verify the "server-provided interval is adopted" rule
**File:** `codelet/providers/tests/copilot_oauth_device_flow_test.rs:303-376`

The test (line 372) only asserts `elapsed >= 200ms`, which verifies the increment was applied — but it does NOT verify that the **server-provided interval of 10 seconds** was adopted. With `poll_interval_override_ms: Some(50)`, the production code at `oauth.rs:252-258` deliberately *ignores* the server-provided interval and reuses the 50ms override. This means the scenario step "the polling loop should adopt the server-provided interval of 10 seconds" (line 78) is **not actually verified by the test** — the test exercises only the increment, not the adoption.

**Recommendation:** Either (a) add a second sub-test without the override that verifies adoption of the server interval, or (b) re-architect the override so the server interval is honored even with the override (and assert `current_interval_ms` post-slow_down via a public observation hook). At minimum, the test currently has a coverage gap on rule 7.

### W9. `oauth.rs:252-258` — override-bypass logic is subtle and underdocumented
**File:** `codelet/providers/src/copilot/oauth.rs:252-258`

```rust
let server_interval_ms = if config.poll_interval_override_ms.is_some() {
    // Use the existing scaled interval base and just add the increment
    current_interval_ms
} else {
    server_interval_secs * 1000
};
current_interval_ms = server_interval_ms;
```

This branch silently ignores the server-provided `interval` when an override is set. The comment "preserve test override behaviour" admits this is a test-only escape hatch. PROV-054 Rule 7 requires the production code to "use server-provided interval if present." The current logic complies in production but **the test override prevents PROV-054 Rule 7 from being verifiable in tests**. See W8 for the fallout.

### W10. `normalize_enterprise_domain` does not validate input
**File:** `codelet/providers/src/copilot/oauth.rs:146-152`

Inputs like `"   https://ghe.example.com   "` (whitespace), `"HTTPS://ghe.example.com/"` (case), or `"ghe.example.com/path/segment"` (path) are not handled. The opencode reference is similarly naive, but a hostile user input like `"javascript:alert(1)"` would slip through unchanged and end up as the URL prefix in `request_device_code`. Consider trimming whitespace, lowercasing the scheme check, validating the result is a syntactically valid host (no spaces, no path), or rejecting non-HTTP(S) schemes outright.

### W11. Test file uses `#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]` blanket allow
**File:** `codelet/providers/tests/copilot_oauth_device_flow_test.rs:1`

While unwrap/expect/panic in tests is generally acceptable, blanketing the entire file is heavy-handed. The test uses `.unwrap()` extensively (lines 87, 98, 105, 123, 128, 129, 134, 159, 198, 206, 207, 209, 210, 211, 212, 364) — fine for tests, but consider scoping the allow narrower or relying on Vitest-style `.expect("...")` with descriptive messages for failure clarity.

### W12. CLI integration missing — feature mentions `codelet auth login github-copilot` but no CLI wiring exists
**Search:** `grep -rn "auth login\|copilot" codelet/cli/src/` returned no matches.

The Gherkin scenarios all begin with `When I run \`codelet auth login github-copilot\`` but the test bypasses the CLI entirely and calls `copilot_device_auth_login()` directly. There is **no CLI command wired up** for `codelet auth login github-copilot` or `codelet auth logout github-copilot`. This means the user-facing entry point implied by every scenario is **not implemented**. Per the description ("Includes CLI login/logout commands"), this scope appears unfinished — the test only validates the library API, not the user journey.

If the intent is that PROV-055 or PROV-056 will add the CLI wiring, then PROV-054's status as `done` is premature unless an explicit follow-up child work unit is tracking the gap. Recommend either (a) creating PROV-054a "CLI wiring for github-copilot auth login/logout" as a follow-up, or (b) adding `@blocked-by-cli-wiring` to the feature file with a comment explaining the deferral.

---

## 🟢 Observations (Nice to Have)

### O1. `oauth.rs:175` uses `unwrap_or_default()` on response body
The fallback to empty string for an unreadable error body is fine, but losing the underlying read error makes diagnosing 500-level failures harder. Consider logging the read error at `debug!` level.

### O2. `oauth.rs:159` URL construction does not normalize trailing slashes on `host_url`
If a caller passes `host_url: "https://github.com/"`, the resulting URL is `https://github.com//login/device/code` (double slash). Most servers tolerate it, but `normalize_enterprise_domain` strips trailing slashes from enterprise hosts only — github.com hosts could still hit this if a developer mistypes the constant. Consider `host_url.trim_end_matches('/')`.

### O3. `auth.rs:44` falls back to `/tmp` if `HOME` is unset
This is mirrored from `claude_auth.rs:35` for consistency, but writing OAuth credentials to `/tmp` (world-readable in many configurations) is a latent security risk on systems without HOME. Consider `Err`-ing instead of falling back, or using `env::temp_dir()` with a random suffix. Low priority because no production environment lacks HOME.

### O4. `CopilotPollConfig` and `CopilotDeviceAuthConfig` differ by exactly one field
Both structs duplicate `host_url`, `timeout_ms`, `poll_interval_override_ms`, `slow_down_increment_override_ms`, `authorization_pending_safety_margin_override_ms`. The `CopilotDeviceAuthConfig` adds `deployment_type` and `display_fn`. Consider composition: `CopilotDeviceAuthConfig { poll: CopilotPollConfig, deployment_type, display_fn }`.

### O5. Constant relocation
`oauth.rs` could move `COPILOT_CLIENT_ID`, `COPILOT_DEFAULT_HOST`, `COPILOT_OAUTH_SCOPE` constants to a `copilot/constants.rs` module to keep `oauth.rs` focused on flow logic.

### O6. Test fixtures verification
Test file fixtures module is inlined via `mod fixtures;` — good. Verify `fixtures::setup_fspec_home` properly sets and unsets `FSPEC_HOME` (not directly inspected here).

### O7. `display_fn` callback signature
The `display_fn` callback uses `Fn(&str, &str)` (two unnamed strings). A small struct `DisplayContext { user_code, verification_uri }` would prevent argument-order mistakes at call sites.

### O8. `oauth.rs:248` extracts `interval` from the slow_down response
The field is documented in `DevicePollResponse:126` as "Optional server-provided polling interval (slow_down responses)" — the JSON spec for github.com's device flow does NOT actually return `interval` on slow_down responses (it returns it on the initial device-code response). Worth verifying against the actual GitHub API docs: https://docs.github.com/en/apps/creating-github-apps/authenticating-with-a-github-app/generating-a-user-access-token-for-a-github-app#using-the-device-flow-to-generate-a-user-access-token

### O9. Doc comment style consistency
`oauth.rs:78` and `auth.rs:1` both use the doc comment `//!` style — consistent and good.

### O10. `expires: 0` magic number
The `expires: 0` semantic ("never expires") is documented inline in `auth.rs:30` but not exported as a constant. Consider `pub const COPILOT_TOKEN_NEVER_EXPIRES: u64 = 0;` so callers checking `auth.expires == 0` use the constant rather than a magic number.

---

## Coverage Verification

- **Feature file:** `spec/features/github-copilot-oauth-device-flow-token-storage.feature` — **OK** (5 scenarios, valid Gherkin, @PROV-054 tag present, architecture doc string present, no placeholders)
- **Test file:** `codelet/providers/tests/copilot_oauth_device_flow_test.rs` — **OK with caveats** (5 tests, all 36 @step comments match Gherkin steps EXACTLY; gaps noted in W7, W8, W12)
- **Impl files:**
  - `codelet/providers/src/copilot/oauth.rs` — **OK** (compiles, tests pass) but **WARN** on size (364 lines, > 300) and DRY (W3)
  - `codelet/providers/src/copilot/auth.rs` — **OK** (compiles, tests pass) but **WARN** on DRY (W4)
- **Scenario coverage:** **5/5 scenarios covered** at @step level, but rules 2, 3, and 7 have semantic verification gaps (W7, W8)
- **Build:** `cargo build -p codelet-providers` ✅ clean
- **Tests:** `cargo test -p codelet-providers --test copilot_oauth_device_flow_test` → **5 passed; 0 failed**

---

## Files Reviewed

1. `/Users/rquast/projects/fspec/spec/features/github-copilot-oauth-device-flow-token-storage.feature` (86 lines)
2. `/Users/rquast/projects/fspec/codelet/providers/tests/copilot_oauth_device_flow_test.rs` (423 lines)
3. `/Users/rquast/projects/fspec/codelet/providers/src/copilot/oauth.rs` (364 lines)
4. `/Users/rquast/projects/fspec/codelet/providers/src/copilot/auth.rs` (125 lines)
5. `/Users/rquast/projects/fspec/codelet/providers/src/copilot/mod.rs` (73 lines)
6. `/Users/rquast/projects/fspec/codelet/providers/src/claude_auth.rs` (87 lines) — for DRY comparison
7. `/Users/rquast/projects/fspec/codelet/providers/src/codex/codex_auth.rs` (226 lines) — for DRY comparison
8. `/Users/rquast/projects/fspec/codelet/providers/src/codex/codex_device_auth.rs` (308 lines) — for DRY comparison
9. `/Users/rquast/projects/fspec/codelet/providers/src/oauth_http_utils.rs` (101 lines) — to check for shared utilities
10. `/Users/rquast/projects/fspec/codelet/providers/src/claude_oauth.rs` (334 lines) — for DRY comparison

---

## Bottom Line

The implementation functionally satisfies PROV-054's acceptance criteria (build clean, tests pass, scenarios mapped, file mode 0600 enforced, all rules implemented in production code paths). The status is `done` and that's defensible.

**However**, the work introduces ~150 lines of copy-paste duplication of `codex_device_auth.rs` and ~80% byte-identical duplication of `claude_auth.rs`, both of which the architecture note hand-waved away (`"codex_device_auth.rs is NOT reusable"`) but the actual code disproves. A follow-up refactor work unit should extract `oauth_device_flow.rs` and `auth_storage.rs` shared modules. There's also a missing CLI wiring (W12) and two test verification gaps (W7, W8) that should be addressed before considering the user journey complete.
