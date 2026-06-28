//! Source-shape integration tests for RPC-005 architecture invariants.
//!
//! Feature: spec/features/rpc-architecture-invariants.feature
//!
//! These tests codify architectural invariants statically by inspecting
//! source files in the workspace. They do not exercise any runtime code
//! path.
//!
//! - Scenario 7: EmbeddedTransport requires a tokio runtime Handle at construction
//! - Scenario 8: WorkUnitInfo is defined once in rpc-types and re-exported by codelet-napi
//! - Scenario 9: Embedded transport uses only tarpc::transport::channel for in-process traffic
//! - Scenario 10: Shared service crate codelet-rpc has no dependency on codelet-core or the NAPI work_units_watcher
//! - Scenario 11: rpc-server binary binds to 127.0.0.1 loopback only

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod source_helpers;

use source_helpers::{
    collect_rs_files, read_to_string_or_panic, strip_rust_comments, workspace_root,
};

#[test]
fn scenario_7_embedded_transport_requires_tokio_handle_at_construction() {
    // @step Given the codelet/rpc-embedded crate exposes an EmbeddedTransport public constructor
    let lib_path = workspace_root()
        .join("rpc-embedded")
        .join("src")
        .join("lib.rs");
    let lib_src = read_to_string_or_panic(&lib_path);

    // @step When I inspect the public type signature of the constructor and search the codelet/rpc-embedded source for runtime construction calls
    let has_pub_struct = lib_src.contains("pub struct EmbeddedTransport");
    let has_handle_ctor = lib_src.contains(
        "pub fn new(handle: tokio::runtime::Handle, service: Arc<SharedFspecService>) -> Self",
    );

    let src_dir = workspace_root().join("rpc-embedded").join("src");
    let rs_files = collect_rs_files(&src_dir);
    let mut forbidden_calls: Vec<String> = Vec::new();
    for path in &rs_files {
        let body = read_to_string_or_panic(path);
        let code = strip_rust_comments(&body);
        // Match on the call sites that would create a SEPARATE runtime.
        // tokio::runtime::Handle::current() is allowed; Builder / Runtime::new are not.
        if code.contains("tokio::runtime::Builder")
            || code.contains("runtime::Builder::new_multi_thread")
            || code.contains("runtime::Builder::new_current_thread")
            || code.contains("tokio::runtime::Runtime::new")
            || code.contains("Runtime::new()")
        {
            forbidden_calls.push(format!("rpc-embedded/{}", path.display()));
        }
    }

    // RPC-008 widening: the same forbidden-runtime invariant applies to the
    // new codelet-fspec-tui crate. We scan codelet/fspec-tui/src/ with the
    // identical needle list and prefix the violation path with the crate
    // name so a future failure says "fspec-tui/..." vs "rpc-embedded/...".
    // Per the RPC-008 architecture rule [19] / scenario "The existing
    // scenario_7_* test in rpc-embedded is widened to scan fspec-tui".
    let fspec_tui_src_dir = workspace_root().join("fspec-tui").join("src");
    if fspec_tui_src_dir.exists() {
        let fspec_tui_rs_files = collect_rs_files(&fspec_tui_src_dir);
        for path in &fspec_tui_rs_files {
            let body = read_to_string_or_panic(path);
            let code = strip_rust_comments(&body);
            if code.contains("tokio::runtime::Builder")
                || code.contains("runtime::Builder::new_multi_thread")
                || code.contains("runtime::Builder::new_current_thread")
                || code.contains("tokio::runtime::Runtime::new")
                || code.contains("Runtime::new()")
            {
                forbidden_calls.push(format!("fspec-tui/{}", path.display()));
            }
        }
    }

    // RPC-010 widening: the same forbidden-runtime invariant applies to
    // the new codelet/fspec/src/ binary crate. Per RPC-010 rule [7] +
    // arch note [3]: `#[tokio::main]` is the ONLY runtime source; every
    // downstream call must use `tokio::runtime::Handle::current()`.
    let fspec_src_dir = workspace_root().join("fspec").join("src");
    if fspec_src_dir.exists() {
        let fspec_rs_files = collect_rs_files(&fspec_src_dir);
        for path in &fspec_rs_files {
            let body = read_to_string_or_panic(path);
            let code = strip_rust_comments(&body);
            if code.contains("tokio::runtime::Builder")
                || code.contains("runtime::Builder::new_multi_thread")
                || code.contains("runtime::Builder::new_current_thread")
                || code.contains("tokio::runtime::Runtime::new")
                || code.contains("Runtime::new()")
            {
                forbidden_calls.push(format!("fspec/src/{}", path.display()));
            }
        }
    }

    // @step Then the constructor takes a tokio::runtime::Handle as a non-defaulted argument and the codelet/rpc-embedded source contains no calls to tokio::runtime::Builder or tokio::runtime::Runtime::new that would create a separate runtime
    assert!(
        has_pub_struct,
        "EmbeddedTransport must be a public struct in {}",
        lib_path.display()
    );
    assert!(
        has_handle_ctor,
        "EmbeddedTransport::new must take `handle: tokio::runtime::Handle` as a \
         non-defaulted argument. Source did not match the expected signature:\n\
         pub fn new(handle: tokio::runtime::Handle, service: Arc<SharedFspecService>) -> Self"
    );
    assert!(
        forbidden_calls.is_empty(),
        "codelet/rpc-embedded and codelet/fspec-tui must not construct their own tokio runtime. \
         Found forbidden Builder/Runtime::new usage in: {forbidden_calls:?}"
    );
}

#[test]
fn scenario_8_work_unit_info_is_defined_once_in_rpc_types_and_re_exported_by_napi() {
    // @step Given codelet/rpc-types defines a public WorkUnitInfo struct with fields id, title, work_type, status, description, estimate, and epic
    let rpc_types_lib = read_to_string_or_panic(
        &workspace_root()
            .join("rpc-types")
            .join("src")
            .join("lib.rs"),
    );
    let required_fields = [
        "pub id: String",
        "pub title: String",
        "pub work_type: String",
        "pub status: String",
        "pub description: Option<String>",
        "pub estimate: Option<i32>",
        "pub epic: Option<String>",
    ];
    assert!(
        rpc_types_lib.contains("pub struct WorkUnitInfo"),
        "rpc-types must define `pub struct WorkUnitInfo`"
    );
    for field in required_fields {
        assert!(
            rpc_types_lib.contains(field),
            "rpc-types::WorkUnitInfo missing field: {field}"
        );
    }

    // @step When I inspect codelet/napi for definitions of WorkUnitInfo and run cargo check on the codelet workspace
    let napi_src_dir = workspace_root().join("napi").join("src");
    let napi_rs_files = collect_rs_files(&napi_src_dir);
    let mut local_definitions: Vec<String> = Vec::new();
    let mut has_re_export = false;
    for path in &napi_rs_files {
        let body = read_to_string_or_panic(path);
        let code = strip_rust_comments(&body);
        if code.contains("pub struct WorkUnitInfo") || code.contains("struct WorkUnitInfo {") {
            local_definitions.push(path.display().to_string());
        }
        if code.contains("pub use codelet_rpc_types::WorkUnitInfo")
            || code.contains("pub use ::codelet_rpc_types::WorkUnitInfo")
        {
            has_re_export = true;
        }
    }

    // @step Then codelet/napi contains no struct definition named WorkUnitInfo and instead re-exports the type from codelet/rpc-types and the workspace builds successfully
    assert!(
        local_definitions.is_empty(),
        "codelet/napi must not redefine WorkUnitInfo. Found local definition(s) in: \
         {local_definitions:?}. Replace with `pub use codelet_rpc_types::WorkUnitInfo;`"
    );
    assert!(
        has_re_export,
        "codelet/napi must re-export WorkUnitInfo from codelet-rpc-types via \
         `pub use codelet_rpc_types::WorkUnitInfo;`"
    );
    // The "workspace builds successfully" half of the Then is enforced by the
    // mere fact that this test compiles and runs — the test crate depends on
    // codelet-rpc-types and codelet-rpc-embedded, both of which transitively
    // require the workspace to type-check.
}

#[test]
fn scenario_9_embedded_transport_uses_only_tarpc_transport_channel() {
    // @step Given codelet/rpc-embedded must perform no network serialization for in-process RPCs
    let src_dir = workspace_root().join("rpc-embedded").join("src");
    let rs_files = collect_rs_files(&src_dir);

    // @step When I search the codelet/rpc-embedded source for transport constructors
    let mut uses_channel = false;
    let mut forbidden_uses: Vec<String> = Vec::new();
    for path in &rs_files {
        let body = read_to_string_or_panic(path);
        let code = strip_rust_comments(&body);
        if code.contains("tarpc::transport::channel::unbounded") {
            uses_channel = true;
        }
        // Anything that would imply network serialization in the embedded crate.
        for needle in [
            "tokio_tungstenite",
            "tokio::net::TcpListener",
            "tokio::net::TcpStream",
            "TcpListener::bind",
            "bincode::serialize",
            "bincode::deserialize",
        ] {
            if code.contains(needle) {
                forbidden_uses.push(format!("{}: {}", path.display(), needle));
            }
        }
    }

    // @step Then the only transport constructor used is tarpc::transport::channel::unbounded and no WebSocket, TCP, or bincode serialization call sites appear in the crate source
    assert!(
        uses_channel,
        "codelet/rpc-embedded must use tarpc::transport::channel::unbounded as its in-process transport",
    );
    assert!(
        forbidden_uses.is_empty(),
        "codelet/rpc-embedded must not perform network serialization. \
         Found forbidden call sites: {forbidden_uses:?}"
    );
}

#[test]
fn scenario_10_codelet_rpc_has_no_codelet_core_or_work_units_watcher_dependency() {
    // @step Given the RPC-005 spike must read from a test-only in-memory fixture and not from the real codelet/core or codelet/napi work-units watcher
    let cargo_path = workspace_root().join("rpc").join("Cargo.toml");
    let cargo = read_to_string_or_panic(&cargo_path);

    // @step When I inspect codelet/rpc/Cargo.toml dependencies and the codelet/rpc/src/ source for codelet-core or work_units_watcher imports
    // RPC-006 widening (architecture rule [10]): codelet-core IS now a
    // legitimate dep on codelet/rpc — the watcher lift moved
    // `WorkUnitsWatcher` into codelet/core and SharedFspecService now
    // reads from it. The widened invariant is asserted by
    // `tests/rpc_006_source_shape.rs::
    //  scenario_codelet_rpc_may_depend_on_codelet_core_but_not_on_codelet_napi`.
    // The RPC-005 scenario is retained here in its narrowed-to-napi
    // form so the historical regression catches accidental NAPI deps.
    let mut cargo_violations: Vec<String> = Vec::new();
    for needle in ["codelet-napi"] {
        if cargo.contains(needle) {
            cargo_violations.push(format!("{}: {}", cargo_path.display(), needle));
        }
    }

    let src_dir = workspace_root().join("rpc").join("src");
    let rs_files = collect_rs_files(&src_dir);
    let mut import_violations: Vec<String> = Vec::new();
    for path in &rs_files {
        let body = read_to_string_or_panic(path);
        let code = strip_rust_comments(&body);
        for needle in ["use codelet_napi", "work_units_watcher"] {
            if code.contains(needle) {
                import_violations.push(format!("{}: {}", path.display(), needle));
            }
        }
    }

    // @step Then codelet/rpc declares no codelet-core or codelet-napi dependency and the codelet/rpc source contains no use codelet_core or use codelet_napi imports
    // After RPC-006 the assertion narrows: codelet-napi is still
    // forbidden (the RPC-005 invariant), codelet-core is permitted
    // (RPC-006 widening).
    assert!(
        cargo_violations.is_empty(),
        "codelet/rpc/Cargo.toml must not depend on codelet-napi (RPC-005 rule [10] preserved post-RPC-006). \
         Found: {cargo_violations:?}"
    );
    assert!(
        import_violations.is_empty(),
        "codelet/rpc/src must not import codelet_napi or the legacy work_units_watcher \
         (RPC-005 rule [10] preserved post-RPC-006). Found: {import_violations:?}"
    );
}

#[test]
fn scenario_11_rpc_server_binary_binds_only_to_127_0_0_1() {
    // @step Given the RPC-005 rpc-server is restricted to TCP loopback in this card
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
        "rpc-server main.rs must bind to 127.0.0.1 (RPC-005 rule [13]). Source: {}",
        main_path.display()
    );
    assert!(
        forbidden_addrs.is_empty(),
        "rpc-server main.rs must not bind to non-loopback addresses (RPC-005 rule [13]). \
         Found: {forbidden_addrs:?}"
    );
}
