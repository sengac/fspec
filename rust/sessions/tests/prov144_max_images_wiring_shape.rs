//! PROV-144: Source-shape regression tests pinning the per-session image
//! budget wiring across the five `set_session_model_vision` set-sites and
//! the destroy-session clear path, plus the no-env-bridge invariant.
//!
//! Feature: spec/features/per-profile-max-images-session-wiring.feature
//!
//! Rule [2] of the work unit: the effective image budget is stored in the
//! tool-layer session capability registry at session creation and on every
//! mid-session model switch (all five set-sites that call
//! `set_session_model_vision`) and cleared on session destroy alongside the
//! vision entry. Rule: `maxImages` is a tool-layer concern only — it is NOT
//! bridged into `OPENAI_*` env vars by `apply_profile_env_vars`.
//!
//! Pattern mirrors `mcp_injection_source_shape.rs` (RPC-062): comment-stripped
//! substring scans + `extract_fn_body` for the negative env-bridge check.
//!
//! RED PHASE: `set_session_model_max_images` / `clear_session_model_max_images`
//! / `resolve_profile_max_images` do not exist in production source yet, so
//! these tests FAIL until the implementation lands.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::{Path, PathBuf};

// =============================================================================
// Path / read helpers — sibling of mcp_injection_source_shape.rs
// =============================================================================

fn workspace_root() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .expect("codelet-sessions manifest dir must have a parent")
        .to_path_buf()
}

fn read(path: &Path) -> String {
    std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()))
}

/// Strip both `//` line comments and `/* ... */` block comments from
/// Rust source so that substring scans don't get fooled by needle
/// references inside doc comments.
fn strip_rust_comments(src: &str) -> String {
    let bytes = src.as_bytes();
    let mut out = String::with_capacity(src.len());
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];
        let next = bytes.get(i + 1).copied();
        if b == b'/' && next == Some(b'/') {
            while i < bytes.len() && bytes[i] != b'\n' {
                i += 1;
            }
        } else if b == b'/' && next == Some(b'*') {
            i += 2;
            while i + 1 < bytes.len() && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                i += 1;
            }
            i = (i + 2).min(bytes.len());
        } else {
            out.push(b as char);
            i += 1;
        }
    }
    out
}

/// Walk the comment-stripped source and return the substring between the
/// byte where `header` first appears and the matching closing `}` of that
/// function body (best-effort brace counter).
fn extract_fn_body<'a>(src: &'a str, header: &str) -> &'a str {
    let start = src
        .find(header)
        .unwrap_or_else(|| panic!("expected to find function header `{header}` in source"));
    let body_start_rel = src[start..]
        .find('{')
        .unwrap_or_else(|| panic!("expected an opening `{{` after `{header}`"));
    let body_start = start + body_start_rel + 1;
    let bytes = src.as_bytes();
    let mut depth = 1usize;
    let mut i = body_start;
    while i < bytes.len() {
        match bytes[i] {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return &src[body_start..i];
                }
            }
            _ => {}
        }
        i += 1;
    }
    panic!("function body for `{header}` not terminated by a matching `}}`")
}

fn source(rel: &str) -> String {
    strip_rust_comments(&read(&workspace_root().join(rel)))
}

// =============================================================================
// Scenario: Session creation and model-switch set-sites register the
//           max-images budget
// =============================================================================

#[test]
fn scenario_shared_create_helper_sets_the_max_images_budget() {
    // @step Given the shared create, isolated create, mid-session set_model, and both NAPI model-switch set-sites exist
    let src = source("sessions/src/session_creation_helper.rs");

    // @step When the max-images budget is resolved through the shared resolver
    let set_count = src.matches("set_session_model_max_images(").count();

    // @step Then each set-site registers the budget alongside the vision entry
    assert_eq!(
        set_count, 1,
        "expected exactly one `set_session_model_max_images(` call in \
         session_creation_helper.rs (the shared create path), found {set_count}"
    );

    // Asserts: budget comes from the shared resolver, not an inline profile lookup
    assert!(
        src.contains("resolve_profile_max_images("),
        "session_creation_helper.rs must source the budget from \
         `resolve_profile_max_images` so the create path cannot drift"
    );
}

#[test]
fn scenario_isolated_session_create_sets_the_max_images_budget() {
    // The file rust/sessions/src/session_manager.rs exists
    let src = source("sessions/src/session_manager.rs");

    // The isolated-session create path is inspected
    let set_count = src.matches("set_session_model_max_images(").count();

    // Asserts: it registers the max-images budget alongside the vision entry
    assert_eq!(
        set_count, 1,
        "expected exactly one `set_session_model_max_images(` call in \
         session_manager.rs (create_isolated_session_with_id), found {set_count}"
    );

    // Asserts: budget comes from the shared resolver
    assert!(
        src.contains("resolve_profile_max_images("),
        "session_manager.rs must source the budget from `resolve_profile_max_images`"
    );
}

#[test]
fn scenario_mid_session_set_model_updates_the_max_images_budget() {
    // The file rust/sessions/src/handle_impl.rs exists
    let src = source("sessions/src/handle_impl.rs");

    // The mid-session set_model path is inspected
    let set_count = src.matches("set_session_model_max_images(").count();

    // Asserts: it updates the max-images budget alongside the vision entry
    assert_eq!(
        set_count, 1,
        "expected exactly one `set_session_model_max_images(` call in \
         handle_impl.rs (set_model), found {set_count}"
    );

    // Asserts: budget comes from the shared resolver
    assert!(
        src.contains("resolve_profile_max_images("),
        "handle_impl.rs must source the budget from `resolve_profile_max_images`"
    );
}

#[test]
fn scenario_napi_model_switches_update_the_max_images_budget() {
    // The file rust/napi/src/session_bindings.rs exists
    let src = source("napi/src/session_bindings.rs");

    // The two NAPI model-switch bindings are inspected
    let set_count = src.matches("set_session_model_max_images(").count();

    // Asserts: both session_set_model and session_set_model_profile register the budget
    assert_eq!(
        set_count, 2,
        "expected exactly two `set_session_model_max_images(` calls in \
         session_bindings.rs (session_set_model + session_set_model_profile), \
         found {set_count}"
    );

    // Asserts: both source the budget from the shared resolver
    let resolver_count = src.matches("resolve_profile_max_images(").count();
    assert_eq!(
        resolver_count, 2,
        "expected exactly two `resolve_profile_max_images(` call sites in \
         session_bindings.rs (one per NAPI model-switch binding), \
         found {resolver_count}"
    );
}

#[test]
fn scenario_destroy_session_clears_the_max_images_budget() {
    // The file rust/sessions/src/session_manager.rs exists
    let src = source("sessions/src/session_manager.rs");

    // @step And the session destroy path clears the budget alongside the vision entry
    let destroy_body = extract_fn_body(&src, "pub fn destroy_session");
    let clear_count = destroy_body
        .matches("clear_session_model_max_images(uuid)")
        .count();

    // Asserts: the max-images entry is cleared alongside the vision entry
    assert_eq!(
        clear_count, 1,
        "expected exactly one `clear_session_model_max_images(uuid)` inside \
         destroy_session (alongside clear_session_model_vision), found {clear_count}"
    );
    assert!(
        destroy_body.contains("clear_session_model_vision(uuid)"),
        "the existing vision-entry clear must remain in destroy_session"
    );
}

// =============================================================================
// Scenario: The max-images resolver is defined once and not bridged into
//           env vars
// =============================================================================

#[test]
fn scenario_the_resolver_is_defined_exactly_once() {
    // @step Given the model resolution module defines resolve_profile_max_images
    let src = source("sessions/src/model_resolution.rs");

    // @step When the shared resolver and the OPENAI_* env bridge are inspected
    let def_count = src.matches("pub fn resolve_profile_max_images").count();

    // @step Then the resolver is defined exactly once
    assert_eq!(
        def_count, 1,
        "expected exactly one `pub fn resolve_profile_max_images` definition \
         in model_resolution.rs, found {def_count}"
    );
}

#[test]
fn scenario_max_images_is_not_bridged_into_openai_env_vars() {
    // The file rust/sessions/src/model_resolution.rs exists
    let src = source("sessions/src/model_resolution.rs");

    // The apply_profile_env_vars body is inspected
    let body = extract_fn_body(&src, "pub fn apply_profile_env_vars");

    // @step And the env bridge does not reference the max-images value
    // (maxImages is a tool-layer concern only; bridging it into OPENAI_*
    // would leak a tool budget into the provider client)
    assert!(
        !body.contains("max_images"),
        "apply_profile_env_vars must NOT reference max_images — the image \
         budget is a tool-layer concern, not a provider env var"
    );
    assert!(
        !body.contains("MAX_IMAGES"),
        "apply_profile_env_vars must NOT set an OPENAI_MAX_IMAGES-style env var"
    );
}
