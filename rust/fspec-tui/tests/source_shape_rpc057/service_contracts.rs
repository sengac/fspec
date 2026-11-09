/// Scenario: All five merge/worktree wire types are exported from codelet-rpc-types
#[test]
fn rpc_types_exports_merge_worktree_wire_types() -> TestResult {
    // @step Given the file rust/rpc-types/src/lib.rs is compiled
    let path = workspace_root()?.join("rust/rpc-types/src/lib.rs");
    let source = codelet_test_helpers::source_composition::read_tree_or_file(&path).or_fail("read rpc-types/src/lib.rs")?;
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
    Ok(())
}

/// Scenario: SessionManagerHandle declares the five new methods
#[test]
fn session_manager_handle_declares_merge_worktree_methods() -> TestResult {
    // @step Given the file rust/core/src/session_manager_handle.rs is compiled
    let path = workspace_root()?.join("rust/core/src/session_manager_handle.rs");
    let source = codelet_test_helpers::source_composition::read_tree_or_file(&path).or_fail("read session_manager_handle.rs")?;
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

    // @step And it declares a trait method named "list_session_worktrees" returning Result<Vec<SessionWorktreeInfo>, String>
    assert!(
        source.contains("fn list_session_worktrees("),
        "session_manager_handle.rs should declare fn list_session_worktrees"
    );
    assert!(
        normalised.contains("-> Result<Vec<SessionWorktreeInfo>, String>"),
        "list_session_worktrees should return Result<Vec<SessionWorktreeInfo>, String>"
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
    Ok(())
}

/// Scenario: StubSessionManagerHandle exposes per-call counters for all five methods
#[test]
fn stub_exposes_per_call_counters() -> TestResult {
    // @step Given the file rust/core/src/session_manager_handle.rs is compiled
    let path = workspace_root()?.join("rust/core/src/session_manager_handle.rs");
    let source = codelet_test_helpers::source_composition::read_tree_or_file(&path).or_fail("read session_manager_handle.rs")?;
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
    Ok(())
}

/// Scenario: FspecService declares the five new RPC methods
#[test]
fn fspec_service_declares_merge_worktree_methods() -> TestResult {
    // @step Given the file rust/rpc/src/lib.rs is compiled
    let path = workspace_root()?.join("rust/rpc/src/lib.rs");
    let source = read_source(&path).or_fail("resolve rpc/src/lib.rs")?;
    let normalised = normalise(&source);

    // @step Then it declares an async fn named "merge_session_worktree" with return type Result<MergeOutcome, String>
    // @step And it declares an async fn named "discard_session_worktree" with return type Result<(), String>
    // @step And it declares an async fn named "prune_orphaned_worktrees" with return type Result<Vec<String>, String>
    // @step And it declares an async fn named "list_session_worktrees" with return type Result<Vec<SessionWorktreeInfo>, String>
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
        normalised.contains("-> Result<Vec<SessionWorktreeInfo>, String>"),
        "list_session_worktrees should return Result<Vec<SessionWorktreeInfo>, String>"
    );
    assert!(
        normalised.contains("-> Result<SessionChangesSummary, String>"),
        "inspect_session_changes should return Result<SessionChangesSummary, String>"
    );
    Ok(())
}
