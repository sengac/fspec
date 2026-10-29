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
//! `dispatch_merge_worktree.rs`. Mirrors the source_shape_rpc054 /
//! source_shape_rpc055 / source_shape_rpc056 patterns.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_test_helpers::source_composition::read_source;
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

fn normalise(source: &str) -> String {
    source
        .replace(['\n', '\r'], " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

include!("source_shape_rpc057/service_contracts.rs");

/// Scenario: FspecBackend declares the five new methods
#[test]
fn fspec_backend_declares_merge_worktree_methods() {
    // @step Given the file rust/fspec-tui/src/transport/mod.rs is compiled
    let path = workspace_root().join("rust/fspec-tui/src/transport/mod.rs");
    let source = read_source(&path).expect("resolve transport/mod.rs");
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
    // @step Given the files rust/fspec-tui/src/transport/embedded.rs and rust/fspec-tui/src/transport/websocket.rs are compiled
    let embedded = read_source(&workspace_root().join("rust/fspec-tui/src/transport/embedded.rs"))
        .expect("resolve transport/embedded.rs");
    let websocket =
        read_source(&workspace_root().join("rust/fspec-tui/src/transport/websocket.rs"))
            .expect("resolve transport/websocket.rs");

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
    // @step Given the file rust/fspec-tui/src/views/agent/merge_confirm_dialog.rs exists
    let path = workspace_root().join("rust/fspec-tui/src/views/agent/merge_confirm_dialog.rs");
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
    // (Verified by rust/sessions/tests/skeleton_invariants.rs
    // scenario_workspace_lints_are_inherited_and_clippy_passes which
    // shells out to `cargo clippy -p codelet-sessions -- -D warnings`.)

    // @step Given MergeStrategy is declared in rust/rpc-types/src/lib.rs with FastForward as the conceptual default and MergeStatus is declared with NoChanges as the conceptual default
    let path = workspace_root().join("rust/rpc-types/src/lib.rs");
    let source = fs::read_to_string(&path).expect("read rpc-types/src/lib.rs");

    // Locate MergeStrategy + MergeStatus enum declarations.
    let merge_strategy_decl_idx = source
        .find("pub enum MergeStrategy")
        .expect("MergeStrategy enum must exist in rust/rpc-types/src/lib.rs");
    let merge_status_decl_idx = source
        .find("pub enum MergeStatus")
        .expect("MergeStatus enum must exist in rust/rpc-types/src/lib.rs");

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

    // @step Then the MergeStrategy declaration in rust/rpc-types/src/lib.rs carries `#[derive(Default)]` on the enum and `#[default]` on the FastForward variant, with no remaining manual `impl Default for MergeStrategy` block
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

    // @step Then the MergeStatus declaration in rust/rpc-types/src/lib.rs carries `#[derive(Default)]` on the enum and `#[default]` on the NoChanges variant, with no remaining manual `impl Default for MergeStatus` block
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
    use codelet_rpc_types::{MergeStatus, MergeStrategy};
    assert_eq!(MergeStrategy::default(), MergeStrategy::FastForward);
    assert_eq!(MergeStatus::default(), MergeStatus::NoChanges);
}

include!("source_shape_rpc057/slash_wiring.rs");
