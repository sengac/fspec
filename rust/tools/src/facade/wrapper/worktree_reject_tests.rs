//! Feature: spec/features/require-session-id-for-all-tools-to-support-worktree-isolation.feature
//!
//! TOOL-014 worktree isolation tests (paths outside the worktree are rejected), moved out of wrapper.rs so each
//! test file maps one feature (COV-075); kept under 300 lines per file.

use super::*;

/// Feature: spec/features/require-session-id-for-all-tools-to-support-worktree-isolation.feature
/// Scenario: ReadTool rejects absolute path outside worktree
#[test]
fn test_validate_and_resolve_path_rejects_absolute_outside_worktree() {
    // @step Given an isolated session with worktree at ".fspec/worktrees/abc123/"
    let worktree = PathBuf::from("/project/.fspec/worktrees/abc123");

    // @step When ReadTool attempts to read "/project/src/main.rs"
    let result =
        validate_and_resolve_path_with_cwd("/project/src/main.rs", Some(&worktree), "read");

    // @step Then it should return ToolError::Validation with tool "read"
    assert!(result.is_err(), "Should reject path outside worktree");

    // @step And the error message should contain "outside isolated worktree"
    let err = result.unwrap_err();
    assert!(matches!(err, ToolError::Validation { tool: "read", .. }));
    assert!(
        err.to_string().contains("blocked") || err.to_string().contains("outside"),
        "Error should mention worktree isolation: {err}"
    );
}

/// Feature: spec/features/require-session-id-for-all-tools-to-support-worktree-isolation.feature
/// Scenario: WriteTool rejects absolute path outside worktree
#[test]
fn test_validate_and_resolve_path_rejects_write_outside_worktree() {
    // @step Given an isolated session with worktree at ".fspec/worktrees/abc123/"
    let worktree = PathBuf::from("/project/.fspec/worktrees/abc123");

    // @step When WriteTool attempts to write to "/project/src/new.rs"
    let result =
        validate_and_resolve_path_with_cwd("/project/src/new.rs", Some(&worktree), "write");

    // @step Then it should return ToolError::Validation with tool "write"
    assert!(result.is_err(), "Should reject path outside worktree");

    // @step And the error message should contain "outside isolated worktree"
    let err = result.unwrap_err();
    assert!(matches!(err, ToolError::Validation { tool: "write", .. }));
}

/// Feature: spec/features/require-session-id-for-all-tools-to-support-worktree-isolation.feature
/// Scenario: EditTool rejects absolute path outside worktree
#[test]
fn test_validate_and_resolve_path_rejects_edit_outside_worktree() {
    // @step Given an isolated session with worktree at ".fspec/worktrees/abc123/"
    let worktree = PathBuf::from("/project/.fspec/worktrees/abc123");

    // @step When EditTool attempts to edit "/project/src/lib.rs"
    let result = validate_and_resolve_path_with_cwd("/project/src/lib.rs", Some(&worktree), "edit");

    // @step Then it should return ToolError::Validation with tool "edit"
    assert!(result.is_err(), "Should reject path outside worktree");

    // @step And the error message should contain "outside isolated worktree"
    let err = result.unwrap_err();
    assert!(matches!(err, ToolError::Validation { tool: "edit", .. }));
}

/// Feature: spec/features/require-session-id-for-all-tools-to-support-worktree-isolation.feature
/// Scenario: GrepTool rejects absolute path outside worktree
#[test]
fn test_validate_and_resolve_path_rejects_grep_outside_worktree() {
    // @step Given an isolated session with worktree at ".fspec/worktrees/abc123/"
    let worktree = PathBuf::from("/project/.fspec/worktrees/abc123");

    // @step When GrepTool attempts to search with path "/project/src"
    let result = validate_and_resolve_path_with_cwd("/project/src", Some(&worktree), "grep");

    // @step Then it should return ToolError::Validation with tool "grep"
    assert!(result.is_err(), "Should reject path outside worktree");

    // @step And the error message should contain "outside isolated worktree"
    let err = result.unwrap_err();
    assert!(matches!(err, ToolError::Validation { tool: "grep", .. }));
}

/// Feature: spec/features/require-session-id-for-all-tools-to-support-worktree-isolation.feature
/// Scenario: GlobTool rejects absolute path outside worktree
#[test]
fn test_validate_and_resolve_path_rejects_glob_outside_worktree() {
    // @step Given an isolated session with worktree at ".fspec/worktrees/abc123/"
    let worktree = PathBuf::from("/project/.fspec/worktrees/abc123");

    // @step When GlobTool attempts to search with path "/project/src"
    let result = validate_and_resolve_path_with_cwd("/project/src", Some(&worktree), "glob");

    // @step Then it should return ToolError::Validation with tool "glob"
    assert!(result.is_err(), "Should reject path outside worktree");

    // @step And the error message should contain "outside isolated worktree"
    let err = result.unwrap_err();
    assert!(matches!(err, ToolError::Validation { tool: "glob", .. }));
}

/// Feature: spec/features/require-session-id-for-all-tools-to-support-worktree-isolation.feature
/// Scenario: LsTool rejects absolute path outside worktree
#[test]
fn test_validate_and_resolve_path_rejects_ls_outside_worktree() {
    // @step Given an isolated session with worktree at ".fspec/worktrees/abc123/"
    let worktree = PathBuf::from("/project/.fspec/worktrees/abc123");

    // @step When LsTool attempts to list "/project/src"
    let result = validate_and_resolve_path_with_cwd("/project/src", Some(&worktree), "ls");

    // @step Then it should return ToolError::Validation with tool "ls"
    assert!(result.is_err(), "Should reject path outside worktree");

    // @step And the error message should contain "outside isolated worktree"
    let err = result.unwrap_err();
    assert!(matches!(err, ToolError::Validation { tool: "ls", .. }));
}

/// Feature: spec/features/require-session-id-for-all-tools-to-support-worktree-isolation.feature
/// Scenario: AstGrepTool rejects absolute path outside worktree
#[test]
fn test_validate_and_resolve_path_rejects_astgrep_outside_worktree() {
    // @step Given an isolated session with worktree at ".fspec/worktrees/abc123/"
    let worktree = PathBuf::from("/project/.fspec/worktrees/abc123");

    // @step When AstGrepTool attempts to search with path "/project/src"
    let result = validate_and_resolve_path_with_cwd("/project/src", Some(&worktree), "ast_grep");

    // @step Then it should return ToolError::Validation with tool "ast_grep"
    assert!(result.is_err(), "Should reject path outside worktree");

    // @step And the error message should contain "outside isolated worktree"
    let err = result.unwrap_err();
    assert!(matches!(
        err,
        ToolError::Validation {
            tool: "ast_grep",
            ..
        }
    ));
}

/// Feature: spec/features/require-session-id-for-all-tools-to-support-worktree-isolation.feature
/// Scenario: AstGrepRefactorTool rejects absolute path outside worktree
#[test]
fn test_validate_and_resolve_path_rejects_astgrep_refactor_outside_worktree() {
    // @step Given an isolated session with worktree at ".fspec/worktrees/abc123/"
    let worktree = PathBuf::from("/project/.fspec/worktrees/abc123");

    // @step When AstGrepRefactorTool attempts to refactor "/project/src/lib.rs"
    let result = validate_and_resolve_path_with_cwd(
        "/project/src/lib.rs",
        Some(&worktree),
        "ast_grep_refactor",
    );

    // @step Then it should return ToolError::Validation with tool "ast_grep_refactor"
    assert!(result.is_err(), "Should reject path outside worktree");

    // @step And the error message should contain "outside isolated worktree"
    let err = result.unwrap_err();
    assert!(matches!(
        err,
        ToolError::Validation {
            tool: "ast_grep_refactor",
            ..
        }
    ));
}
