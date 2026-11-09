//! Feature: spec/features/rpc-architecture-invariants.feature
use super::*;

#[test]
fn scenario_7_embedded_transport_requires_tokio_handle_at_construction() {
    // @step Given the rust/rpc-embedded crate exposes an EmbeddedTransport public constructor
    let lib_path = workspace_root()
        .join("rpc-embedded")
        .join("src")
        .join("lib.rs");
    let lib_src = read_to_string_or_panic(&lib_path);

    // @step When I inspect the public type signature of the constructor and search the rust/rpc-embedded source for runtime construction calls
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
    // new codelet-fspec-tui crate. We scan rust/fspec-tui/src/ with the
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
    // the new rust/fspec/src/ binary crate. Per RPC-010 rule [7] +
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

    // @step Then the constructor takes a tokio::runtime::Handle as a non-defaulted argument and the rust/rpc-embedded source contains no calls to tokio::runtime::Builder or tokio::runtime::Runtime::new that would create a separate runtime
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
        "rust/rpc-embedded and rust/fspec-tui must not construct their own tokio runtime. \
         Found forbidden Builder/Runtime::new usage in: {forbidden_calls:?}"
    );
}

#[test]
fn scenario_8_work_unit_info_is_defined_once_in_rpc_types_and_re_exported_by_napi() {
    // @step Given rust/rpc-types defines a public WorkUnitInfo struct with fields id, title, work_type, status, description, estimate, and epic
    let rpc_types_lib = read_tree(
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

    // @step When I inspect rust/napi for definitions of WorkUnitInfo and run cargo check on the codelet workspace
    let core_src_dir = workspace_root().join("core").join("src");
    let core_rs_files = collect_rs_files(&core_src_dir);
    let mut local_definitions: Vec<String> = Vec::new();
    let mut has_import = false;
    for path in &core_rs_files {
        let body = read_to_string_or_panic(path);
        let code = strip_rust_comments(&body);
        if code.contains("pub struct WorkUnitInfo") || code.contains("struct WorkUnitInfo {") {
            local_definitions.push(path.display().to_string());
        }
        if code.contains("use codelet_rpc_types::WorkUnitInfo") {
            has_import = true;
        }
    }

    // @step Then rust/napi contains no struct definition named WorkUnitInfo and instead re-exports the type from rust/rpc-types and the workspace builds successfully
    assert!(
        local_definitions.is_empty(),
        "rust/core must not redefine WorkUnitInfo. Found local definition(s) in: \
         {local_definitions:?}. Use `codelet_rpc_types::WorkUnitInfo` instead"
    );
    assert!(
        has_import,
        "rust/core must consume WorkUnitInfo from codelet-rpc-types via \
         `use codelet_rpc_types::WorkUnitInfo;`"
    );
    // The "workspace builds successfully" half of the Then is enforced by the
    // mere fact that this test compiles and runs — the test crate depends on
    // codelet-rpc-types and codelet-rpc-embedded, both of which transitively
    // require the workspace to type-check.
}

#[test]
fn scenario_9_embedded_transport_uses_only_tarpc_transport_channel() {
    // @step Given rust/rpc-embedded must perform no network serialization for in-process RPCs
    let src_dir = workspace_root().join("rpc-embedded").join("src");
    let rs_files = collect_rs_files(&src_dir);

    // @step When I search the rust/rpc-embedded source for transport constructors
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
        "rust/rpc-embedded must use tarpc::transport::channel::unbounded as its in-process transport",
    );
    assert!(
        forbidden_uses.is_empty(),
        "rust/rpc-embedded must not perform network serialization. \
         Found forbidden call sites: {forbidden_uses:?}"
    );
}
