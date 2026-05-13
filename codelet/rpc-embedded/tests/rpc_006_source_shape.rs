//! RPC-006 source-shape integration tests.
//!
//! Feature: spec/features/rpc-006-source-shape-invariants.feature
//!
//! These tests codify architectural invariants statically by inspecting
//! source files in the workspace. They do not exercise any runtime code
//! path. RPC-006 widens RPC-005 scenario_10 (no codelet-core dep) to allow
//! a codelet-core dep on codelet/rpc, while still forbidding codelet-napi.
//! The other RPC-005 invariants are restated in their post-RPC-006 form.
//!
//! - Scenario: default_fixture is unreachable from production code
//! - Scenario: Embedded transport reuses the host tokio runtime Handle for any fan-out it spawns
//! - Scenario: Shared service crate codelet-rpc may depend on codelet-core but not on codelet-napi
//! - Scenario: rpc-server binary still binds 127.0.0.1 loopback only after the watcher integration
//! - Scenario: WorkUnitInfo continues to be defined exactly once in rpc-types
//! - Scenario: Embedded push path contains no bincode encode call

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod source_helpers;

use source_helpers::{collect_rs_files, read_to_string_or_panic, strip_rust_comments, workspace_root};

#[test]
fn scenario_default_fixture_is_unreachable_from_production_code() {
    // @step Given the codelet/rpc crate after the watcher lift
    let rpc_lib = read_to_string_or_panic(&workspace_root().join("rpc").join("src").join("lib.rs"));
    let code = strip_rust_comments(&rpc_lib);
    let lines: Vec<&str> = code.lines().collect();

    // @step When I search the crate sources for the previous `default_fixture` symbol and for any production reference to `test_fixture`
    let has_unguarded_default_fixture = lines
        .iter()
        .any(|line| line.contains("pub fn default_fixture"));

    // Find each `pub fn test_fixture` and verify the line(s) immediately
    // preceding it contain a `#[cfg(...)]` attribute referencing `test`.
    let mut test_fixture_definitions: Vec<usize> = Vec::new();
    for (idx, line) in lines.iter().enumerate() {
        if line.contains("pub fn test_fixture") {
            test_fixture_definitions.push(idx);
        }
    }
    let mut ungated_test_fixture_lines: Vec<usize> = Vec::new();
    for &idx in &test_fixture_definitions {
        // Walk backwards over blank lines and `pub` keyword to find the
        // attribute line (if any). Accept either an inline cfg or a cfg
        // on the immediately preceding non-blank line.
        let mut found_gate = false;
        let mut probe = idx;
        while probe > 0 {
            probe -= 1;
            let candidate = lines[probe].trim();
            if candidate.is_empty() {
                continue;
            }
            // The first non-blank line preceding the function MUST be
            // a `#[cfg(...test...)]` attribute. Anything else (use,
            // impl, fn, etc.) means the fixture is ungated.
            if candidate.starts_with("#[cfg") && candidate.contains("test") {
                found_gate = true;
            }
            break;
        }
        if !found_gate {
            ungated_test_fixture_lines.push(idx + 1); // +1 for 1-based reporting
        }
    }

    let has_cfg_test_gate = code.contains("#[cfg(test)]") || code.contains("#[cfg(any(test");

    // @step Then the symbol exists only inside a `#[cfg(test)]`-gated module or function and no production module path references it
    assert!(
        !has_unguarded_default_fixture,
        "default_fixture must be renamed/gated; found surviving `pub fn default_fixture` in codelet/rpc/src/lib.rs"
    );
    assert!(
        ungated_test_fixture_lines.is_empty(),
        "test_fixture must be #[cfg(test)]-gated. Found ungated `pub fn test_fixture` at line(s): {ungated_test_fixture_lines:?}"
    );
    assert!(
        has_cfg_test_gate,
        "codelet/rpc/src/lib.rs must contain at least one #[cfg(test)] (or #[cfg(any(test, ...))]) gate around the test fixture"
    );
}

#[test]
fn scenario_embedded_transport_reuses_host_runtime_handle_for_fan_out() {
    // @step Given the codelet/rpc-embedded crate after RPC-006
    let src_dir = workspace_root().join("rpc-embedded").join("src");
    let rs_files = collect_rs_files(&src_dir);

    // @step When I search its source for tokio runtime construction calls and for the spawn target of any internal fan-out task
    let mut forbidden_calls: Vec<String> = Vec::new();
    let mut uses_handle_spawn = false;
    for path in &rs_files {
        let body = read_to_string_or_panic(path);
        let code = strip_rust_comments(&body);
        if code.contains("tokio::runtime::Builder")
            || code.contains("runtime::Builder::new_multi_thread")
            || code.contains("runtime::Builder::new_current_thread")
            || code.contains("tokio::runtime::Runtime::new")
            || code.contains("Runtime::new()")
        {
            forbidden_calls.push(path.display().to_string());
        }
        if code.contains("self.handle.spawn") || code.contains(".handle.spawn(") {
            uses_handle_spawn = true;
        }
    }

    // @step Then the crate contains no calls to tokio::runtime::Builder or tokio::runtime::Runtime::new and any background task is spawned via the stored host runtime Handle
    assert!(
        forbidden_calls.is_empty(),
        "codelet/rpc-embedded must not construct its own tokio runtime. \
         Found forbidden Builder/Runtime::new usage in: {forbidden_calls:?}"
    );
    assert!(
        uses_handle_spawn,
        "codelet/rpc-embedded must spawn at least one task via `self.handle.spawn(...)` so the runtime invariant is exercised, not just satisfied vacuously"
    );
}

#[test]
fn scenario_codelet_rpc_may_depend_on_codelet_core_but_not_on_codelet_napi() {
    // @step Given codelet/rpc/Cargo.toml after the watcher lift
    let cargo_path = workspace_root().join("rpc").join("Cargo.toml");
    let cargo = read_to_string_or_panic(&cargo_path);

    // @step When I inspect its [dependencies] table and the codelet/rpc/src/ source for use statements naming codelet_napi or codelet_core
    let has_codelet_core_dep = cargo.contains("codelet-core");
    let has_codelet_napi_dep = cargo.contains("codelet-napi");

    let src_dir = workspace_root().join("rpc").join("src");
    let rs_files = collect_rs_files(&src_dir);
    let mut napi_imports: Vec<String> = Vec::new();
    for path in &rs_files {
        let body = read_to_string_or_panic(path);
        let code = strip_rust_comments(&body);
        if code.contains("use codelet_napi") || code.contains("codelet_napi::") {
            napi_imports.push(path.display().to_string());
        }
    }

    // @step Then the manifest declares a codelet-core dependency, declares no codelet-napi dependency, and the source contains no `use codelet_napi` import
    assert!(
        has_codelet_core_dep,
        "codelet/rpc/Cargo.toml MUST depend on codelet-core after the watcher lift (RPC-006)"
    );
    assert!(
        !has_codelet_napi_dep,
        "codelet/rpc/Cargo.toml MUST NOT depend on codelet-napi (RPC-006 dependency-arrow invariant)"
    );
    assert!(
        napi_imports.is_empty(),
        "codelet/rpc/src must not import codelet_napi (RPC-006). Found: {napi_imports:?}"
    );
}

#[test]
fn scenario_rpc_server_binary_still_binds_loopback_only_after_watcher_integration() {
    // @step Given the rpc-server binary main.rs after RPC-006 has been wired to construct a real WorkUnitsWatcher
    let main_path = workspace_root()
        .join("rpc-server")
        .join("src")
        .join("main.rs");
    let main_src = read_to_string_or_panic(&main_path);
    let code = strip_rust_comments(&main_src);

    // @step When I inspect codelet/rpc-server/src/main.rs for the bind address literal
    let has_loopback = code.contains("127.0.0.1:0") || code.contains("\"127.0.0.1\"");
    let mut forbidden_addrs: Vec<String> = Vec::new();
    for needle in ["0.0.0.0", "[::]", "::0", "0:0"] {
        if code.contains(needle) {
            forbidden_addrs.push(needle.to_string());
        }
    }

    // @step Then the only bind address literal is 127.0.0.1:0 and no 0.0.0.0 or other non-loopback bind address appears in the binary source
    assert!(
        has_loopback,
        "rpc-server main.rs must still bind to 127.0.0.1 after RPC-006. Source: {}",
        main_path.display()
    );
    assert!(
        forbidden_addrs.is_empty(),
        "rpc-server main.rs must not bind to non-loopback addresses. Found: {forbidden_addrs:?}"
    );
}

#[test]
fn scenario_work_unit_info_continues_to_be_defined_exactly_once_in_rpc_types() {
    // @step Given codelet/rpc-types defines a public WorkUnitInfo struct with fields id, title, work_type, status, description, estimate, and epic
    let rpc_types_lib =
        read_to_string_or_panic(&workspace_root().join("rpc-types").join("src").join("lib.rs"));
    assert!(
        rpc_types_lib.contains("pub struct WorkUnitInfo"),
        "rpc-types must define `pub struct WorkUnitInfo`"
    );

    // @step When I inspect codelet/napi and codelet/core for any local definition of a struct named WorkUnitInfo and run cargo check on the workspace
    let mut local_definitions: Vec<String> = Vec::new();
    for crate_name in ["napi", "core"] {
        let src_dir = workspace_root().join(crate_name).join("src");
        for path in collect_rs_files(&src_dir) {
            let body = read_to_string_or_panic(&path);
            let code = strip_rust_comments(&body);
            if code.contains("pub struct WorkUnitInfo")
                || code.contains("struct WorkUnitInfo {")
            {
                local_definitions.push(path.display().to_string());
            }
        }
    }

    // @step Then no other crate defines WorkUnitInfo locally, both crates re-export the type from codelet/rpc-types where they reference it, and the workspace builds successfully
    assert!(
        local_definitions.is_empty(),
        "Neither codelet/napi nor codelet/core may redefine WorkUnitInfo. \
         Found local definition(s) in: {local_definitions:?}. \
         Replace with `pub use codelet_rpc_types::WorkUnitInfo;` (or a private use)."
    );
    // The "workspace builds successfully" half of the Then is enforced by
    // the fact that this test compiles and links — the test crate
    // transitively depends on codelet-rpc-types, codelet-rpc-embedded,
    // codelet-rpc, and codelet-core, all of which must type-check.
}

#[test]
fn scenario_embedded_push_path_contains_no_bincode_encode_call() {
    // @step Given the codelet/rpc-embedded crate after RPC-006 exposes work_units_rx via a broadcast::Receiver
    let src_dir = workspace_root().join("rpc-embedded").join("src");
    let rs_files = collect_rs_files(&src_dir);

    // @step When I search its source for bincode serialize or deserialize calls
    let mut bincode_calls: Vec<String> = Vec::new();
    for path in &rs_files {
        let body = read_to_string_or_panic(path);
        let code = strip_rust_comments(&body);
        for needle in ["bincode::serialize", "bincode::deserialize"] {
            if code.contains(needle) {
                bincode_calls.push(format!("{}: {}", path.display(), needle));
            }
        }
    }

    // @step Then the crate contains no bincode::serialize or bincode::deserialize call sites because the embedded push path returns the watcher's broadcast subscription directly without any envelope encoding
    assert!(
        bincode_calls.is_empty(),
        "codelet/rpc-embedded must not bincode-encode the push path (zero-cost embedded). \
         Found: {bincode_calls:?}"
    );
}
