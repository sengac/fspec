# PROV-012 Review Fixes

Post-completion review of the Anthropic OAuth Login Flow (PROV-012) and all child cards (PROV-020 through PROV-027).

---

## 🔴 FIX 1: Parallel Test Failures — Missing `#[serial]` Annotations

**Severity:** Blocker — CI will report failures on every run  
**Effort:** ~5 minutes

### Problem

4 tests manipulate shared process-wide env vars (`FSPEC_HOME`, `ANTHROPIC_API_KEY`, `CLAUDE_CODE_OAUTH_TOKEN`) without `#[serial]` serialization. When Cargo runs tests in parallel (default), env var mutations leak between threads, causing non-deterministic failures.

All 4 pass with `--test-threads=1` but fail in default parallel execution.

### Failing Tests

**`codelet/providers/tests/claude_oauth_routing_test.rs`:**
- `test_credential_detection_with_claude_auth_json`
- `test_read_claude_auth_sync_reads_valid_file`

**`codelet/napi/tests/claude_oauth_resolver_test.rs`:**
- `test_credential_resolver_finds_oauth_tokens_from_claude_auth`
- `test_credential_resolver_sets_claude_code_oauth_token_env_var`

### Fix

Add `use serial_test::serial;` import and `#[serial]` attribute to every test in both files that touches env vars. This is the same pattern already used correctly in `claude_parity_regression_test.rs` and `claude_refreshing_client_test.rs`.

For `claude_oauth_routing_test.rs` — all 7 tests need `#[serial]`:
```rust
use serial_test::serial;

#[test]
#[serial]
fn test_credential_detection_with_claude_auth_json() { ... }
```

For `claude_oauth_resolver_test.rs` — all 3 tests need `#[serial]`:
```rust
use serial_test::serial;

#[test]
#[serial]
fn test_credential_resolver_finds_oauth_tokens_from_claude_auth() { ... }
```

Verify `serial_test` is already in `[dev-dependencies]` for both `codelet-providers` and `codelet-napi` Cargo.toml (it is — used by other test files in the same packages).

---

## 🟡 FIX 2: Spec Drift — User-Agent Version

**Severity:** Low — cosmetic spec/code mismatch  
**Effort:** ~2 minutes

### Problem

PROV-020 rule [7] and example map example [3] say:
> `user-agent: claude-cli/2.1.2 (external, cli)`

The code uses:
> `claude-cli/2.1.3 (external, cli)` (in `CLAUDE_USER_AGENT` constant)

The parity regression test (PROV-027) asserts against `2.1.3`, so it passes — but the spec text is stale.

### Fix

Update PROV-020 rule [7] and example [3] to reference `2.1.3` instead of `2.1.2`. Or alternatively, update the constant to match the spec. Either way, spec and code must agree.

---

## 🟡 FIX 3: Feature File Clarification — `mcp_` Tool Prefixing Scope

**Severity:** Low — documentation clarity  
**Effort:** ~5 minutes

### Problem

PROV-020 rule [8] says:
> "Tool names must be prefixed with mcp_ when using OAuth mode"

PROV-027 rule [1] says:
> "Tool names must be prefixed with mcp_ in requests"

But the implementation explicitly does NOT apply `mcp_` prefixing in the production request path. The functions `prefix_tool_name()` / `strip_tool_name_prefix()` exist as parity reference implementations only. The code comments explain why:

> "Our tools are native (not MCP), so this function is not in the production request path. It exists for parity verification testing and for future MCP integration."

This is architecturally correct — opencode routes through MCP servers and needs the prefix; codelet uses native tools and doesn't. But the feature file rules read as if prefixing is actively applied to every request.

### Fix

Add an architecture note or docstring annotation to:
- `spec/features/claude-oauth-core.feature` scenario "Tool names prefixed with mcp_ in OAuth mode" 
- `spec/features/anthropic-oauth-parity.feature` scenario "Tool names are prefixed with mcp_ in OAuth mode requests"

Clarifying that these test the parity reference functions, not production request interception. Example annotation:

```gherkin
  # Architecture: mcp_ prefixing is a parity reference — codelet uses native tools
  # (not MCP), so prefixing is not applied in the production request path. These
  # functions exist for parity verification against opencode and future MCP support.
```

---

## 🟡 FIX 4: PROV-012 Parent Story Housekeeping

**Severity:** Low — workflow state  
**Effort:** ~2 minutes

### Problem

- PROV-012 lists children: PROV-021, PROV-022, PROV-023, PROV-024, PROV-026, PROV-027
- Missing from children list: **PROV-020** (core OAuth) and **PROV-025** (TUI settings)
- Both PROV-020 and PROV-025 are done and have `parent: PROV-012` or belong to the same epic
- PROV-012 itself is still in `specifying` status despite all child work being complete

### Fix

1. Add PROV-020 and PROV-025 as children of PROV-012 (if they aren't already linked via parent field)
2. After fixing the test failures (FIX 1), advance PROV-012 through the workflow to `done`
