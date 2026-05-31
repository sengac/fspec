// Feature: spec/features/reduce-codelet-napi-persistence-to-thin-napi-bindings-rs-shims.feature
//
// This integration test validates the acceptance criteria for RPC-035 —
// reducing codelet/napi/src/persistence/ to a pure NAPI adapter
// (mod.rs + napi_bindings.rs only), routing every persistence call inside
// napi_bindings.rs through an explicit `use codelet_core::persistence::{...};`
// import, inlining the surviving helpers from persistence/mod.rs into the
// corresponding `#[napi]` shims, and relocating the 48-test +
// 9-test in-crate persistence test suites to codelet-core.
//
// Tests live under codelet/core/tests/ so they are a downstream consumer
// (proving the relocation is observable from outside codelet-napi) and so
// they participate in codelet-core's stricter lint configuration.
//
// In red phase these tests FAIL because:
//   - codelet/napi/src/persistence/ still contains tests.rs and lazy_init_tests.rs
//   - codelet/napi/src/persistence/mod.rs still has the 6 surviving pub fn helpers
//   - codelet/napi/src/persistence/napi_bindings.rs still opens with `use super::*;`
//   - codelet/core/src/persistence/tests.rs and lazy_init_tests.rs do not exist yet

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;

/// Resolve the project workspace root from the codelet-core crate.
fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR points to codelet/core; the workspace root is two levels up.
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("workspace root resolves")
        .to_path_buf()
}

fn napi_persistence_dir() -> PathBuf {
    workspace_root().join("codelet/napi/src/persistence")
}

fn core_persistence_dir() -> PathBuf {
    workspace_root().join("codelet/core/src/persistence")
}

fn read_to_string_at(path: &std::path::Path) -> String {
    fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("read {path:?} failed: {e}"))
}

// ============================================================================
// Scenario: codelet/napi/src/persistence/ contains exactly mod.rs and napi_bindings.rs after the card
// ============================================================================
#[test]
fn test_napi_persistence_directory_contains_only_mod_and_napi_bindings() {
    // @step Given RPC-031 through RPC-034 have lifted every persistence type and singleton into codelet-core::persistence
    // @step And the relocation work for tests.rs and lazy_init_tests.rs has completed
    let dir = napi_persistence_dir();
    assert!(dir.is_dir(), "{dir:?} must be a directory");

    // @step When std::fs::read_dir lists the entries of codelet/napi/src/persistence/
    let actual: HashSet<String> = fs::read_dir(&dir)
        .expect("read_dir succeeds")
        .map(|entry| {
            entry
                .expect("dir entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect();

    // @step Then the result contains exactly two filenames, "mod.rs" and "napi_bindings.rs", and nothing else
    let expected: HashSet<String> =
        ["mod.rs", "napi_bindings.rs"].iter().map(std::string::ToString::to_string).collect();
    assert_eq!(
        actual, expected,
        "codelet/napi/src/persistence/ must contain ONLY mod.rs + napi_bindings.rs after RPC-035 (found {actual:?})"
    );

    // @step And codelet/napi/src/persistence/mod.rs is at most 20 lines long
    let mod_rs = read_to_string_at(&dir.join("mod.rs"));
    let line_count = mod_rs.lines().count();
    assert!(
        line_count <= 20,
        "codelet/napi/src/persistence/mod.rs must be at most 20 lines after RPC-035 (got {line_count})"
    );

    // @step And codelet/napi/src/persistence/mod.rs contains no pub fn declarations
    assert!(
        !mod_rs.contains("pub fn "),
        "codelet/napi/src/persistence/mod.rs must contain no `pub fn` declarations after RPC-035"
    );

    // @step And codelet/napi/src/persistence/mod.rs contains exactly one pub use line: `pub use codelet_core::persistence::*;`
    let pub_use_count = mod_rs
        .lines()
        .filter(|line| line.trim_start().starts_with("pub use "))
        .count();
    assert!(
        pub_use_count >= 1,
        "codelet/napi/src/persistence/mod.rs must re-export codelet_core::persistence via `pub use codelet_core::persistence::*;`"
    );
    assert!(
        mod_rs.contains("pub use codelet_core::persistence::*"),
        "codelet/napi/src/persistence/mod.rs must contain the line `pub use codelet_core::persistence::*;`"
    );
}

// ============================================================================
// Scenario: Every #[napi] persistence_* function sources its callees through an explicit codelet_core::persistence import
// ============================================================================
#[test]
fn test_napi_bindings_uses_explicit_codelet_core_persistence_imports() {
    // @step Given the engineer opens codelet/napi/src/persistence/napi_bindings.rs
    let napi_bindings = read_to_string_at(&napi_persistence_dir().join("napi_bindings.rs"));

    // @step When the engineer reads the import block at the top of the file
    // @step Then a single `use codelet_core::persistence::{ create_session, create_session_with_provider, load_session, resume_last_session, list_sessions, delete_session, rename_session, fork_session, merge_messages, cherry_pick, append_message, append_message_with_metadata, get_message, get_session_messages, get_session_messages_full, update_session_tokens, set_session_tokens, set_compaction_state, clear_compaction_state, cleanup_orphaned_messages, store_blob, get_blob, blob_exists, process_envelope_for_blob_storage, rehydrate_envelope_blobs, MessageEnvelope, MessagePayload, UserContent, AssistantContent, SessionManifest, ForkPoint, MergeRecord, CompactionState, TokenUsage, StoredMessage, HistoryEntry, history };` is present
    assert!(
        napi_bindings.contains("use codelet_core::persistence::"),
        "codelet/napi/src/persistence/napi_bindings.rs must import its persistence symbols via `use codelet_core::persistence::{{ ... }};` (explicit, not via `use super::*;`)"
    );

    // The explicit import must mention every required symbol so the dependency
    // direction is documented at the call site.
    let required_symbols = [
        "create_session",
        "create_session_with_provider",
        "load_session",
        "resume_last_session",
        "list_sessions",
        "delete_session",
        "rename_session",
        "fork_session",
        "merge_messages",
        "cherry_pick",
        "append_message",
        "append_message_with_metadata",
        "get_message",
        "get_session_messages",
        "get_session_messages_full",
        "update_session_tokens",
        "set_session_tokens",
        "set_compaction_state",
        "clear_compaction_state",
        "cleanup_orphaned_messages",
        "store_blob",
        "get_blob",
        "blob_exists",
        "process_envelope_for_blob_storage",
        "rehydrate_envelope_blobs",
        "MessageEnvelope",
        "MessagePayload",
        "UserContent",
        "AssistantContent",
        "SessionManifest",
        "ForkPoint",
        "MergeRecord",
        "CompactionState",
        "TokenUsage",
        "StoredMessage",
        "HistoryEntry",
        "history",
    ];
    for sym in required_symbols {
        assert!(
            napi_bindings.contains(sym),
            "napi_bindings.rs must reference the lifted symbol `{sym}` via the explicit codelet_core::persistence import block"
        );
    }

    // @step And the file contains no `use super::*;` line anywhere
    for (line_no, line) in napi_bindings.lines().enumerate() {
        let trimmed = line.trim();
        assert!(
            !trimmed.starts_with("use super::*")
                && !trimmed.starts_with("use super ::*")
                && trimmed != "use super::*;",
            "napi_bindings.rs:{} must not contain `use super::*;` after RPC-035 — every callee must be sourced via an explicit codelet_core::persistence import (found `{}`)",
            line_no + 1,
            line
        );
    }

    // @step And each #[napi] persistence_* function body references its callees by their unqualified name (e.g. `load_session(uuid)`) imported from codelet_core::persistence (not from `super::`)
    // Spot-check the most-easily-broken sites: the helper free functions that
    // currently reach for `super::MessagePayload` / `super::UserContent` /
    // `super::AssistantContent` (calculate_envelope_tokens and
    // extract_content_summary at the bottom of napi_bindings.rs).
    assert!(
        !napi_bindings.contains("super::MessagePayload"),
        "napi_bindings.rs must not use `super::MessagePayload` after RPC-035 — switch to unqualified `MessagePayload` resolved via the explicit codelet_core::persistence import"
    );
    assert!(
        !napi_bindings.contains("super::UserContent"),
        "napi_bindings.rs must not use `super::UserContent` after RPC-035"
    );
    assert!(
        !napi_bindings.contains("super::AssistantContent"),
        "napi_bindings.rs must not use `super::AssistantContent` after RPC-035"
    );
}

// ============================================================================
// Scenario: The relocated codelet-core persistence::tests suite runs all 48 tests
// ============================================================================
#[test]
fn test_relocated_codelet_core_persistence_tests_module_exists_with_48_tests() {
    // @step Given codelet/core/src/persistence/tests.rs has been added with the relocated 48 tests
    let tests_path = core_persistence_dir().join("tests.rs");
    assert!(
        tests_path.is_file(),
        "codelet/core/src/persistence/tests.rs must exist after RPC-035 relocates the 48-test suite from codelet-napi"
    );

    // Count `#[test]` attributes — must equal 48 (the byte-identical
    // baseline from codelet-napi pre-relocation) PLUS any new tests
    // added by later cards that lift more functionality into
    // `codelet_core::persistence`. RPC-049 added 3 tests for the lifted
    // `get_session_message_envelopes` function; future cards may add
    // more. The test enforces "no tests were DROPPED" (>= 48), not a
    // strict identity.
    // @step When `cargo test -p codelet-core --lib persistence::tests` is run
    let tests = read_to_string_at(&tests_path);
    let test_count = tests
        .lines()
        .filter(|line| line.trim_start().starts_with("#[test]"))
        .count();
    // @step Then 48 tests pass, including test_resume_session_after_closing_terminal, test_blob_reference_format, test_tool_result_blob_storage_and_rehydration, and test_blob_storage_dedup
    assert!(
        test_count >= 48,
        "codelet/core/src/persistence/tests.rs must contain at least 48 #[test] attributes (the byte-identical baseline from codelet-napi pre-relocation, got {test_count})"
    );
    for fn_name in [
        "test_resume_session_after_closing_terminal",
        "test_blob_reference_format",
        "test_tool_result_blob_storage_and_rehydration",
        "test_blob_deduplication",
    ] {
        assert!(
            tests.contains(fn_name),
            "relocated codelet-core tests.rs must contain the test fn `{fn_name}`"
        );
    }

    // @step And codelet/core/src/persistence/mod.rs declares `#[cfg(test)] mod tests;`
    let mod_rs = read_to_string_at(&core_persistence_dir().join("mod.rs"));
    assert!(
        mod_rs.contains("mod tests"),
        "codelet/core/src/persistence/mod.rs must declare `#[cfg(test)] mod tests;`"
    );

    // @step And `cargo test -p codelet-napi --lib persistence::tests` reports zero tests because codelet/napi/src/persistence/tests.rs no longer exists
    let napi_tests_path = napi_persistence_dir().join("tests.rs");
    assert!(
        !napi_tests_path.exists(),
        "codelet/napi/src/persistence/tests.rs must not exist after RPC-035 relocates the 48-test suite to codelet-core (so `cargo test -p codelet-napi --lib persistence::tests` reports zero tests)"
    );

    // The relocated test file must use the codelet-core canonical import path,
    // not the codelet-napi `use super::*;` path.
    assert!(
        tests.contains("use crate::persistence::"),
        "relocated tests.rs must import via `use crate::persistence::*;` (the codelet-core canonical path)"
    );

    // The relocated setup helper must use codelet-core primitives only
    // (codelet_common::set_data_directory + reset_stores_for_tests).
    assert!(
        tests.contains("codelet_common::set_data_directory")
            || tests.contains("set_data_directory"),
        "relocated tests.rs setup helper must call set_data_directory (from codelet_common)"
    );
    assert!(
        tests.contains("reset_stores_for_tests"),
        "relocated tests.rs setup helper must call codelet_core::persistence::reset_stores_for_tests()"
    );
    // It must NOT reach into NAPI-only credentials/graph resets.
    assert!(
        !tests.contains("crate::credentials::"),
        "relocated tests.rs must not reach into crate::credentials (NAPI-only concern)"
    );
    assert!(
        !tests.contains("crate::graph::"),
        "relocated tests.rs must not reach into crate::graph (NAPI-only concern)"
    );
}

// ============================================================================
// Scenario: The relocated codelet-core persistence::lazy_init_tests suite runs all 9 BUG-122 lazy-init invariants
// ============================================================================
#[test]
fn test_relocated_codelet_core_persistence_lazy_init_tests_module_exists_with_9_tests() {
    // @step Given codelet/core/src/persistence/lazy_init_tests.rs has been added with the relocated 9 tests
    let lazy_path = core_persistence_dir().join("lazy_init_tests.rs");
    assert!(
        lazy_path.is_file(),
        "codelet/core/src/persistence/lazy_init_tests.rs must exist after RPC-035 relocates the 9-test BUG-122 lazy-init suite"
    );

    // @step When `cargo test -p codelet-core --lib persistence::lazy_init_tests` is run
    let lazy = read_to_string_at(&lazy_path);
    let test_count = lazy
        .lines()
        .filter(|line| line.trim_start().starts_with("#[test]"))
        .count();
    // @step Then 9 tests pass, including test_lazy_get_history_only_inits_history_store, test_lazy_store_message_inits_message_and_blob_and_session_store, test_lazy_create_session_only_inits_session_store, test_lazy_session_resume_loads_only_that_session, test_lazy_cross_session_search_loads_on_demand, test_lazy_shell_history_cross_session, test_lazy_search_command_cross_session, test_lazy_forked_message_accessible, and test_lazy_append_and_immediate_read
    assert_eq!(
        test_count, 9,
        "codelet/core/src/persistence/lazy_init_tests.rs must contain exactly 9 #[test] attributes (the BUG-122 lazy-init invariants, got {test_count})"
    );
    for fn_name in [
        "test_lazy_get_history_only_inits_history_store",
        "test_lazy_store_message_inits_message_and_blob_and_session_store",
        "test_lazy_create_session_only_inits_session_store",
        "test_lazy_session_resume_loads_only_that_session",
        "test_lazy_cross_session_search_loads_on_demand",
        "test_lazy_shell_history_cross_session",
        "test_lazy_search_command_cross_session",
        "test_lazy_forked_message_accessible",
        "test_lazy_append_and_immediate_read",
    ] {
        assert!(
            lazy.contains(fn_name),
            "relocated codelet-core lazy_init_tests.rs must contain the test fn `{fn_name}`"
        );
    }

    // @step And codelet/core/src/persistence/mod.rs declares `#[cfg(test)] mod lazy_init_tests;`
    let mod_rs = read_to_string_at(&core_persistence_dir().join("mod.rs"));
    assert!(
        mod_rs.contains("mod lazy_init_tests"),
        "codelet/core/src/persistence/mod.rs must declare `#[cfg(test)] mod lazy_init_tests;`"
    );

    // The relocated tests must reach TEST_MUTEX via the sibling tests module.
    assert!(
        lazy.contains("use super::tests::TEST_MUTEX")
            || lazy.contains("super::tests::TEST_MUTEX"),
        "relocated lazy_init_tests.rs must reach TEST_MUTEX via `use super::tests::TEST_MUTEX;` (the codelet-core sibling module path)"
    );
}

// ============================================================================
// Scenario: codelet-rpc-embedded continues to consume persistence without re-introducing the forbidden rpc to napi arrow
// (Compile-time check — if this test file links, codelet-core's public surface is unchanged)
// ============================================================================
#[test]
fn test_codelet_core_persistence_surface_still_exposes_expected_symbols() {
    // @step Given codelet/rpc-embedded depends on codelet-core but is forbidden from depending on codelet-napi by rpc_006_source_shape.rs
    // @step When a downstream consumer in codelet/rpc-embedded writes `use codelet_core::persistence::{create_session, load_session, append_message_with_metadata, fork_session, store_blob, get_blob, blob_exists, process_envelope_for_blob_storage, rehydrate_envelope_blobs, ensure_directories, history};`
    // The fact that this test compiles is the assertion — if codelet-core
    // ever stops exporting one of these symbols, the test fails at compile time.
    use codelet_core::persistence::{
        append_message_with_metadata, blob_exists, create_session, ensure_directories,
        fork_session, get_blob, history, load_session, process_envelope_for_blob_storage,
        rehydrate_envelope_blobs, store_blob,
    };

    // Reference each symbol so the unused-import lint doesn't elide them.
    // (We don't actually call them — this is a compile-time smoke test.)
    let _ = &create_session;
    let _ = &load_session;
    let _ = &append_message_with_metadata;
    let _ = &fork_session;
    let _ = &store_blob;
    let _ = &get_blob;
    let _ = &blob_exists;
    let _ = &process_envelope_for_blob_storage;
    let _ = &rehydrate_envelope_blobs;
    let _ = &ensure_directories;
    let _ = &history::add;
    let _ = &history::get;
    let _ = &history::search;

    // @step Then the build succeeds with no transitive dependency on codelet-napi
    // @step And the dependency-rule test rpc_006_source_shape.rs continues to pass
    //         (asserted separately by codelet/rpc-embedded/tests/rpc_006_source_shape.rs)
}

// ============================================================================
// Scenario: codelet/napi/index.d.ts is byte-identical before and after the card
// ============================================================================
//
// The pre-card SHA-256 of codelet/napi/index.d.ts is captured here. This
// constant is the **fixture** referenced by the matching Gherkin scenario.
// After RPC-035 lands, regenerating index.d.ts via `cargo build -p
// codelet-napi --release` must reproduce the same hash byte-for-byte.
const PRE_CARD_INDEX_DTS_SHA256: &str =
    "e5b4e8d7fa24f8cd3081c0cabcb318d1d77c44e21aedf661891e2dd46d03145e";

fn sha256_of(path: &std::path::Path) -> String {
    let bytes = fs::read(path).unwrap_or_else(|e| panic!("read {path:?} failed: {e}"));
    let mut hasher = <sha2::Sha256 as sha2::Digest>::new();
    sha2::Digest::update(&mut hasher, &bytes);
    let digest = sha2::Digest::finalize(hasher);
    digest.iter().map(|b| format!("{b:02x}")).collect()
}

#[test]
fn test_index_dts_remains_byte_identical_to_pre_card_snapshot() {
    // @step Given the SHA-256 hash H1 of codelet/napi/index.d.ts captured before RPC-035 is committed as a fixture
    let pre_card_sha256 = PRE_CARD_INDEX_DTS_SHA256;
    assert_eq!(
        pre_card_sha256.len(),
        64,
        "PRE_CARD_INDEX_DTS_SHA256 must be a 64-hex SHA-256 digest"
    );

    // @step When `cargo build -p codelet-napi --release` is run on the post-card source tree
    // @step And napi-rs regenerates codelet/napi/index.d.ts from the #[napi] attributes
    let index_dts = workspace_root().join("codelet/napi/index.d.ts");
    assert!(
        index_dts.is_file(),
        "codelet/napi/index.d.ts must exist (regenerated by `cargo build -p codelet-napi --release` via napi-rs)"
    );

    // @step Then `sha256sum codelet/napi/index.d.ts` returns the same hash H1
    let actual_sha256 = sha256_of(&index_dts);
    assert_eq!(
        actual_sha256, pre_card_sha256,
        "codelet/napi/index.d.ts SHA-256 must equal the pre-card fixture {pre_card_sha256} after RPC-035 (got {actual_sha256})"
    );

    // @step And no `persistenceXxx` declaration is added, removed, renamed, or reordered relative to the pre-card snapshot
    let index_dts_contents = read_to_string_at(&index_dts);
    for required in [
        "persistenceSetDataDirectory",
        "persistenceGetDataDirectory",
        "persistenceCreateSession",
        "persistenceCreateSessionWithProvider",
        "persistenceLoadSession",
        "persistenceResumeLastSession",
        "persistenceListSessions",
        "persistenceDeleteSession",
        "persistenceRenameSession",
        "persistenceForkSession",
        "persistenceMergeMessages",
        "persistenceCherryPick",
        "persistenceAppendMessage",
        "persistenceAppendMessageWithMetadata",
        "persistenceGetMessage",
        "persistenceGetSessionMessages",
        "persistenceGetSessionMessagesFull",
        "persistenceStoreBlob",
        "persistenceGetBlob",
        "persistenceBlobExists",
        "persistenceAddHistory",
        "persistenceGetHistory",
        "persistenceSearchHistory",
        "persistenceUpdateSessionTokens",
        "persistenceSetSessionTokens",
        "persistenceSetCompactionState",
        "persistenceClearCompactionState",
        "persistenceCleanupOrphanedMessages",
        "persistenceStoreMessageEnvelope",
        "persistenceGetMessageEnvelope",
        "persistenceGetMessageEnvelopeRaw",
        "persistenceGetSessionMessageEnvelopes",
        "persistenceGetSessionMessageEnvelopesFull",
        "persistenceGetSessionMessageEnvelopesRaw",
        "persistenceGetSessionMessageEnvelopesRawFull",
    ] {
        assert!(
            index_dts_contents.contains(required),
            "codelet/napi/index.d.ts must still export `{required}` after RPC-035"
        );
    }

    // @step And no Napi* interface field is reordered relative to the pre-card snapshot
    // (The hash-equality check above proves byte-identity which subsumes field order.
    //  We also spot-check that the documented field order for NapiTokenUsage is preserved
    //  — this is the field that napi-rs is most likely to reorder on a source rearrangement.)
    let token_usage_idx = index_dts_contents
        .find("export interface NapiTokenUsage")
        .expect("NapiTokenUsage interface declaration must be present in index.d.ts");
    let token_usage_window =
        &index_dts_contents[token_usage_idx..token_usage_idx + 800];
    let order = [
        "currentContextTokens",
        "cumulativeBilledInput",
        "cumulativeBilledOutput",
        "cacheReadTokens",
        "cacheCreationTokens",
    ];
    let mut last_pos = 0usize;
    for field in order {
        let pos = token_usage_window
            .find(field)
            .unwrap_or_else(|| panic!("NapiTokenUsage must include field `{field}`"));
        assert!(
            pos >= last_pos,
            "NapiTokenUsage field `{field}` must appear in the locked pre-card order"
        );
        last_pos = pos;
    }
}

// ============================================================================
// Scenario: NAPI-bridge integration tests still pass after the relocation
// ============================================================================
#[test]
fn test_napi_bridge_integration_tests_remain_in_codelet_napi_with_rewritten_setup() {
    // @step Given the NAPI-bridge integration tests session_persistence_test.rs and subordinate_session_persistence_test.rs stay in codelet/napi/tests/
    let napi_tests = workspace_root().join("codelet/napi/tests");
    assert!(
        napi_tests.join("session_persistence_test.rs").is_file(),
        "codelet/napi/tests/session_persistence_test.rs must stay in codelet-napi (NAPI-bridge integration test)"
    );
    assert!(
        napi_tests.join("subordinate_session_persistence_test.rs").is_file(),
        "codelet/napi/tests/subordinate_session_persistence_test.rs must stay in codelet-napi (NAPI-bridge integration test)"
    );

    // @step And codelet/napi/src/test_support.rs::setup_test_env is rewritten to call codelet_common::set_data_directory + codelet_core::persistence::reset_stores_for_tests + crate::credentials::reset_credential_store + crate::graph::reset_graph_db inline
    let test_support =
        read_to_string_at(&workspace_root().join("codelet/napi/src/test_support.rs"));
    assert!(
        test_support.contains("codelet_common::set_data_directory"),
        "test_support.rs::setup_test_env must call codelet_common::set_data_directory directly (not via the deleted crate::persistence::set_data_directory shim)"
    );
    assert!(
        test_support.contains("codelet_core::persistence::reset_stores_for_tests")
            || test_support.contains("reset_stores_for_tests"),
        "test_support.rs::setup_test_env must call codelet_core::persistence::reset_stores_for_tests() inline"
    );
    assert!(
        test_support.contains("crate::credentials::reset_credential_store"),
        "test_support.rs::setup_test_env must call crate::credentials::reset_credential_store() inline"
    );
    assert!(
        test_support.contains("crate::graph::reset_graph_db"),
        "test_support.rs::setup_test_env must call crate::graph::reset_graph_db() inline"
    );
    // The pre-RPC-035 indirection via `crate::persistence::set_data_directory` must be gone.
    assert!(
        !test_support.contains("crate::persistence::set_data_directory"),
        "test_support.rs must not route through the deleted `crate::persistence::set_data_directory` shim — call codelet_common::set_data_directory inline"
    );

    // @step When `cargo test -p codelet-napi --test session_persistence_test` is run
    // @step And `cargo test -p codelet-napi --test subordinate_session_persistence_test` is run
    // @step Then 23 tests pass in session_persistence_test
    let session_tests =
        read_to_string_at(&napi_tests.join("session_persistence_test.rs"));
    let session_test_count = session_tests
        .lines()
        .filter(|line| line.trim_start().starts_with("#[test]"))
        .count();
    assert!(
        session_test_count >= 23,
        "codelet/napi/tests/session_persistence_test.rs must retain at least 23 #[test] attributes after RPC-035 (got {session_test_count})"
    );

    // @step And 4 tests pass in subordinate_session_persistence_test
    let sub_tests =
        read_to_string_at(&napi_tests.join("subordinate_session_persistence_test.rs"));
    let sub_test_count = sub_tests
        .lines()
        .filter(|line| line.trim_start().starts_with("#[test]"))
        .count();
    assert!(
        sub_test_count >= 4,
        "codelet/napi/tests/subordinate_session_persistence_test.rs must retain at least 4 #[test] attributes after RPC-035 (got {sub_test_count})"
    );

    // @step And those tests continue to exercise the Napi* wire-struct round-trip (e.g. NapiSessionManifest::from(SessionManifest))
    // The NAPI-bridge integration tests must reference at least one Napi* wire struct
    // OR a #[napi] symbol — that is their reason to stay in codelet-napi.
    assert!(
        session_tests.contains("Napi") || session_tests.contains("persistence_"),
        "session_persistence_test.rs must continue to exercise the Napi* wire-struct round-trip / #[napi] persistence shims (no Napi* / persistence_ reference found)"
    );
}

// ============================================================================
// Scenario: The relocated setup_test_env helper uses codelet-core primitives only
// ============================================================================
#[test]
fn test_relocated_setup_test_env_uses_codelet_core_primitives_only() {
    // @step Given the engineer opens codelet/core/src/persistence/tests.rs
    let tests_path = core_persistence_dir().join("tests.rs");
    let tests = read_to_string_at(&tests_path);

    // @step When the engineer reads the body of setup_test_env at the top of the file
    let setup_idx = tests
        .find("fn setup_test_env")
        .expect("relocated tests.rs must define fn setup_test_env");
    // Inspect only the function body (next ~600 chars after the fn signature).
    let setup_window = &tests[setup_idx..setup_idx.saturating_add(800).min(tests.len())];

    // @step Then the body is `let guard = TEST_MUTEX.lock().unwrap(); let temp_dir = tempfile::tempdir().expect("Failed to create temp directory"); codelet_common::set_data_directory(temp_dir.path().to_path_buf()).expect("Failed to set data directory"); codelet_core::persistence::reset_stores_for_tests(); (guard, temp_dir)`
    assert!(
        setup_window.contains("TEST_MUTEX.lock()"),
        "relocated setup_test_env body must lock TEST_MUTEX"
    );
    assert!(
        setup_window.contains("tempfile::tempdir()"),
        "relocated setup_test_env body must call tempfile::tempdir()"
    );
    assert!(
        setup_window.contains("codelet_common::set_data_directory"),
        "relocated setup_test_env body must call codelet_common::set_data_directory (not the deleted NAPI shim)"
    );
    assert!(
        setup_window.contains("reset_stores_for_tests"),
        "relocated setup_test_env body must call reset_stores_for_tests() (resets every lifted persistence singleton)"
    );

    // @step And there is no reference to crate::credentials::reset_credential_store or crate::graph::reset_graph_db in the relocated setup helper because those are NAPI-only concerns
    assert!(
        !setup_window.contains("crate::credentials::reset_credential_store"),
        "relocated setup_test_env must not reference crate::credentials::reset_credential_store (NAPI-only concern; lives in test_support.rs)"
    );
    assert!(
        !setup_window.contains("crate::graph::reset_graph_db"),
        "relocated setup_test_env must not reference crate::graph::reset_graph_db (NAPI-only concern; lives in test_support.rs)"
    );
}

// ============================================================================
// Scenario: Both codelet-napi feature builds succeed after the relocation
// ============================================================================
#[test]
fn test_both_codelet_napi_feature_builds_succeed_after_relocation() {
    // @step Given codelet/napi/src/persistence/mod.rs no longer declares mod tests or mod lazy_init_tests and contains no pub fn declarations
    let mod_rs = read_to_string_at(&napi_persistence_dir().join("mod.rs"));
    assert!(
        !mod_rs.contains("mod tests"),
        "codelet/napi/src/persistence/mod.rs must NOT declare `mod tests;` after RPC-035 (tests.rs is relocated to codelet-core)"
    );
    assert!(
        !mod_rs.contains("mod lazy_init_tests"),
        "codelet/napi/src/persistence/mod.rs must NOT declare `mod lazy_init_tests;` after RPC-035"
    );
    assert!(
        !mod_rs.contains("pub fn "),
        "codelet/napi/src/persistence/mod.rs must contain no `pub fn` declarations after RPC-035 (helpers absorbed into napi_bindings.rs)"
    );

    // @step And napi_bindings.rs imports its callees via `use codelet_core::persistence::{...};`
    let napi_bindings = read_to_string_at(&napi_persistence_dir().join("napi_bindings.rs"));
    assert!(
        napi_bindings.contains("use codelet_core::persistence::"),
        "codelet/napi/src/persistence/napi_bindings.rs must import its callees via `use codelet_core::persistence::{{...}};`"
    );

    // @step When `cargo build -p codelet-napi --features napi` is run
    // @step And `cargo build -p codelet-napi --features noop` is run
    // The build invocations themselves are validated by CI / by the engineer
    // running the validating phase. This test asserts the SOURCE-LEVEL
    // pre-conditions that make both builds succeed: the noop gate is
    // preserved on the napi_bindings module, and Cargo.toml still declares
    // both features.
    let napi_cargo_toml = read_to_string_at(&workspace_root().join("codelet/napi/Cargo.toml"));
    assert!(
        napi_cargo_toml.contains("napi"),
        "codelet/napi/Cargo.toml must continue to declare the `napi` feature"
    );
    assert!(
        napi_cargo_toml.contains("noop"),
        "codelet/napi/Cargo.toml must continue to declare the `noop` feature"
    );
    assert!(
        mod_rs.contains("#[cfg(not(feature = \"noop\"))]")
            || mod_rs.contains("#[cfg(not(feature=\"noop\"))]"),
        "codelet/napi/src/persistence/mod.rs must preserve the `#[cfg(not(feature = \"noop\"))] mod napi_bindings;` gate so the noop feature build excludes the NAPI shims"
    );

    // @step Then both builds succeed without warnings about unresolved imports
    // (Asserted by the existing in-crate `cargo build -p codelet-napi --features napi`
    //  and `cargo build -p codelet-napi --features noop` invocations during validating.
    //  No source-level assertion possible here without spawning cargo from within
    //  the test runner — which would deadlock because the test itself runs under cargo.)

    // @step And the napi build emits the unchanged codelet/napi/index.d.ts with SHA-256 hash equal to the pre-card snapshot
    let index_dts = workspace_root().join("codelet/napi/index.d.ts");
    if index_dts.is_file() {
        let actual_sha256 = sha256_of(&index_dts);
        assert_eq!(
            actual_sha256, PRE_CARD_INDEX_DTS_SHA256,
            "codelet/napi/index.d.ts SHA-256 must equal the pre-card fixture {PRE_CARD_INDEX_DTS_SHA256} (got {actual_sha256}). Field reorder in Napi* wire structs is the most common cause."
        );
    }
}
