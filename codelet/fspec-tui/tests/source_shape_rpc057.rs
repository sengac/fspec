//! RPC-057 — Source-shape assertions for the /merge-worktree RPC surface.
//!
//! Feature: spec/features/rpc057-merge-worktree-source-shape.feature
//!
//! These tests scan source files at compile time to pin the layering
//! contract for the FIVE new RPC methods (merge_session_worktree,
//! discard_session_worktree, prune_orphaned_worktrees,
//! list_session_worktrees, inspect_session_changes), the FIVE new wire
//! types (MergeStrategy, MergeStatus, MergeOutcome, SessionWorktreeInfo,
//! SessionChangesSummary), the `MergeConfirmDialog` compositor dialog,
//! and the `/merge-worktree` slash-command dispatch routing in
//! `dispatch_rpc057.rs`. Mirrors the source_shape_rpc054 /
//! source_shape_rpc055 / source_shape_rpc056 patterns.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::PathBuf;

fn workspace_root() -> PathBuf {
    let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    crate_dir
        .parent()
        .and_then(|p| p.parent())
        .expect("workspace root above codelet/fspec-tui")
        .to_path_buf()
}

fn normalise(source: &str) -> String {
    source
        .replace(['\n', '\r'], " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Scenario: All five merge/worktree wire types are exported from codelet-rpc-types
#[test]
fn rpc_types_exports_merge_worktree_wire_types() {
    // @step Given the file codelet/rpc-types/src/lib.rs is compiled
    let path = workspace_root().join("codelet/rpc-types/src/lib.rs");
    let source = fs::read_to_string(&path).expect("read rpc-types/src/lib.rs");
    let normalised = normalise(&source);

    // @step Then it declares a public enum named "MergeStrategy"
    assert!(
        source.contains("pub enum MergeStrategy"),
        "rpc-types/src/lib.rs should declare pub enum MergeStrategy"
    );

    // @step And it declares a public enum named "MergeStatus"
    assert!(
        source.contains("pub enum MergeStatus"),
        "rpc-types/src/lib.rs should declare pub enum MergeStatus"
    );

    // @step And it declares a public struct named "MergeOutcome"
    assert!(
        source.contains("pub struct MergeOutcome"),
        "rpc-types/src/lib.rs should declare pub struct MergeOutcome"
    );

    // @step And MergeOutcome has fields named status, conflicts, merge_commit
    for field in ["pub status:", "pub conflicts:", "pub merge_commit:"] {
        assert!(
            normalised.contains(field),
            "MergeOutcome should declare field {field:?}"
        );
    }

    // @step And it declares a public struct named "SessionWorktreeInfo"
    assert!(
        source.contains("pub struct SessionWorktreeInfo"),
        "rpc-types/src/lib.rs should declare pub struct SessionWorktreeInfo"
    );

    // @step And SessionWorktreeInfo has fields named session_id, worktree_path, base_commit, head_commit, dirty
    for field in [
        "pub session_id:",
        "pub worktree_path:",
        "pub base_commit:",
        "pub head_commit:",
        "pub dirty:",
    ] {
        assert!(
            normalised.contains(field),
            "SessionWorktreeInfo should declare field {field:?}"
        );
    }

    // @step And it declares a public struct named "SessionChangesSummary"
    assert!(
        source.contains("pub struct SessionChangesSummary"),
        "rpc-types/src/lib.rs should declare pub struct SessionChangesSummary"
    );

    // @step And SessionChangesSummary has fields named files_changed, insertions, deletions, commits
    for field in [
        "pub files_changed:",
        "pub insertions:",
        "pub deletions:",
        "pub commits:",
    ] {
        assert!(
            normalised.contains(field),
            "SessionChangesSummary should declare field {field:?}"
        );
    }
}

/// Scenario: SessionManagerHandle declares the five new methods
#[test]
fn session_manager_handle_declares_merge_worktree_methods() {
    // @step Given the file codelet/core/src/session_manager_handle.rs is compiled
    let path = workspace_root().join("codelet/core/src/session_manager_handle.rs");
    let source = fs::read_to_string(&path).expect("read session_manager_handle.rs");
    let normalised = normalise(&source);

    // @step Then it declares a trait method named "merge_session_worktree" returning Result<MergeOutcome, String>
    assert!(
        source.contains("fn merge_session_worktree("),
        "session_manager_handle.rs should declare fn merge_session_worktree"
    );
    assert!(
        normalised.contains("-> Result<MergeOutcome, String>"),
        "merge_session_worktree should return Result<MergeOutcome, String>"
    );

    // @step And it declares a trait method named "discard_session_worktree" returning Result<(), String>
    assert!(
        source.contains("fn discard_session_worktree("),
        "session_manager_handle.rs should declare fn discard_session_worktree"
    );

    // @step And it declares a trait method named "prune_orphaned_worktrees" returning Result<Vec<String>, String>
    assert!(
        source.contains("fn prune_orphaned_worktrees("),
        "session_manager_handle.rs should declare fn prune_orphaned_worktrees"
    );

    // @step And it declares a trait method named "list_session_worktrees" returning Vec<SessionWorktreeInfo>
    assert!(
        source.contains("fn list_session_worktrees("),
        "session_manager_handle.rs should declare fn list_session_worktrees"
    );
    assert!(
        normalised.contains("-> Vec<SessionWorktreeInfo>"),
        "list_session_worktrees should return Vec<SessionWorktreeInfo>"
    );

    // @step And it declares a trait method named "inspect_session_changes" returning Result<SessionChangesSummary, String>
    assert!(
        source.contains("fn inspect_session_changes("),
        "session_manager_handle.rs should declare fn inspect_session_changes"
    );
    assert!(
        normalised.contains("-> Result<SessionChangesSummary, String>"),
        "inspect_session_changes should return Result<SessionChangesSummary, String>"
    );
}

/// Scenario: StubSessionManagerHandle exposes per-call counters for all five methods
#[test]
fn stub_exposes_per_call_counters() {
    // @step Given the file codelet/core/src/session_manager_handle.rs is compiled
    let path = workspace_root().join("codelet/core/src/session_manager_handle.rs");
    let source = fs::read_to_string(&path).expect("read session_manager_handle.rs");
    let normalised = normalise(&source);

    // @step Then StubSessionManagerHandle declares a method named "merge_session_worktree_calls" returning u64
    // @step And StubSessionManagerHandle declares a method named "discard_session_worktree_calls" returning u64
    // @step And StubSessionManagerHandle declares a method named "prune_orphaned_worktrees_calls" returning u64
    // @step And StubSessionManagerHandle declares a method named "list_session_worktrees_calls" returning u64
    // @step And StubSessionManagerHandle declares a method named "inspect_session_changes_calls" returning u64
    for counter in [
        "merge_session_worktree_calls",
        "discard_session_worktree_calls",
        "prune_orphaned_worktrees_calls",
        "list_session_worktrees_calls",
        "inspect_session_changes_calls",
    ] {
        let needle = format!("pub fn {counter}(");
        assert!(
            source.contains(&needle),
            "StubSessionManagerHandle should declare pub fn {counter}"
        );
        let sig = format!("pub fn {counter}(&self) -> u64");
        assert!(
            normalised.contains(&sig),
            "StubSessionManagerHandle should declare {counter}(&self) -> u64"
        );
    }
}

/// Scenario: FspecService declares the five new RPC methods
#[test]
fn fspec_service_declares_merge_worktree_methods() {
    // @step Given the file codelet/rpc/src/lib.rs is compiled
    let path = workspace_root().join("codelet/rpc/src/lib.rs");
    let source = fs::read_to_string(&path).expect("read rpc/src/lib.rs");
    let normalised = normalise(&source);

    // @step Then it declares an async fn named "merge_session_worktree" with return type Result<MergeOutcome, String>
    // @step And it declares an async fn named "discard_session_worktree" with return type Result<(), String>
    // @step And it declares an async fn named "prune_orphaned_worktrees" with return type Result<Vec<String>, String>
    // @step And it declares an async fn named "list_session_worktrees" with return type Vec<SessionWorktreeInfo>
    // @step And it declares an async fn named "inspect_session_changes" with return type Result<SessionChangesSummary, String>
    for method in [
        "merge_session_worktree",
        "discard_session_worktree",
        "prune_orphaned_worktrees",
        "list_session_worktrees",
        "inspect_session_changes",
    ] {
        let needle = format!("async fn {method}(");
        assert!(
            source.contains(&needle),
            "rpc/src/lib.rs should declare async fn {method}"
        );
    }

    // Spot-check the documented return shapes appear in the service surface.
    assert!(
        normalised.contains("-> Result<MergeOutcome, String>"),
        "merge_session_worktree should return Result<MergeOutcome, String>"
    );
    assert!(
        normalised.contains("-> Vec<SessionWorktreeInfo>"),
        "list_session_worktrees should return Vec<SessionWorktreeInfo>"
    );
    assert!(
        normalised.contains("-> Result<SessionChangesSummary, String>"),
        "inspect_session_changes should return Result<SessionChangesSummary, String>"
    );
}

/// Scenario: FspecBackend declares the five new methods
#[test]
fn fspec_backend_declares_merge_worktree_methods() {
    // @step Given the file codelet/fspec-tui/src/transport/mod.rs is compiled
    let path = workspace_root().join("codelet/fspec-tui/src/transport/mod.rs");
    let source = fs::read_to_string(&path).expect("read transport/mod.rs");
    let normalised = normalise(&source);

    // @step Then it declares an async fn named "merge_session_worktree" on the FspecBackend trait returning Result<MergeOutcome>
    // @step And it declares an async fn named "discard_session_worktree" on the FspecBackend trait returning Result<()>
    // @step And it declares an async fn named "prune_orphaned_worktrees" on the FspecBackend trait returning Result<Vec<String>>
    // @step And it declares an async fn named "list_session_worktrees" on the FspecBackend trait returning Result<Vec<SessionWorktreeInfo>>
    // @step And it declares an async fn named "inspect_session_changes" on the FspecBackend trait returning Result<SessionChangesSummary>
    for method in [
        "merge_session_worktree",
        "discard_session_worktree",
        "prune_orphaned_worktrees",
        "list_session_worktrees",
        "inspect_session_changes",
    ] {
        let needle = format!("async fn {method}(");
        assert!(
            source.contains(&needle),
            "transport/mod.rs should declare async fn {method} on FspecBackend"
        );
    }

    // Each must return Result<...> (anyhow-style for the trait surface).
    assert!(
        normalised.contains("-> Result<MergeOutcome>"),
        "FspecBackend::merge_session_worktree should return Result<MergeOutcome>"
    );
    assert!(
        normalised.contains("-> Result<Vec<SessionWorktreeInfo>>"),
        "FspecBackend::list_session_worktrees should return Result<Vec<SessionWorktreeInfo>>"
    );
    assert!(
        normalised.contains("-> Result<SessionChangesSummary>"),
        "FspecBackend::inspect_session_changes should return Result<SessionChangesSummary>"
    );
}

/// Scenario: Both transports implement the five new methods
#[test]
fn both_transports_implement_merge_worktree_methods() {
    // @step Given the files codelet/fspec-tui/src/transport/embedded.rs and codelet/fspec-tui/src/transport/websocket.rs are compiled
    let embedded = fs::read_to_string(
        workspace_root().join("codelet/fspec-tui/src/transport/embedded.rs"),
    )
    .expect("read transport/embedded.rs");
    let websocket = fs::read_to_string(
        workspace_root().join("codelet/fspec-tui/src/transport/websocket.rs"),
    )
    .expect("read transport/websocket.rs");

    // @step Then each file contains an impl of "merge_session_worktree" that calls the corresponding tarpc client method
    // @step And each file contains an impl of "discard_session_worktree" that calls the corresponding tarpc client method
    // @step And each file contains an impl of "prune_orphaned_worktrees" that calls the corresponding tarpc client method
    // @step And each file contains an impl of "list_session_worktrees" that calls the corresponding tarpc client method
    // @step And each file contains an impl of "inspect_session_changes" that calls the corresponding tarpc client method
    for method in [
        "merge_session_worktree",
        "discard_session_worktree",
        "prune_orphaned_worktrees",
        "list_session_worktrees",
        "inspect_session_changes",
    ] {
        let impl_needle = format!("async fn {method}(");
        let forward_needle = format!(".{method}(");
        assert!(
            embedded.contains(&impl_needle),
            "embedded.rs should impl {method}"
        );
        assert!(
            embedded.contains(&forward_needle),
            "embedded.rs should forward to the tarpc client's {method}"
        );
        assert!(
            websocket.contains(&impl_needle),
            "websocket.rs should impl {method}"
        );
        assert!(
            websocket.contains(&forward_needle),
            "websocket.rs should forward to the tarpc client's {method}"
        );
    }
}

/// Scenario: MergeConfirmDialog module exists with the documented entry points
#[test]
fn merge_confirm_dialog_module_exists() {
    // @step Given the file codelet/fspec-tui/src/views/agent/merge_confirm_dialog.rs exists
    let path = workspace_root()
        .join("codelet/fspec-tui/src/views/agent/merge_confirm_dialog.rs");
    let source = fs::read_to_string(&path).expect("read views/agent/merge_confirm_dialog.rs");

    // @step Then it declares a public struct named "MergeConfirmDialog"
    assert!(
        source.contains("pub struct MergeConfirmDialog"),
        "merge_confirm_dialog.rs should declare pub struct MergeConfirmDialog"
    );

    // @step And it declares an enum named "MergeConfirmDialogOutcome" with variants for Merge, Discard, Cancel
    assert!(
        source.contains("enum MergeConfirmDialogOutcome"),
        "merge_confirm_dialog.rs should declare enum MergeConfirmDialogOutcome"
    );
    for variant in ["Merge", "Discard", "Cancel"] {
        // Variants may be bare (`Merge,`) or struct-form (`Merge {`).
        assert!(
            source.contains(&format!("{variant},"))
                || source.contains(&format!("{variant} {{"))
                || source.contains(&format!("{variant}\n")),
            "MergeConfirmDialogOutcome should declare variant {variant}"
        );
    }

    // @step And MergeConfirmDialog declares a constructor "new" taking a SessionId and a SessionChangesSummary
    assert!(
        source.contains("pub fn new("),
        "MergeConfirmDialog should declare pub fn new"
    );
    assert!(
        source.contains("SessionId") && source.contains("SessionChangesSummary"),
        "MergeConfirmDialog::new should reference SessionId and SessionChangesSummary"
    );

    // @step And MergeConfirmDialog declares a method named "render" taking (&self, Rect, &mut Buffer)
    assert!(
        source.contains("pub fn render("),
        "MergeConfirmDialog should declare pub fn render"
    );

    // @step And MergeConfirmDialog declares a method named "handle_key" taking (&mut self, KeyCode, KeyModifiers) returning MergeConfirmDialogOutcome
    assert!(
        source.contains("pub fn handle_key("),
        "MergeConfirmDialog should declare pub fn handle_key"
    );
}

/// Scenario: /merge-worktree slash command wiring lives in dispatch_rpc057.rs
#[test]
fn dispatch_rpc057_file_has_expected_shape() {
    // @step Given the file codelet/fspec-tui/src/app/dispatch_rpc057.rs exists
    let path = workspace_root().join("codelet/fspec-tui/src/app/dispatch_rpc057.rs");
    let source = fs::read_to_string(&path).expect("read app/dispatch_rpc057.rs");

    // @step Then it declares a method named "handle_slash_merge_worktree"
    // @step And it declares a method named "handle_inspect_changes_loaded"
    // @step And it declares a method named "handle_merge_confirmed"
    // @step And it declares a method named "handle_discard_confirmed"
    // @step And it declares a method named "handle_cancel_merge_dialog"
    // @step And it declares a method named "try_dispatch_rpc057"
    for method in [
        "handle_slash_merge_worktree",
        "handle_inspect_changes_loaded",
        "handle_merge_confirmed",
        "handle_discard_confirmed",
        "handle_cancel_merge_dialog",
        "try_dispatch_rpc057",
    ] {
        let needle = format!("fn {method}(");
        assert!(
            source.contains(&needle),
            "dispatch_rpc057.rs should declare fn {method}"
        );
    }
}

/// Scenario: MergeStrategy and MergeStatus use derive(Default) with default variant attribute (RPC-057 retro 2026-05-27)
///
/// RPC-057 retro followup: the original card authored both enums with
/// manual `impl Default { fn default() -> Self { Self::<Variant> } }`
/// blocks. The codelet workspace's `-D warnings` clippy lint level
/// includes `clippy::derivable_impls`, which flags this pattern as
/// reducible to `#[derive(Default)]` + `#[default]`. The lint regression
/// blocked the codelet-sessions skeleton_invariants suite (the lint test
/// runs `cargo clippy -p codelet-sessions -- -D warnings`). The fix is
/// byte-equivalent (Default::default() returns the same variant) and
/// has no serialisation, no API-surface, no runtime semantics change.
#[test]
fn merge_strategy_and_status_use_derive_default() {
    // @step Given the codelet workspace inherits the lint level `-D warnings` which includes `clippy::derivable_impls`
    // (Verified by codelet/sessions/tests/skeleton_invariants.rs
    // scenario_workspace_lints_are_inherited_and_clippy_passes which
    // shells out to `cargo clippy -p codelet-sessions -- -D warnings`.)

    // @step Given MergeStrategy is declared in codelet/rpc-types/src/lib.rs with FastForward as the conceptual default and MergeStatus is declared with NoChanges as the conceptual default
    let path = workspace_root().join("codelet/rpc-types/src/lib.rs");
    let source = fs::read_to_string(&path).expect("read rpc-types/src/lib.rs");

    // Locate MergeStrategy + MergeStatus enum declarations.
    let merge_strategy_decl_idx = source
        .find("pub enum MergeStrategy")
        .expect("MergeStrategy enum must exist in codelet/rpc-types/src/lib.rs");
    let merge_status_decl_idx = source
        .find("pub enum MergeStatus")
        .expect("MergeStatus enum must exist in codelet/rpc-types/src/lib.rs");

    // Inspect the 600-byte window BEFORE each enum keyword to capture the
    // attribute block (#[derive(...)], #[serde(...)], etc.).
    let prefix_window = |idx: usize| {
        let start = idx.saturating_sub(600);
        source[start..idx].to_string()
    };
    let strategy_prefix = prefix_window(merge_strategy_decl_idx);
    let status_prefix = prefix_window(merge_status_decl_idx);

    // @step When I run `cargo clippy -p codelet-sessions -- -D warnings` against the post-fix worktree
    // (Compile-only structural assertion at this layer — the actual
    // clippy run is exercised by skeleton_invariants's
    // scenario_workspace_lints_are_inherited_and_clippy_passes.)

    // @step Then clippy exits with code 0 and emits no `clippy::derivable_impls` errors against MergeStrategy or MergeStatus
    // (Verified transitively by the structural assertions below; the
    // lint cannot fire when the `#[derive(Default)]` + `#[default]`
    // pattern is in place.)

    // @step Then the MergeStrategy declaration in codelet/rpc-types/src/lib.rs carries `#[derive(Default)]` on the enum and `#[default]` on the FastForward variant, with no remaining manual `impl Default for MergeStrategy` block
    assert!(
        strategy_prefix.contains("Default"),
        "MergeStrategy enum must carry `#[derive(Default)]` (or include Default in its derive list). Got attribute prefix:\n{strategy_prefix}"
    );
    // The closest #[default] attribute that appears between the enum
    // declaration and the next `}` must be on the FastForward variant.
    let strategy_after = &source[merge_strategy_decl_idx..];
    let strategy_body_end = strategy_after
        .find("\n}\n")
        .expect("MergeStrategy enum body must terminate with `}` on its own line");
    let strategy_body = &strategy_after[..strategy_body_end];
    assert!(
        strategy_body.contains("#[default]"),
        "MergeStrategy must mark one of its variants with `#[default]`. Got body:\n{strategy_body}"
    );
    let default_attr_pos = strategy_body.find("#[default]").unwrap();
    let after_default_attr = &strategy_body[default_attr_pos..];
    assert!(
        after_default_attr.contains("FastForward"),
        "MergeStrategy's `#[default]` attribute must precede the FastForward variant. Got body slice:\n{after_default_attr}"
    );
    assert!(
        !source.contains("impl Default for MergeStrategy"),
        "RPC-057 retro: the manual `impl Default for MergeStrategy` block must be removed (derive(Default) + #[default] FastForward replaces it)"
    );

    // @step Then the MergeStatus declaration in codelet/rpc-types/src/lib.rs carries `#[derive(Default)]` on the enum and `#[default]` on the NoChanges variant, with no remaining manual `impl Default for MergeStatus` block
    assert!(
        status_prefix.contains("Default"),
        "MergeStatus enum must carry `#[derive(Default)]` (or include Default in its derive list). Got attribute prefix:\n{status_prefix}"
    );
    let status_after = &source[merge_status_decl_idx..];
    let status_body_end = status_after
        .find("\n}\n")
        .expect("MergeStatus enum body must terminate with `}` on its own line");
    let status_body = &status_after[..status_body_end];
    assert!(
        status_body.contains("#[default]"),
        "MergeStatus must mark one of its variants with `#[default]`. Got body:\n{status_body}"
    );
    let default_attr_pos = status_body.find("#[default]").unwrap();
    let after_default_attr = &status_body[default_attr_pos..];
    assert!(
        after_default_attr.contains("NoChanges"),
        "MergeStatus's `#[default]` attribute must precede the NoChanges variant. Got body slice:\n{after_default_attr}"
    );
    assert!(
        !source.contains("impl Default for MergeStatus"),
        "RPC-057 retro: the manual `impl Default for MergeStatus` block must be removed (derive(Default) + #[default] NoChanges replaces it)"
    );

    // @step Then the Default::default() values are byte-identical to the pre-fix manual impls: MergeStrategy::default() == MergeStrategy::FastForward and MergeStatus::default() == MergeStatus::NoChanges
    //
    // Compile-time witness: we statically reference the variants the
    // Default impls are required to produce. If a future change
    // accidentally points #[default] at a different variant, both
    // assertions above will fail (the #[default] attribute is bound to
    // ONE variant only). The runtime equality check below is the final
    // proof — Default::default() returning Self::FastForward / Self::NoChanges
    // is the entire behavioural contract of the manual impls that were
    // replaced.
    use codelet_rpc_types::{MergeStrategy, MergeStatus};
    assert_eq!(MergeStrategy::default(), MergeStrategy::FastForward);
    assert_eq!(MergeStatus::default(), MergeStatus::NoChanges);
}
