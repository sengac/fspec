# PROV-057 — GitHub Copilot Integration Broken: Investigation Report

**Date:** 2026-04-08
**Investigation method:** 5-agent parallel deep search via AgentManager
**Parent work units:** PROV-053, PROV-054, PROV-055, PROV-056 (all marked done but non-functional)
**Reported by:** User screenshot of TUI error in `/Users/rquast/Desktop/copilot.png`

---

## 1. The Visible Error

When the user picks a `github-copilot/<model>` from the model selector in the TUI:

```
Error
Failed to switch model: Failed to select model: [github-copilot] Authentication
error: Provider 'github-copilot' requires credentials. Available providers:
claude, gemini, zai, codex
```

Note `github-copilot` is NOT in the "Available providers" list — this misled three of the five investigation agents into thinking the provider wasn't registered. It IS registered. The list is filtered by which credentials happen to exist on the user's machine, not by which providers are wired into the codebase.

---

## 2. Three Independent Layers of Brokenness

The integration is broken at **three independent layers**, which is why a "fix the visible symptom" approach will fail. Fixing any one layer in isolation still leaves the user unable to use Copilot.

| Layer | Diagnosis | Identified by |
|---|---|---|
| **L1 — OAuth login fails** | Wrong `client_id` constant → device flow's token exchange is rejected by Copilot's gating endpoint, so the user can never write `copilot_auth.json` in the first place | Agent 5 (Copilot API expert) |
| **L2 — Token can't call API** | No token-exchange step → the GitHub OAuth `gho_*` token is sent directly as `Authorization: Bearer gho_*` to `api.githubcopilot.com`, which always returns 401 | Agent 5 |
| **L3 — Agent loop dispatch missing** | `run_with_provider!` macro in `session_manager.rs` has no `"github-copilot"` arm; even with valid credentials the agent loop falls through to `_ => Unsupported provider` | Agent 1 |

Plus a **bonus stale-cache bug** identified by Agent 2: even if a user did successfully log in today, `ProviderManager::detect()` snapshots credential availability once at construction and never re-detects, so the freshly-written `copilot_auth.json` would remain invisible until process restart.


---

## 3. Complete Call Chain (TUI → Error Origin)

Traced by Agent 4 (Model Selection Flow Tracer).

| # | Site | Action |
|---|---|---|
| 1 | `src/tui/components/AgentView.tsx:2585-2592` | `/model` slash command opens `ModelSelectorScreen` |
| 2 | `src/tui/components/AgentView.tsx:2917` | `handleModelSelect(selection)` invokes `selectModel({...})` |
| 3 | `src/tui/services/modelSelectionService.ts:108-126` | Cloud branch calls `sessionSetModel(sessionId, "github-copilot", modelId)` |
| 4 | `codelet/napi/src/session_manager.rs:6242` | `#[napi] pub async fn session_set_model` |
| 5 | `codelet/napi/src/session_manager.rs:6261` | `inner.provider_manager_mut().select_model("github-copilot/<model>")` |
| 6 | `codelet/providers/src/manager.rs:213` | `ProviderManager::select_model` |
| 7 | `codelet/providers/src/manager.rs:225, 339-355` | `map_provider_id_to_type("github-copilot")` → `ProviderType::GitHubCopilot` ✓ |
| 8 | `codelet/providers/src/manager.rs:238` | `provider_type.has_credentials(&self.credentials)` → **`false`** |
| 9 | `codelet/providers/src/credentials.rs:131-143` → `copilot/auth.rs:118-128` | `read_copilot_auth_sync()` returns `Ok(None)` (file missing because login never succeeded) |
| 10 | `codelet/providers/src/manager.rs:239-246` | **ORIGIN**: `Err(ProviderError::auth(...))` |

### Error string assembly (4 prefix layers stacking bottom-up)

| Layer | File:line | Adds |
|---|---|---|
| 1 | `codelet/providers/src/manager.rs:241-245` | `"Provider '{}' requires credentials. Available providers: {}"` |
| 2 | `codelet/providers/src/manager.rs:239` (via `ProviderError::auth` Display) | `"[github-copilot] Authentication error: ..."` |
| 3 | `codelet/napi/src/session_manager.rs:6270` | `"Failed to select model: ..."` |
| 4 | `src/tui/components/AgentView.tsx:2930` | `"Failed to switch model: ..."` |


---

## 4. Provider Wiring Matrix

Verified by Agent 3 (Registry Architecture Expert) — at the trait/registry layer, `github-copilot` is fully wired:

```
Provider        | ProviderType | FromStr | has_credentials | available_providers | LlmProvider impl
----------------+--------------+---------+-----------------+---------------------+------------------
claude          |      ✓       |    ✓    |        ✓        |          ✓          |        ✓
gemini          |      ✓       |    ✓    |        ✓        |          ✓          |        ✓
zai             |      ✓       |    ✓    |        ✓        |          ✓          |        ✓
codex           |      ✓       |    ✓    |        ✓        |          ✓          |        ✓
github-copilot  |      ✓       |    ✓    |        ✓        |          ✓          |        ✓
```

The "Available providers" string in the error comes from `ProviderCredentials::available_providers()` at `codelet/providers/src/credentials.rs:79-100`, which iterates over a set of *runtime boolean flags* (`claude_available`, `gemini_available`, …, `github_copilot_available`). Those booleans are populated by `ProviderCredentials::detect()` based on which credential files / env vars are present at construction time. **The provider is registered; the user just isn't authenticated.**

This is why "wire it up" fixes are insufficient — the provider IS wired. The wiring leads to a credential check that can never succeed because the OAuth flow that produces the credentials is itself broken.


---

## 5. Layer 1 — Wrong OAuth Client ID

**File:** `codelet/providers/src/copilot/oauth_types.rs:14`
**Current:** `COPILOT_CLIENT_ID = "Ov23li8tweQw6odWQebz"` (this is the **opencode** project's client ID — borrowed verbatim from their codebase without verifying it works for fspec)
**Required:** `COPILOT_CLIENT_ID = "Iv1.b507a08c87ecfe98"`

`Iv1.b507a08c87ecfe98` is the well-known GitHub Copilot OAuth client ID used by:
- `copilot.vim` (GitHub's official Neovim plugin)
- The JetBrains Copilot plugin
- aider, cline, and most third-party Copilot integrations

**Why it matters:** GitHub's `/copilot_internal/v2/token` token-exchange endpoint validates the originating `client_id`. With a non-Copilot client ID, the device-code request to `https://github.com/login/device/code` may succeed (it's not gated), but the resulting `gho_*` token will be rejected when you later try to mint a Copilot token from it. The user can dance through the device flow forever and never get a usable session.

---

## 6. Layer 2 — Missing Token Exchange Step (THE BIG ONE)

**Spec from Agent 5 (Copilot API integration expert):**

GitHub Copilot uses a **two-token model**:

1. The OAuth device flow gives you a long-lived **GitHub** OAuth token (`gho_*` / `ghu_*`).
2. That token is then exchanged at `https://api.github.com/copilot_internal/v2/token` for a short-lived (~25 min) **Copilot** API token.
3. *Only* the short-lived Copilot token can be sent as `Authorization: Bearer <token>` to `api.githubcopilot.com`.

**The current code skips step 2 entirely.** It stores the `gho_*` token in `copilot_auth.json` and tries to use it directly against the Copilot API. Every request returns HTTP 401.

### Token exchange endpoint specification

| Field | Value |
|---|---|
| **Endpoint** | `GET https://api.github.com/copilot_internal/v2/token` |
| **Auth header** | `Authorization: token <gho_*>` (note: `token`, **not** `Bearer`) |
| **Required headers** | `Editor-Version: codelet/<version>` (e.g. `Neovim/0.9.0`, `vscode/1.86.0`)<br>`Editor-Plugin-Version: copilot.vim/1.16.0` (any non-empty value)<br>`User-Agent: GithubCopilot/<version>`<br>`Accept: application/json` |
| **Response (200)** | `{ "token": "tid=...;exp=...;...", "expires_at": <unix-seconds>, "refresh_in": <seconds-until-refresh>, "endpoints": { "api": "https://api.githubcopilot.com", ... }, "tracking_id": "...", "sku": "..." }` |
| **Token format** | Looks like `tid=<id>;exp=<unix>;sku=<sku>;...:<sig>` — opaque, do NOT parse |
| **Lifetime** | ~25 minutes |
| **Refresh strategy** | Re-call the same `/copilot_internal/v2/token` endpoint with the same `gho_*` token any time `now + 60s >= expires_at`. The `gho_*` token itself never expires; only the minted Copilot token does. |
| **Enterprise variant** | For GHE deployments, the exchange endpoint becomes `https://<enterprise-host>/api/v3/copilot_internal/v2/token` and the `endpoints.api` field in the response will point at `https://copilot-api.<host>` (not `api.githubcopilot.com`). **Always trust the `endpoints.api` field from the response over hard-coded URLs.** |


### Required schema change

`CopilotAuthJson` in `codelet/providers/src/copilot/auth.rs:62-76` currently treats `access_token` and `refresh_token` as identical with `expires: 0` (the "never expires" sentinel). It must be extended to track *both* tokens separately:

```rust
pub struct CopilotAuthJson {
    // Long-lived GitHub OAuth token (gho_* / ghu_*)
    pub github_oauth_token: String,

    // Short-lived Copilot API token minted from /copilot_internal/v2/token
    pub copilot_token: Option<String>,
    pub copilot_token_expires_at: Option<u64>,

    // The endpoints.api field returned by the exchange — trust this over hard-coded URLs
    pub endpoints_api: Option<String>,

    // Enterprise host (None for github.com)
    pub enterprise_url: Option<String>,
}
```

### Required refresh logic

In `CopilotProvider::complete()` and `complete_with_tools()` (`codelet/providers/src/copilot/provider.rs`), before each request:

```rust
let now = unix_timestamp();
let needs_refresh = self.auth.copilot_token.is_none()
    || self.auth.copilot_token_expires_at.map_or(true, |exp| exp <= now + 60);
if needs_refresh {
    let exchange_response = exchange_github_token_for_copilot_token(&self.auth.github_oauth_token, ...).await?;
    self.auth.copilot_token = Some(exchange_response.token);
    self.auth.copilot_token_expires_at = Some(exchange_response.expires_at);
    self.auth.endpoints_api = Some(exchange_response.endpoints.api);
    write_copilot_auth(&self.auth).await?;
}
```


---

## 7. Layer 3 — Agent Loop Dispatch Arm Missing

**File:** `codelet/napi/src/session_manager.rs:5019-5073`

The `run_with_provider!` dispatch macro that wraps the agent loop has arms for `claude`, `gemini`, `zai`, and `codex` — but **no `"github-copilot"` arm**. After fixing layers 1 and 2, the user could authenticate and call `select_model` successfully, but the next request that hits the agent loop would still fall through to:

```rust
_ => Err(Error::from_reason(format!("Unsupported provider: {}", current_provider)))
```

**Fix:** Add a `"github-copilot" | "copilot" => { ... CopilotProvider::new(deployment, token, model) ... }` arm matching the pattern of the existing arms.

Related missing wiring at the same file/macro level:
- `codelet/napi/src/deep_search_provider_config.rs:1` — imports only `build_gemini_system_prompt, select_claude_facade`. Need to add a `select_copilot_facade` import + branch so DeepSearch sub-agents can use Copilot.

---

## 8. Bonus — Stale Credential Cache (Agent 2 finding)

**File:** `codelet/providers/src/manager.rs:213` (`ProviderManager::select_model`)

`ProviderManager::detect()` runs **once at construction** and the result is cached. After the OAuth flow writes `copilot_auth.json`, the manager never re-detects, so `has_github_copilot_auth()` would still return its cached `false` until process restart.

Compare `manager.rs:397-424` (`get_claude()`) which re-reads the auth file on every call. Copilot needs the same treatment, OR `select_model()` needs to call `self.credentials = ProviderCredentials::detect()` before the `has_credentials` check.

**This is independent of layers 1–3.** Even with the OAuth flow fixed and the dispatch arm added, the user would still see "requires credentials" until they restart fspec — unless this cache invalidation is also fixed.


---

## 9. What Already Exists And Is Complete

Verified by Agent 1 (Codebase Investigator). All 13 of these files in `codelet/providers/src/copilot/` are functionally complete and need no changes for PROV-057:

- `mod.rs` (95 lines) — module declarations, re-exports, `CONTEXT_WINDOW=200000`, `MAX_OUTPUT_TOKENS=4096`
- `auth.rs` (298 lines) — `CopilotAuthJson`, read/write/delete + `get_copilot_auth_path` (`$FSPEC_HOME/credentials/copilot_auth.json` mode 0600). ⚠️ Schema needs extension per §6.
- `oauth_types.rs` — `CopilotDeploymentType::{GitHubCom, Enterprise{host}}`, `CopilotPollResult`, scopes. ⚠️ `COPILOT_CLIENT_ID` constant needs the fix from §5.
- `oauth_device_code.rs` — `request_device_code(host_url)`, `normalize_enterprise_domain`
- `oauth_polling.rs` — `poll_device_token` with RFC 8628 handling
- `base_url.rs` — `base_url_for(deployment)` (api.githubcopilot.com vs copilot-api.<host>)
- `constants.rs` — User-Agent, x-initiator, Openai-Intent literals
- `header_facade.rs` — `CopilotHeaderFacade::build_headers` with conditional Copilot-Vision-Request
- `classifier.rs` — `CopilotRequestClassifier::classify` (vision detection)
- `endpoint.rs` — `CopilotEndpointFacade::select` (gpt-5 → /responses, others → /chat/completions)
- `behavior_facade.rs` — `GptBehaviorFacade`, `ClaudeBehaviorFacade`, `GeminiBehaviorFacade` + selector
- `codelet/providers/src/manager.rs` — `ProviderType::GitHubCopilot` variant + `has_credentials`, `context_window`, `max_output_tokens` dispatch (L630-L640)
- `src/tui/utils/copilotLoginFlow.ts` — `startCopilotLogin`, `submitCopilotEnterpriseHost` (orphaned but complete — see §10 item 9)

**Two files exceed the 300-line CLAUDE.md budget** and need splitting (out of scope for PROV-057, track separately):
- `codelet/providers/src/copilot/oauth.rs` — 364 lines
- `codelet/providers/src/copilot/provider.rs` — 468 lines


---

## 10. Ordered Fix Plan

| # | Layer | File | Change | Why |
|---|---|---|---|---|
| 1 | L1 | `codelet/providers/src/copilot/oauth_types.rs:14` | Change `COPILOT_CLIENT_ID` to `Iv1.b507a08c87ecfe98` | Without this, the device flow's eventual token exchange is rejected |
| 2 | L2 | New: `codelet/providers/src/copilot/token_exchange.rs` | Add `exchange_github_token_for_copilot_token()` calling `GET /copilot_internal/v2/token` with the headers spec'd in §6 | Without this, every API call returns 401 |
| 3 | L2 | `codelet/providers/src/copilot/auth.rs` | Extend `CopilotAuthJson` schema per §6 to track `github_oauth_token`, `copilot_token`, `copilot_token_expires_at`, `endpoints_api` separately | Two-token model needs two slots |
| 4 | L2 | `codelet/providers/src/copilot/provider.rs` | In `complete()` / `complete_with_tools()`, check `copilot_token_expires_at` and call refresh before each request; trust `endpoints_api` from response | 25-min TTL requires proactive refresh |
| 5 | L3 | `codelet/napi/src/session_manager.rs:5019-5073` | Add `"github-copilot" \| "copilot" => { ... }` arm to `run_with_provider!` dispatch | Without this, agent loop fails even after auth works |
| 6 | L3 | `codelet/napi/src/deep_search_provider_config.rs` | Import + branch for `select_copilot_facade` | DeepSearch sub-agents need it |
| 7 | Stale cache | `codelet/providers/src/manager.rs:213` (`select_model`) | Call `self.credentials = ProviderCredentials::detect()` before `has_credentials` check (or re-read auth file like `get_claude()` does) | Otherwise login → switch in same session still fails |
| 8 | Model picker | `src/tui/store/...` + `src/tui/services/modelInitialization.ts` | After OAuth completes, call `copilotListModels()` and merge into model store | TUI picker needs to show Copilot models after login |
| 9 | UX | `src/tui/components/AgentView.tsx:2917` | If `selectedProvider === 'github-copilot'` and no creds, dispatch `startCopilotLogin()` from `src/tui/utils/copilotLoginFlow.ts` instead of erroring | The orphaned login flow finally gets invoked |
| 10 | Hygiene | Track separately | Split `oauth.rs` (364 → 2 files) + `provider.rs` (468 → 2 files) | CLAUDE.md 300-line rule (out of scope for PROV-057) |


---

## 11. How To Verify The Fix

After implementing the fix plan, verification has three independent checkpoints:

### Checkpoint A — OAuth login completes
1. Run the Copilot OAuth device flow from the TUI provider settings panel.
2. Verify `~/.fspec/credentials/copilot_auth.json` exists with mode `0600`.
3. Verify the JSON contains a non-empty `github_oauth_token` field starting with `gho_` or `ghu_`.

### Checkpoint B — Token exchange works
1. With `copilot_auth.json` present, restart fspec.
2. Call `select_model("github-copilot/<some-model-id>")` from the TUI.
3. Verify no "requires credentials" error.
4. Verify `copilot_auth.json` has been updated with non-empty `copilot_token`, `copilot_token_expires_at`, and `endpoints_api`.

### Checkpoint C — Agent loop dispatches
1. With a valid Copilot session, send a chat message that requires tool calls.
2. Verify the response streams without "Unsupported provider" error.
3. Verify the request goes to the URL in `endpoints_api` (not a hard-coded URL).
4. Wait 26 minutes (past the Copilot token TTL) and send another message — verify the token is silently refreshed and the request still succeeds.

### Regression checkpoints
- Ensure `claude`, `gemini`, `zai`, `codex` model selection still works.
- Ensure DeepSearch sub-agents can use `github-copilot` as their provider.
- Ensure the stale-cache fix doesn't introduce unnecessary `detect()` calls in hot paths.


---

## 12. Investigation Methodology

This investigation was performed by spawning 5 parallel AgentManager subordinates, each given a different scope of inquiry into the same screenshot-based bug report:

| Agent | Role | Scope |
|---|---|---|
| 1 | Codebase Investigator | Inventory every file mentioning copilot/github-copilot — what exists, what's stubbed, what's missing |
| 2 | Auth/OAuth Specialist | How each provider stores credentials; trace the exact `has_credentials` failure |
| 3 | Provider Registry Architect | Wiring matrix — compare github-copilot to claude/gemini/zai/codex for every registration point |
| 4 | Model Selection Flow Tracer | End-to-end call chain from TUI keystroke to error emission site |
| 5 | GitHub Copilot API Expert | Definitive spec for what a working integration needs (client_id, token exchange, refresh, endpoints) |

The agents converged on **different layers of the same multi-layer problem**, which is why the report stitches their findings together. Critically:

- Agents 1, 2, 3, 4 each independently concluded the visible error meant "the user just hasn't logged in yet" — a *correct* but *insufficient* diagnosis.
- Agent 5 identified *why* the user could never log in even if they tried: the OAuth client_id is wrong AND there is no token exchange step.
- Combining all five reports revealed the three-layer brokenness summarized in §2.

This is a good example of why parallel multi-agent investigation beats single-threaded exploration for systems with multiple interacting layers — no single agent would have surfaced all three layers, but the convergence of five agents on the same code paths from different angles made the gaps obvious.

---

## 13. References

- Screenshot of the original error: `/Users/rquast/Desktop/copilot.png`
- Parent work units: PROV-053 (provider), PROV-054 (OAuth device flow), PROV-055 (HTTP middleware), PROV-056 (model catalog) — all marked done but non-functional
- Well-known Copilot client_id reference: used by `copilot.vim`, JetBrains plugin, aider, cline, opencode (note: opencode uses a *different* client_id `Ov23li8tweQw6odWQebz` which is what fspec accidentally borrowed)
- Token exchange endpoint reference: `GET https://api.github.com/copilot_internal/v2/token`

