//! Feature: spec/features/require-session-id-for-all-tools-to-support-worktree-isolation.feature
//!
//! TOOL-014 worktree isolation tests (paths resolve into the worktree), moved out of wrapper.rs so each
//! test file maps one feature (COV-075); kept under 300 lines per file.

use super::*;

// ========================================================================
// Tests for validate_and_resolve_path (TOOL-014 IMPLEMENTATION)
// ========================================================================

/// Feature: spec/features/require-session-id-for-all-tools-to-support-worktree-isolation.feature
/// Scenario: Tools operate normally with Uuid::nil() in tests
#[test]
fn test_validate_and_resolve_path_no_worktree() {
    // @step Given a non-isolated session with Uuid::nil()
    // (effective_cwd is None)

    // @step When get_effective_cwd is called
    // @step Then it should return None
    // @step And tools should operate in the current directory without path validation
    let result = validate_and_resolve_path_with_cwd("src/file.rs", None, "read");
    assert!(result.is_ok(), "Should succeed without worktree");
    assert_eq!(result.unwrap(), PathBuf::from("src/file.rs"));

    let result = validate_and_resolve_path_with_cwd("/absolute/path/file.rs", None, "read");
    assert!(
        result.is_ok(),
        "Absolute path should succeed without worktree"
    );
    assert_eq!(result.unwrap(), PathBuf::from("/absolute/path/file.rs"));
}

/// Feature: spec/features/require-session-id-for-all-tools-to-support-worktree-isolation.feature
/// Scenario: ReadTool resolves relative path to worktree in isolated session
#[test]
fn test_validate_and_resolve_relative_to_worktree() {
    // @step Given an isolated session with worktree at ".fspec/worktrees/abc123/"
    let worktree = PathBuf::from("/project/.fspec/worktrees/abc123");

    // @step When ReadTool reads "src/file.rs"
    let result = validate_and_resolve_path_with_cwd("src/file.rs", Some(&worktree), "read");

    // @step Then it should read from ".fspec/worktrees/abc123/src/file.rs"
    assert!(result.is_ok(), "Should resolve relative path");
    assert_eq!(
        result.unwrap(),
        PathBuf::from("/project/.fspec/worktrees/abc123/src/file.rs"),
        "Relative path should be resolved to worktree"
    );
}

/// BUG-456: the worktree the isolation callback below reports for one
/// session (every other session is not isolated).
static BASH_WORKTREE: std::sync::OnceLock<(Uuid, PathBuf)> = std::sync::OnceLock::new();

fn bash_worktree_isolation(session_id: String) -> Result<Option<IsolationContext>, String> {
    Ok(BASH_WORKTREE
        .get()
        .filter(|(isolated, _)| isolated.to_string() == session_id)
        .map(|(_, worktree)| IsolationContext {
            worktree_path: worktree.clone(),
            blocked_project_path: worktree.join("../../.."),
        }))
}

/// Feature: spec/features/require-session-id-for-all-tools-to-support-worktree-isolation.feature
/// Scenario: BashTool runs in worktree directory in isolated session
#[tokio::test]
async fn test_bash_tool_runs_in_the_session_worktree() -> Result<(), String> {
    use rig::tool::Tool;

    codelet_test_helpers::isolated_user_directory()?;
    // @step Given an isolated session with worktree at ".fspec/worktrees/abc123/"
    let project = tempfile::tempdir().map_err(|error| error.to_string())?;
    let worktree = project.path().join(".fspec/worktrees/abc123");
    std::fs::create_dir_all(&worktree).map_err(|error| error.to_string())?;
    let session_id = Uuid::new_v4();
    BASH_WORKTREE
        .set((session_id, worktree))
        .map_err(|_| "the bash worktree is already set")?;
    set_get_effective_cwd_callback(bash_worktree_isolation);

    // @step When BashTool runs "pwd"
    let output = crate::bash::BashTool::new(session_id)
        .call(crate::bash::BashArgs {
            command: "pwd".to_string(),
            cwd: None,
        })
        .await
        .map_err(|error| error.to_string())?;

    // @step Then the output should contain ".fspec/worktrees/abc123"
    assert!(output.contains(".fspec/worktrees/abc123"), "{output}");
    Ok(())
}

/// Feature: spec/features/require-session-id-for-all-tools-to-support-worktree-isolation.feature
/// Scenario: LsTool lists worktree root in isolated session
#[test]
fn test_validate_and_resolve_dot_to_worktree() {
    // @step Given an isolated session with worktree at ".fspec/worktrees/abc123/"
    let worktree = PathBuf::from("/project/.fspec/worktrees/abc123");

    // @step When LsTool lists "."
    let result = validate_and_resolve_path_with_cwd(".", Some(&worktree), "ls");

    // @step Then it should list contents of ".fspec/worktrees/abc123/"
    assert!(result.is_ok(), "Should resolve dot path");
    assert_eq!(
        result.unwrap(),
        PathBuf::from("/project/.fspec/worktrees/abc123/.")
    );
}

// ========================================================================
// Additional Happy Path Tests for Tool-Specific Scenarios
// ========================================================================

/// Feature: spec/features/require-session-id-for-all-tools-to-support-worktree-isolation.feature
/// Scenario: WriteTool writes to worktree path in isolated session
#[test]
fn test_write_tool_resolves_to_worktree() {
    // @step Given an isolated session with worktree at ".fspec/worktrees/abc123/"
    let worktree = PathBuf::from("/project/.fspec/worktrees/abc123");

    // @step When WriteTool writes to "src/new.rs"
    let result = validate_and_resolve_path_with_cwd("src/new.rs", Some(&worktree), "write");

    // @step Then the file should be created at ".fspec/worktrees/abc123/src/new.rs"
    assert!(result.is_ok(), "Should resolve relative path");
    assert_eq!(
        result.unwrap(),
        PathBuf::from("/project/.fspec/worktrees/abc123/src/new.rs")
    );

    // @step And the main project directory should be unchanged
    // (Verified by path being within worktree)
}

/// Feature: spec/features/require-session-id-for-all-tools-to-support-worktree-isolation.feature
/// Scenario: EditTool edits file in worktree in isolated session
#[test]
fn test_edit_tool_resolves_to_worktree() {
    // @step Given an isolated session with worktree at ".fspec/worktrees/abc123/"
    let worktree = PathBuf::from("/project/.fspec/worktrees/abc123");

    // @step And a file exists at ".fspec/worktrees/abc123/src/lib.rs"
    // (File existence is checked by EditTool, not path validation)

    // @step When EditTool edits "src/lib.rs"
    let result = validate_and_resolve_path_with_cwd("src/lib.rs", Some(&worktree), "edit");

    // @step Then it should modify ".fspec/worktrees/abc123/src/lib.rs"
    assert!(result.is_ok(), "Should resolve relative path");
    assert_eq!(
        result.unwrap(),
        PathBuf::from("/project/.fspec/worktrees/abc123/src/lib.rs")
    );

    // @step And the main project file should be unchanged
    // (Verified by path being within worktree)
}

/// Feature: spec/features/require-session-id-for-all-tools-to-support-worktree-isolation.feature
/// Scenario: GrepTool searches within worktree in isolated session
#[test]
fn test_grep_tool_resolves_to_worktree() {
    // @step Given an isolated session with worktree at ".fspec/worktrees/abc123/"
    let worktree = PathBuf::from("/project/.fspec/worktrees/abc123");

    // @step When GrepTool searches with path "src/"
    let result = validate_and_resolve_path_with_cwd("src/", Some(&worktree), "grep");

    // @step Then it should search within ".fspec/worktrees/abc123/src/"
    assert!(result.is_ok(), "Should resolve relative path");
    assert_eq!(
        result.unwrap(),
        PathBuf::from("/project/.fspec/worktrees/abc123/src/")
    );

    // @step And it should not search the main project directory
    // (Verified by path being within worktree)
}

/// Feature: spec/features/require-session-id-for-all-tools-to-support-worktree-isolation.feature
/// Scenario: GlobTool finds files only within worktree in isolated session
#[test]
fn test_glob_tool_resolves_to_worktree() {
    // @step Given an isolated session with worktree at ".fspec/worktrees/abc123/"
    let worktree = PathBuf::from("/project/.fspec/worktrees/abc123");

    // @step When GlobTool searches for "*.rs" without a path
    // Note: GlobTool uses "." as default path if not specified
    let result = validate_and_resolve_path_with_cwd(".", Some(&worktree), "glob");

    // @step Then it should only return files within ".fspec/worktrees/abc123/"
    assert!(result.is_ok(), "Should resolve dot path");
    assert_eq!(
        result.unwrap(),
        PathBuf::from("/project/.fspec/worktrees/abc123/.")
    );

    // @step And it should not return files from the main project
    // (Verified by search starting within worktree)
}

/// Feature: spec/features/require-session-id-for-all-tools-to-support-worktree-isolation.feature
/// Scenario: AstGrepTool searches within worktree in isolated session
#[test]
fn test_astgrep_tool_resolves_to_worktree() {
    // @step Given an isolated session with worktree at ".fspec/worktrees/abc123/"
    let worktree = PathBuf::from("/project/.fspec/worktrees/abc123");

    // @step When AstGrepTool searches with path "src/"
    let result = validate_and_resolve_path_with_cwd("src/", Some(&worktree), "ast_grep");

    // @step Then it should search within ".fspec/worktrees/abc123/src/"
    assert!(result.is_ok(), "Should resolve relative path");
    assert_eq!(
        result.unwrap(),
        PathBuf::from("/project/.fspec/worktrees/abc123/src/")
    );
}

/// Feature: spec/features/require-session-id-for-all-tools-to-support-worktree-isolation.feature
/// Scenario: AstGrepRefactorTool modifies worktree copy only
#[test]
fn test_astgrep_refactor_tool_resolves_to_worktree() {
    // @step Given an isolated session with worktree at ".fspec/worktrees/abc123/"
    let worktree = PathBuf::from("/project/.fspec/worktrees/abc123");

    // @step And a file exists at ".fspec/worktrees/abc123/src/lib.rs"
    // (File existence is checked by AstGrepRefactorTool, not path validation)

    // @step When AstGrepRefactorTool refactors "src/lib.rs"
    let result =
        validate_and_resolve_path_with_cwd("src/lib.rs", Some(&worktree), "ast_grep_refactor");

    // @step Then it should modify ".fspec/worktrees/abc123/src/lib.rs"
    assert!(result.is_ok(), "Should resolve relative path");
    assert_eq!(
        result.unwrap(),
        PathBuf::from("/project/.fspec/worktrees/abc123/src/lib.rs")
    );

    // @step And the main project file should be unchanged
    // (Verified by path being within worktree)
}

/// Feature: spec/features/require-session-id-for-all-tools-to-support-worktree-isolation.feature
/// Scenario: WriteTool writes to worktree path in isolated session
#[test]
fn test_validate_and_resolve_path_resolves_relative_to_worktree() {
    // @step Given an isolated session with worktree at ".fspec/worktrees/abc123/"
    let worktree = PathBuf::from("/project/.fspec/worktrees/abc123");

    // @step When WriteTool writes to "src/new.rs"
    let result = validate_and_resolve_path_with_cwd("src/new.rs", Some(&worktree), "write");

    // @step Then the file should be created at ".fspec/worktrees/abc123/src/new.rs"
    assert!(result.is_ok(), "Should resolve relative path to worktree");
    assert_eq!(
        result.unwrap(),
        PathBuf::from("/project/.fspec/worktrees/abc123/src/new.rs")
    );

    // @step And the main project directory should be unchanged
    // (This is verified by the path being within worktree, not main project)
}

/// Feature: spec/features/require-session-id-for-all-tools-to-support-worktree-isolation.feature
/// Scenario: Tools operate normally with Uuid::nil() in tests
#[test]
fn test_validate_and_resolve_path_allows_all_paths_with_nil_session() {
    // @step Given a non-isolated session with Uuid::nil()
    // (effective_cwd is None)

    // @step When get_effective_cwd is called
    // @step Then it should return None
    // (simulated by passing None to validate_and_resolve_path_with_cwd)

    // @step And tools should operate in the current directory without path validation
    let result = validate_and_resolve_path_with_cwd("/any/absolute/path", None, "read");
    assert!(
        result.is_ok(),
        "Should allow any path when no worktree isolation"
    );
    assert_eq!(result.unwrap(), PathBuf::from("/any/absolute/path"));

    let result = validate_and_resolve_path_with_cwd("relative/path", None, "write");
    assert!(
        result.is_ok(),
        "Should allow relative paths when no worktree isolation"
    );
    assert_eq!(result.unwrap(), PathBuf::from("relative/path"));
}
