//! Feature: spec/features/rpc-architecture-invariants.feature
use super::*;

#[test]
fn scenario_10_codelet_rpc_has_no_codelet_core_or_work_units_watcher_dependency() {
    // @step Given the RPC-005 spike must read from a test-only in-memory fixture and not from the real rust/core or rust/napi work-units watcher
    let cargo_path = workspace_root().join("rpc").join("Cargo.toml");
    let cargo = read_to_string_or_panic(&cargo_path);

    // @step When I inspect rust/rpc/Cargo.toml dependencies and the rust/rpc/src/ source for codelet-core or work_units_watcher imports
    // RPC-006 widening (architecture rule [10]): codelet-core IS now a
    // legitimate dep on rust/rpc — the watcher lift moved
    // `WorkUnitsWatcher` into rust/core and SharedFspecService now
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

    // @step Then rust/rpc declares no codelet-core or codelet-napi dependency and the rust/rpc source contains no use codelet_core or use codelet_napi imports
    // After RPC-006 the assertion narrows: codelet-napi is still
    // forbidden (the RPC-005 invariant), codelet-core is permitted
    // (RPC-006 widening).
    assert!(
        cargo_violations.is_empty(),
        "rust/rpc/Cargo.toml must not depend on codelet-napi (RPC-005 rule [10] preserved post-RPC-006). \
         Found: {cargo_violations:?}"
    );
    assert!(
        import_violations.is_empty(),
        "rust/rpc/src must not import codelet_napi or the legacy work_units_watcher \
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

    // @step When I inspect rust/rpc-server/src/main.rs for the bind address literal
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
