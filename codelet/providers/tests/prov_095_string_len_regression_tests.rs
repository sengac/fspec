#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Feature: spec/features/custom-provider-script-shadowing-builtin-providers.feature
//!
//! PROV-095 — regression test for the screenshot-captured Rhai failure:
//!
//! ```text
//! API Error: Streaming error: ProviderError: [claude-rhai]
//!   Configuration error:
//!   script missing required function 'len (&str | ImmutableString | String)'
//!   (map_error)
//! ```
//!
//! Background
//! ----------
//! The user provided a screenshot showing that sending any message through
//! the `claude-rhai` custom provider produced the error above — raised from
//! the `map_error` function when it runs the line `if body.len() > 0 { … }`
//! against the (empty) HTTP error body.
//!
//! That error is emitted by
//! [`crate::custom::error_mapping::map_rhai_error_to_provider`] when the
//! Rhai engine raises `EvalAltResult::ErrorFunctionNotFound("len (&str …)")`
//! — i.e. the sandbox has no `.len()` function registered for strings.
//!
//! The prior regression suite (`prov_095_build_request_iterable_regression_tests.rs`)
//! only exercises `build_request` against array iteration. It does NOT
//! exercise `.len()` on a string, nor does it exercise `map_error`, nor
//! does it exercise the `build_request` path where a script calls
//! `body.len()` on a Rhai `Map` before an early return. The screenshot's
//! failure mode therefore slipped through the existing offline tests.
//!
//! This file adds a narrow, offline, zero-network regression suite that:
//!
//!   1. Compiles the `claude_rhai.rhai` script's `map_error` body verbatim
//!      and invokes it via `RhaiCustomProvider::invoke_map_error`, proving
//!      that the sandboxed engine exposes `.len()` on strings and does
//!      NOT raise `script missing required function 'len (&str …)'`.
//!   2. Exercises the engine directly with the smallest possible script
//!      that triggers the original regression (`fn call_len(s) { s.len() }`
//!      on a `&str` arg) — this is the unit-level guard. If someone drops
//!      `CorePackage` / `BasicStringPackage` from `build_sandboxed_engine`
//!      in the future, this test fails immediately with a clear message.
//!   3. Also verifies `.len()` works against `Map` values (as used by
//!      `claude_rhai.rhai`'s `build_request` at `if body.len() > 0`), and
//!      against `Array` values (already covered elsewhere but pinned here
//!      as a belt-and-braces check so the full "string | map | array"
//!      triad stays in one regression file).
//!
//! Unlike the previous e2e test, this test:
//!
//!   - does not require `ANTHROPIC_API_KEY`,
//!   - does not perform a network call,
//!   - does not skip on CI,
//!   - does not rely on a developer's local `~/.fspec/providers/` files.
//!
//! If the sandbox regresses to the state captured in the screenshot, every
//! scenario here fails with a diagnostic pointing at the exact missing
//! function signature (`len (&str | ImmutableString | String)`).

#[path = "custom_http_test_helpers.rs"]
mod helpers;

use std::sync::Arc;

use codelet_providers::custom::{RhaiCustomProvider, ScriptLoader};
use codelet_providers::oauth::engine::build_default_engine;
use codelet_providers::ProviderError;
use helpers::config_with_full_script;
use rhai::{Dynamic, Engine};

// ---------------------------------------------------------------------------
// A script containing the 7 required provider functions, with `map_error`
// copied VERBATIM from ~/.fspec/providers/claude_rhai.rhai — this is the
// exact body that surfaced the screenshot's error. The other 6 functions
// are minimal stubs so `ScriptLoader::validate_required_functions`
// accepts the script; `map_error` is the one under test.
//
// The key line is `if body.len() > 0 { … }` — `body` is bound to a `String`
// by `invoke_map_error`, and the sandboxed engine must expose `.len()` on
// strings for this to succeed.
// ---------------------------------------------------------------------------
const CLAUDE_RHAI_MAP_ERROR_SCRIPT: &str = r#"
fn api_token() { "sk-ant-test-key-offline" }

fn build_url(config) {
    config.base_url + "/v1/messages"
}

fn build_headers(config) {
    #{ "Content-Type": "application/json" }
}

fn build_request(request) {
    #{ model: "claude-opus-4-7", max_tokens: 128, messages: [] }
}

fn parse_response(raw) {
    #{ content: [], stop_reason: "end_turn" }
}

fn parse_stream_chunk(chunk) {
    #{ kind: "ignore" }
}

fn build_stream_request(request) {
    let body = build_request(request);
    body.stream = true;
    body
}

// ----- VERBATIM from ~/.fspec/providers/claude_rhai.rhai -------------------
fn map_error(status, body) {
    let msg = body;

    if body.len() > 0 {
        let parsed = json::parse(body);
        if type_of(parsed) != "()" && type_of(parsed.error) != "()" {
            if type_of(parsed.error.message) == "string" {
                msg = parsed.error.message;
            }
        }
    }

    if status == 401 || status == 403 {
        return #{ type: "auth", message: msg };
    }
    if status == 429 {
        return #{ type: "rate_limit", message: msg };
    }
    if status == 529 {
        return #{ type: "api", message: "Anthropic API overloaded: " + msg };
    }

    #{ type: "api", message: "HTTP " + status + ": " + msg }
}
"#;

// ---------------------------------------------------------------------------
// Unit guard at the engine level — the smallest possible script that pins
// `.len()` on strings. If CorePackage / BasicStringPackage is ever dropped
// from `build_sandboxed_engine`, this test fails with a diagnostic that
// name-checks the exact signature from the screenshot.
// ---------------------------------------------------------------------------

fn run_call_len(engine: &Engine, script: &str, arg: Dynamic) -> Result<i64, String> {
    let ast = engine
        .compile(script)
        .map_err(|e| format!("compile error: {e}"))?;
    let mut scope = rhai::Scope::new();
    engine
        .call_fn::<i64>(&mut scope, &ast, "call_len", (arg,))
        .map_err(|e| format!("{e}"))
}

#[test]
fn sandboxed_engine_exposes_len_on_strings() {
    // @step Given the default sandboxed Rhai engine used by every custom
    //       provider (same factory as `ScriptLoader::with_default_engine`)
    let engine = build_default_engine();

    // @step And a minimal Rhai script that calls `.len()` on its argument
    let script = r#"
        fn call_len(s) { s.len() }
    "#;

    // @step When I invoke `call_len` with a String argument
    //       (this is the exact shape `invoke_map_error` passes in — the
    //        HTTP body is wrapped with `Dynamic::from(body.to_string())`
    //        in `RhaiCustomProvider::invoke_map_error`)
    let result = run_call_len(
        &engine,
        script,
        Dynamic::from("hello".to_string()),
    );

    // @step Then the engine does NOT raise
    //       `script missing required function 'len (&str | ImmutableString | String)'`
    // @step And the returned length is 5
    match result {
        Ok(len) => assert_eq!(len, 5, "String.len() should return 5 for \"hello\""),
        Err(msg) => {
            if msg.contains("Function not found: len") || msg.contains("missing required function 'len") {
                panic!(
                    "PROV-095 STRING REGRESSION: the sandboxed engine no longer exposes \
                     `.len()` on strings — this is the exact failure shown in the \
                     user's screenshot (`script missing required function \
                     'len (&str | ImmutableString | String)'`). Raw error: {msg}"
                );
            }
            panic!("unexpected engine error while calling `call_len(\"hello\")`: {msg}");
        }
    }
}

#[test]
fn sandboxed_engine_exposes_len_on_immutable_string() {
    // @step Given the default sandboxed Rhai engine
    let engine = build_default_engine();

    // @step And a script that calls `.len()` on an ImmutableString
    //       (this is the Rhai-native string type — a different branch of
    //        the signature `len (&str | ImmutableString | String)` from
    //        the screenshot error.)
    let script = r#"
        fn call_len(s) { s.len() }
    "#;

    let result = run_call_len(
        &engine,
        script,
        Dynamic::from(rhai::ImmutableString::from("world!")),
    );

    match result {
        Ok(len) => assert_eq!(len, 6),
        Err(msg) => panic!(
            "PROV-095 STRING REGRESSION: sandbox engine cannot call `.len()` on an \
             ImmutableString. Raw error: {msg}"
        ),
    }
}

#[test]
fn sandboxed_engine_exposes_len_on_maps() {
    // @step Given the default sandboxed Rhai engine
    let engine = build_default_engine();

    // @step And a script that builds an empty map and reads `.len()`
    //       (this mirrors `if body.len() > 0` in the production
    //        `build_request` at `~/.fspec/providers/claude_rhai.rhai`,
    //        line 185 — the third distinct `.len()` call site in the
    //        real script)
    let script = r#"
        fn empty_map_len() {
            let m = #{};
            m.len()
        }
    "#;
    let ast = engine.compile(script).expect("compile empty_map_len");
    let mut scope = rhai::Scope::new();
    let len: i64 = engine
        .call_fn(&mut scope, &ast, "empty_map_len", ())
        .unwrap_or_else(|e| {
            panic!(
                "PROV-095 MAP REGRESSION: `.len()` is not registered for Rhai maps \
                 in the sandboxed engine. Raw error: {e}"
            )
        });
    // @step Then the engine returns 0 for an empty map
    assert_eq!(len, 0);
}

#[test]
fn sandboxed_engine_exposes_len_on_arrays() {
    // @step Given the default sandboxed Rhai engine
    let engine = build_default_engine();

    // @step And a script that calls `.len()` on an Array
    //       (already covered by the build_request iteration tests but
    //        pinned here so the "string | map | array" triad stays
    //        together in one regression file)
    let script = r#"
        fn arr_len() {
            let a = ["one", "two", "three"];
            a.len()
        }
    "#;
    let ast = engine.compile(script).expect("compile arr_len");
    let mut scope = rhai::Scope::new();
    let len: i64 = engine
        .call_fn(&mut scope, &ast, "arr_len", ())
        .unwrap_or_else(|e| {
            panic!(
                "PROV-095 ARRAY REGRESSION: `.len()` is not registered for Rhai \
                 arrays in the sandboxed engine. Raw error: {e}"
            )
        });
    // @step Then the engine returns 3 for a 3-element array
    assert_eq!(len, 3);
}

// ---------------------------------------------------------------------------
// Integration guard — end-to-end through the real provider API
// (`invoke_map_error`), replicating what the production agent loop does
// when a non-2xx response comes back from Anthropic. This is the closest
// offline reproduction of the screenshot path.
// ---------------------------------------------------------------------------

fn build_provider() -> (tempfile::TempDir, RhaiCustomProvider) {
    let (tmp, cfg) = config_with_full_script(
        "claude-rhai",
        "https://api.anthropic.com",
        "claude-opus-4-7",
        CLAUDE_RHAI_MAP_ERROR_SCRIPT,
    );
    let loader = Arc::new(ScriptLoader::with_default_engine());
    let provider = RhaiCustomProvider::new(Arc::new(cfg), loader, "smart".to_string())
        .expect("construct RhaiCustomProvider with embedded map_error script");
    (tmp, provider)
}

#[tokio::test]
async fn invoke_map_error_handles_non_empty_body_without_string_len_regression() {
    // @step Given the claude-rhai map_error script (verbatim from the user's
    //       ~/.fspec/providers/claude_rhai.rhai)
    let (_tmp, provider) = build_provider();

    // @step When the agent loop receives a non-2xx response with a JSON body
    //       and forwards it to `map_error` via `invoke_map_error`
    let body = r#"{"error":{"message":"invalid api key"}}"#;
    let error = provider.invoke_map_error(401, body).await;

    // @step Then the engine does NOT raise
    //       `script missing required function 'len (&str | ImmutableString | String)'`
    //       at `if body.len() > 0`. The script should recognize 401 as auth.
    match error {
        ProviderError::Authentication { message, .. } => {
            // The script extracts `parsed.error.message` from the body.
            assert!(
                message.contains("invalid api key"),
                "expected parsed Anthropic error message, got {message:?}"
            );
        }
        ProviderError::Configuration { message, .. }
            if message.contains("missing required function 'len") =>
        {
            panic!(
                "PROV-095 REGRESSION REPRODUCED: invoke_map_error surfaced the exact \
                 error from the user's screenshot — `{message}`. The sandboxed Rhai \
                 engine is no longer exposing `.len()` on strings."
            );
        }
        other => panic!(
            "unexpected ProviderError from invoke_map_error with a 401 body: {other:?}"
        ),
    }
}

#[tokio::test]
async fn invoke_map_error_handles_empty_body_without_string_len_regression() {
    // @step Given the claude-rhai map_error script
    let (_tmp, provider) = build_provider();

    // @step When the agent loop receives a non-2xx response with an EMPTY body
    //       (this is the SPECIFIC shape shown in the user's screenshot — a
    //        streaming failure that produces an empty body but still goes
    //        through `map_error`, hitting `if body.len() > 0` which is the
    //        line that raised the original "missing required function 'len'"
    //        error).
    let body = "";
    let error = provider.invoke_map_error(500, body).await;

    // @step Then no "missing required function 'len …'" is raised
    // @step And the resulting ProviderError carries the HTTP status
    match error {
        ProviderError::Configuration { message, .. }
            if message.contains("missing required function 'len") =>
        {
            panic!(
                "PROV-095 REGRESSION REPRODUCED with empty body: `{message}`. \
                 This is the exact failure the user reported — the screenshot \
                 showed `Configuration error: script missing required function \
                 'len (&str | ImmutableString | String)' (map_error)`."
            );
        }
        ProviderError::Api { message, .. } => {
            // Script returns `#{ type: "api", message: "HTTP 500: " }` for 5xx.
            assert!(
                message.starts_with("HTTP 500"),
                "expected `HTTP 500…` message from map_error, got {message:?}"
            );
        }
        other => panic!(
            "unexpected ProviderError from invoke_map_error with empty body: {other:?}"
        ),
    }
}
