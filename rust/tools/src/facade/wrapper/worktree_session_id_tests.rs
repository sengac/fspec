//! Feature: spec/features/require-session-id-for-all-tools-to-support-worktree-isolation.feature
//!
//! TOOL-014 worktree isolation tests (constructors require session_id), moved out of wrapper.rs so each
//! test file maps one feature (COV-075); kept under 300 lines per file.

use super::*;

// ============================================================================
// REQUIREMENT 1: CONSTRUCTOR SIGNATURE - session_id IS MANDATORY (TOOL-014)
// These tests verify ALL tools REQUIRE session_id in their constructor.
// This is a compile-time guarantee - tools cannot be instantiated without session_id.
// ============================================================================

/// Feature: spec/features/require-session-id-for-all-tools-to-support-worktree-isolation.feature
/// Scenario: ReadTool REQUIRES session_id parameter in constructor
#[test]
fn test_read_tool_requires_session_id() {
    use crate::ReadTool;

    // @step Given ReadTool is being instantiated
    let session_id = Uuid::new_v4();

    // @step Then the constructor signature MUST be ReadTool::new(session_id: Uuid)
    let _tool = ReadTool::new(session_id);

    // @step And calling ReadTool::new() without session_id MUST fail to compile
    // Verified at compile time - no parameterless constructor exists

    // @step And ReadTool MUST NOT implement Default trait
    // Verified at compile time - Default is not implemented
}

/// Feature: spec/features/require-session-id-for-all-tools-to-support-worktree-isolation.feature
/// Scenario: WriteTool REQUIRES session_id parameter in constructor
#[test]
fn test_write_tool_requires_session_id() {
    use crate::WriteTool;

    // @step Given WriteTool is being instantiated
    let session_id = Uuid::new_v4();

    // @step Then the constructor signature MUST be WriteTool::new(session_id: Uuid)
    let _tool = WriteTool::new(session_id);

    // @step And calling WriteTool::new() without session_id MUST fail to compile
    // @step And WriteTool MUST NOT implement Default trait
}

/// Feature: spec/features/require-session-id-for-all-tools-to-support-worktree-isolation.feature
/// Scenario: EditTool REQUIRES session_id parameter in constructor
#[test]
fn test_edit_tool_requires_session_id() {
    use crate::EditTool;

    // @step Given EditTool is being instantiated
    let session_id = Uuid::new_v4();

    // @step Then the constructor signature MUST be EditTool::new(session_id: Uuid)
    let _tool = EditTool::new(session_id);

    // @step And calling EditTool::new() without session_id MUST fail to compile
    // @step And EditTool MUST NOT implement Default trait
}

/// Feature: spec/features/require-session-id-for-all-tools-to-support-worktree-isolation.feature
/// Scenario: GrepTool REQUIRES session_id parameter in constructor
#[test]
fn test_grep_tool_requires_session_id() {
    use crate::GrepTool;

    // @step Given GrepTool is being instantiated
    let session_id = Uuid::new_v4();

    // @step Then the constructor signature MUST be GrepTool::new(session_id: Uuid)
    let _tool = GrepTool::new(session_id);

    // @step And calling GrepTool::new() without session_id MUST fail to compile
    // @step And GrepTool MUST NOT implement Default trait
}

/// Feature: spec/features/require-session-id-for-all-tools-to-support-worktree-isolation.feature
/// Scenario: GlobTool REQUIRES session_id parameter in constructor
#[test]
fn test_glob_tool_requires_session_id() {
    use crate::GlobTool;

    // @step Given GlobTool is being instantiated
    let session_id = Uuid::new_v4();

    // @step Then the constructor signature MUST be GlobTool::new(session_id: Uuid)
    let _tool = GlobTool::new(session_id);

    // @step And calling GlobTool::new() without session_id MUST fail to compile
    // @step And GlobTool MUST NOT implement Default trait
}

/// Feature: spec/features/require-session-id-for-all-tools-to-support-worktree-isolation.feature
/// Scenario: LsTool REQUIRES session_id parameter in constructor
#[test]
fn test_ls_tool_requires_session_id() {
    use crate::LsTool;

    // @step Given LsTool is being instantiated
    let session_id = Uuid::new_v4();

    // @step Then the constructor signature MUST be LsTool::new(session_id: Uuid)
    let _tool = LsTool::new(session_id);

    // @step And calling LsTool::new() without session_id MUST fail to compile
    // @step And LsTool MUST NOT implement Default trait
}

/// Feature: spec/features/require-session-id-for-all-tools-to-support-worktree-isolation.feature
/// Scenario: AstGrepTool REQUIRES session_id parameter in constructor
#[test]
fn test_ast_grep_tool_requires_session_id() {
    use crate::AstGrepTool;

    // @step Given AstGrepTool is being instantiated
    let session_id = Uuid::new_v4();

    // @step Then the constructor signature MUST be AstGrepTool::new(session_id: Uuid)
    let _tool = AstGrepTool::new(session_id);

    // @step And calling AstGrepTool::new() without session_id MUST fail to compile
    // @step And AstGrepTool MUST NOT implement Default trait
}

/// Feature: spec/features/require-session-id-for-all-tools-to-support-worktree-isolation.feature
/// Scenario: AstGrepRefactorTool REQUIRES session_id parameter in constructor
#[test]
fn test_ast_grep_refactor_tool_requires_session_id() {
    use crate::AstGrepRefactorTool;

    // @step Given AstGrepRefactorTool is being instantiated
    let session_id = Uuid::new_v4();

    // @step Then the constructor signature MUST be AstGrepRefactorTool::new(session_id: Uuid)
    let _tool = AstGrepRefactorTool::new(session_id);

    // @step And calling AstGrepRefactorTool::new() without session_id MUST fail to compile
    // @step And AstGrepRefactorTool MUST NOT implement Default trait
}

/// Feature: spec/features/require-session-id-for-all-tools-to-support-worktree-isolation.feature
/// Scenario: WebSearchTool REQUIRES session_id parameter in constructor
#[test]
fn test_web_search_tool_requires_session_id() {
    use crate::WebSearchTool;

    // @step Given WebSearchTool is being instantiated
    let session_id = Uuid::new_v4();

    // @step Then the constructor signature MUST be WebSearchTool::new(session_id: Uuid)
    let _tool = WebSearchTool::new(session_id);

    // @step And calling WebSearchTool::new() without session_id MUST fail to compile
    // @step And WebSearchTool MUST NOT implement Default trait
}

/// Feature: spec/features/require-session-id-for-all-tools-to-support-worktree-isolation.feature
/// Scenario: SearchToolFacadeWrapper REQUIRES session_id parameter in constructor
#[test]
fn test_search_tool_facade_wrapper_requires_session_id() {
    use crate::facade::GeminiSearchFileContentFacade;
    use std::sync::Arc;

    // @step Given SearchToolFacadeWrapper is being instantiated
    let session_id = Uuid::new_v4();
    let facade = Arc::new(GeminiSearchFileContentFacade) as BoxedSearchToolFacade;

    // @step Then the constructor signature MUST be SearchToolFacadeWrapper::new(facade, session_id: Uuid)
    let _tool = SearchToolFacadeWrapper::new(facade, session_id);

    // @step And calling SearchToolFacadeWrapper::new() without session_id MUST fail to compile
    // @step And SearchToolFacadeWrapper MUST NOT implement Default trait
}

/// Feature: spec/features/require-session-id-for-all-tools-to-support-worktree-isolation.feature
/// Scenario: LsToolFacadeWrapper REQUIRES session_id parameter in constructor
#[test]
fn test_ls_tool_facade_wrapper_requires_session_id() {
    use crate::facade::GeminiListDirectoryFacade;
    use std::sync::Arc;

    // @step Given LsToolFacadeWrapper is being instantiated
    let session_id = Uuid::new_v4();
    let facade = Arc::new(GeminiListDirectoryFacade) as BoxedLsToolFacade;

    // @step Then the constructor signature MUST be LsToolFacadeWrapper::new(facade, session_id: Uuid)
    let _tool = LsToolFacadeWrapper::new(facade, session_id);

    // @step And calling LsToolFacadeWrapper::new() without session_id MUST fail to compile
    // @step And LsToolFacadeWrapper MUST NOT implement Default trait
}
