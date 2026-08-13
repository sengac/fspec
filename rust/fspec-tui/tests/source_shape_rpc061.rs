//! RPC-061 — Source-shape assertions for the supervisor / subordinate
//! links surface.
//!
//! Feature: spec/features/rpc061-source-shape.feature
//!
//! These tests scan source files at compile time to pin the layering
//! contract for the new wire type, trait additions, stub state, RPC
//! service additions, backend forwarders, dispatch helper module, and
//! UI plumbing. Mirrors the source_shape_rpc060 pattern.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::PathBuf;

fn workspace_root() -> PathBuf {
    let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    crate_dir
        .parent()
        .and_then(|p| p.parent())
        .expect("workspace root above rust/fspec-tui")
        .to_path_buf()
}

fn read_source(path: &PathBuf) -> String {
    fs::read_to_string(path).unwrap_or_else(|_| panic!("read {}", path.display()))
}

/// Scenario: codelet-rpc-types exposes IncomingMessageInput
#[test]
fn rpc_types_declares_incoming_message_input() {
    // @step Given the crate codelet-rpc-types is compiled
    let path = workspace_root().join("rust/rpc-types/src/lib.rs");
    let source = read_source(&path);

    // @step Then it declares a public struct named "IncomingMessageInput"
    assert!(
        source.contains("pub struct IncomingMessageInput"),
        "rpc-types/src/lib.rs should declare pub struct IncomingMessageInput"
    );
    // @step And the struct has field "source_session_id" of type String
    assert!(
        source.contains("source_session_id: String"),
        "IncomingMessageInput should declare source_session_id: String"
    );
    // @step And the struct has field "role_name" of type String
    assert!(
        source.contains("role_name: String"),
        "IncomingMessageInput should declare role_name: String"
    );
    // @step And the struct has field "message" of type String
    assert!(
        source.contains("pub message: String"),
        "IncomingMessageInput should declare pub message: String"
    );
    // @step And the struct has field "images" of type Option<Vec<IncomingMessageImage>>
    assert!(
        source.contains("images: Option<Vec<IncomingMessageImage>>"),
        "IncomingMessageInput should declare images: Option<Vec<IncomingMessageImage>>"
    );
    // @step And the struct derives Debug, Clone, PartialEq, Eq, Serialize, Deserialize
    assert!(
        source.contains("Debug, Clone, PartialEq, Eq, Serialize, Deserialize")
            || source.contains("Debug, Clone, Serialize, Deserialize, PartialEq, Eq")
            || (source.contains("Debug")
                && source.contains("Clone")
                && source.contains("PartialEq")
                && source.contains("Eq")
                && source.contains("Serialize")
                && source.contains("Deserialize")),
        "IncomingMessageInput should derive Debug, Clone, PartialEq, Eq, Serialize, Deserialize"
    );
}

/// Scenario: SessionManagerHandle trait declares all five RPC-061 methods
#[test]
fn session_manager_handle_declares_supervisor_methods() {
    // @step Given the trait file rust/core/src/session_manager_handle.rs is compiled
    let path = workspace_root().join("rust/core/src/session_manager_handle.rs");
    let source = read_source(&path);

    // @step Then it declares fn add_supervisor with the documented signature
    assert!(
        source.contains("fn add_supervisor("),
        "SessionManagerHandle should declare fn add_supervisor"
    );
    // @step And it declares fn remove_supervisor
    assert!(
        source.contains("fn remove_supervisor("),
        "SessionManagerHandle should declare fn remove_supervisor"
    );
    // @step And it declares fn get_subordinate
    assert!(
        source.contains("fn get_subordinate("),
        "SessionManagerHandle should declare fn get_subordinate"
    );
    // @step And it declares fn get_subordinates
    assert!(
        source.contains("fn get_subordinates("),
        "SessionManagerHandle should declare fn get_subordinates"
    );
    // @step And it declares fn receive_incoming_message
    assert!(
        source.contains("fn receive_incoming_message("),
        "SessionManagerHandle should declare fn receive_incoming_message"
    );
}

/// Scenario: FspecService trait declares the five RPC-061 methods
#[test]
fn fspec_service_declares_supervisor_methods() {
    // @step Given the crate codelet-rpc is compiled
    let path = workspace_root().join("rust/rpc/src/lib.rs");
    let source = read_source(&path);

    // @step Then the FspecService trait declares async fn add_supervisor
    assert!(
        source.contains("async fn add_supervisor("),
        "FspecService should declare async fn add_supervisor"
    );
    // @step And it declares async fn remove_supervisor
    assert!(
        source.contains("async fn remove_supervisor("),
        "FspecService should declare async fn remove_supervisor"
    );
    // @step And it declares async fn get_subordinate
    assert!(
        source.contains("async fn get_subordinate("),
        "FspecService should declare async fn get_subordinate"
    );
    // @step And it declares async fn get_subordinates
    assert!(
        source.contains("async fn get_subordinates("),
        "FspecService should declare async fn get_subordinates"
    );
    // @step And it declares async fn receive_incoming_message
    assert!(
        source.contains("async fn receive_incoming_message("),
        "FspecService should declare async fn receive_incoming_message"
    );
}

/// Scenario: components/mod.rs Action enum gains the two RPC-061 variants
#[test]
fn action_enum_gains_rpc061_variants() {
    // @step Given the file rust/fspec-tui/src/components/mod.rs is compiled
    let path = workspace_root().join("rust/fspec-tui/src/components/mod.rs");
    let source = read_source(&path);

    // @step Then it declares Action::SupervisorsLoaded
    assert!(
        source.contains("SupervisorsLoaded"),
        "Action enum should declare SupervisorsLoaded"
    );
    // @step And it declares Action::SendToSubordinate
    assert!(
        source.contains("SendToSubordinate"),
        "Action enum should declare SendToSubordinate"
    );
}

/// Scenario: handle_impl.rs in codelet-sessions wires the supervisor methods
#[test]
fn sessions_handle_impl_wires_supervisor_methods() {
    // @step Given the file rust/sessions/src/handle_impl.rs is compiled
    let path = workspace_root().join("rust/sessions/src/handle_impl.rs");
    let source = read_source(&path);

    // @step Then it impls fn add_supervisor
    assert!(
        source.contains("fn add_supervisor("),
        "handle_impl.rs should impl fn add_supervisor"
    );
    // @step And it impls fn remove_supervisor
    assert!(
        source.contains("fn remove_supervisor("),
        "handle_impl.rs should impl fn remove_supervisor"
    );
    // @step And it impls fn get_subordinate
    assert!(
        source.contains("fn get_subordinate("),
        "handle_impl.rs should impl fn get_subordinate"
    );
    // @step And it impls fn get_subordinates
    assert!(
        source.contains("fn get_subordinates("),
        "handle_impl.rs should impl fn get_subordinates"
    );
    // @step And it impls fn receive_incoming_message
    assert!(
        source.contains("fn receive_incoming_message("),
        "handle_impl.rs should impl fn receive_incoming_message"
    );
}

/// Scenario: FspecBackend trait gains the five RPC-061 forwarders
#[test]
fn fspec_backend_trait_gains_supervisor_forwarders() {
    // @step Given the file rust/fspec-tui/src/transport/mod.rs is compiled
    let path = workspace_root().join("rust/fspec-tui/src/transport/mod.rs");
    let source = read_source(&path);

    // @step Then it declares async fn add_supervisor
    assert!(
        source.contains("async fn add_supervisor("),
        "FspecBackend should declare async fn add_supervisor"
    );
    // @step And it declares async fn remove_supervisor
    assert!(
        source.contains("async fn remove_supervisor("),
        "FspecBackend should declare async fn remove_supervisor"
    );
    // @step And it declares async fn get_subordinate
    assert!(
        source.contains("async fn get_subordinate("),
        "FspecBackend should declare async fn get_subordinate"
    );
    // @step And it declares async fn get_subordinates
    assert!(
        source.contains("async fn get_subordinates("),
        "FspecBackend should declare async fn get_subordinates"
    );
    // @step And it declares async fn receive_incoming_message
    assert!(
        source.contains("async fn receive_incoming_message("),
        "FspecBackend should declare async fn receive_incoming_message"
    );
}

/// Scenario: SessionHeader gains a subordinate_label field
#[test]
fn session_header_gains_subordinate_label_field() {
    // @step Given the file rust/fspec-tui/src/views/agent/header.rs is compiled
    let path = workspace_root().join("rust/fspec-tui/src/views/agent/header.rs");
    let source = read_source(&path);

    // @step Then SessionHeader declares subordinate_label
    assert!(
        source.contains("subordinate_label"),
        "SessionHeader should declare subordinate_label field"
    );
}

/// Scenario: SessionFooter gains a supervisor_pending_count field
#[test]
fn session_footer_gains_supervisor_pending_count_field() {
    // @step Given the file rust/fspec-tui/src/views/agent/footer.rs is compiled
    let path = workspace_root().join("rust/fspec-tui/src/views/agent/footer.rs");
    let source = read_source(&path);

    // @step Then SessionFooter declares supervisor_pending_count
    assert!(
        source.contains("supervisor_pending_count"),
        "SessionFooter should declare supervisor_pending_count field"
    );
}
