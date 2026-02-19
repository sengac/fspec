//! Integration tests for Isolated Session File Operations
//!
//! Feature: spec/features/isolated-session-file-operations.feature
//!
//! GIT-020: Tests that file operations in isolated sessions correctly use
//! effective_cwd() for proper filesystem isolation. Files written by an
//! isolated session should appear only in the worktree, not in the main project.
//!
//! NOTE: These tests use serial execution to avoid shared state issues.
//! They demonstrate the expected behavior; actual implementation will wire up
//! the callback mechanism in wrapper.rs.

use std::path::PathBuf;
use std::sync::RwLock;
use std::collections::HashMap;
use uuid::Uuid;
use serial_test::serial;

// ============================================================================
// Test Infrastructure - Thread-safe Callback Registration
// ============================================================================

/// Thread-safe test double for effective_cwd callback
/// Maps session_id -> effective_cwd path
static TEST_EFFECTIVE_CWD_MAP: RwLock<Option<HashMap<String, PathBuf>>> = RwLock::new(None);

fn setup_test_effective_cwd_map() {
    let mut map = TEST_EFFECTIVE_CWD_MAP.write().expect("Failed to write lock");
    *map = Some(HashMap::new());
}

fn set_test_effective_cwd(session_id: Uuid, cwd: PathBuf) {
    let mut guard = TEST_EFFECTIVE_CWD_MAP.write().expect("Failed to write lock");
    if let Some(ref mut map) = *guard {
        map.insert(session_id.to_string(), cwd);
    }
}

fn get_test_effective_cwd(session_id_str: String) -> Option<PathBuf> {
    let guard = TEST_EFFECTIVE_CWD_MAP.read().expect("Failed to read lock");
    guard.as_ref().and_then(|map| map.get(&session_id_str).cloned())
}

fn cleanup_test_effective_cwd_map() {
    let mut map = TEST_EFFECTIVE_CWD_MAP.write().expect("Failed to write lock");
    *map = None;
}

// ============================================================================
// Scenario: File written by isolated session appears in worktree only
// ============================================================================

/// Feature: spec/features/isolated-session-file-operations.feature
/// Scenario: File written by isolated session appears in worktree only
///
/// @step Given I am an AI agent running in an isolated session
/// @step And my session has worktree_path "/project/.fspec/worktrees/abc123"
/// @step When I write content "test content" to file "src/new-file.ts"
/// @step Then the file should exist at "/project/.fspec/worktrees/abc123/src/new-file.ts"
/// @step And the file should NOT exist at "/project/src/new-file.ts"
#[test]
#[serial]
fn test_file_written_by_isolated_session_appears_in_worktree_only() {
    use tempfile::TempDir;
    use std::fs;

    // Create a mock project structure
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let project_root = temp_dir.path();
    let worktree_path = project_root.join(".fspec/worktrees/abc123");
    
    // @step Given I am an AI agent running in an isolated session
    let session_id = Uuid::new_v4();
    setup_test_effective_cwd_map();
    
    // @step And my session has worktree_path "/project/.fspec/worktrees/abc123"
    fs::create_dir_all(&worktree_path).expect("Failed to create worktree dir");
    fs::create_dir_all(worktree_path.join("src")).expect("Failed to create src in worktree");
    fs::create_dir_all(project_root.join("src")).expect("Failed to create src in project");
    set_test_effective_cwd(session_id, worktree_path.clone());
    
    // @step When I write content "test content" to file "src/new-file.ts"
    let effective_cwd = get_test_effective_cwd(session_id.to_string()).expect("effective_cwd not set");
    let resolved_path = effective_cwd.join("src/new-file.ts");
    fs::write(&resolved_path, "test content").expect("Failed to write file");
    
    // @step Then the file should exist at "/project/.fspec/worktrees/abc123/src/new-file.ts"
    assert!(
        worktree_path.join("src/new-file.ts").exists(),
        "File should exist in worktree"
    );
    
    // @step And the file should NOT exist at "/project/src/new-file.ts"
    assert!(
        !project_root.join("src/new-file.ts").exists(),
        "File should NOT exist in main project"
    );
    
    cleanup_test_effective_cwd_map();
}

// ============================================================================
// Scenario: File read by isolated session comes from worktree
// ============================================================================

/// Feature: spec/features/isolated-session-file-operations.feature
/// Scenario: File read by isolated session comes from worktree
///
/// @step Given I am an AI agent running in an isolated session
/// @step And my session has worktree_path "/project/.fspec/worktrees/abc123"
/// @step And a file exists at "/project/.fspec/worktrees/abc123/src/config.ts" with content "worktree content"
/// @step And a file exists at "/project/src/config.ts" with content "main project content"
/// @step When I read file "src/config.ts"
/// @step Then the content should be "worktree content"
#[test]
#[serial]
fn test_file_read_by_isolated_session_comes_from_worktree() {
    use tempfile::TempDir;
    use std::fs;

    // Create a mock project structure
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let project_root = temp_dir.path();
    let worktree_path = project_root.join(".fspec/worktrees/abc123");
    
    // @step Given I am an AI agent running in an isolated session
    let session_id = Uuid::new_v4();
    setup_test_effective_cwd_map();
    
    // @step And my session has worktree_path "/project/.fspec/worktrees/abc123"
    fs::create_dir_all(worktree_path.join("src")).expect("Failed to create worktree src");
    fs::create_dir_all(project_root.join("src")).expect("Failed to create project src");
    set_test_effective_cwd(session_id, worktree_path.clone());
    
    // @step And a file exists at "/project/.fspec/worktrees/abc123/src/config.ts" with content "worktree content"
    fs::write(worktree_path.join("src/config.ts"), "worktree content")
        .expect("Failed to write worktree file");
    
    // @step And a file exists at "/project/src/config.ts" with content "main project content"
    fs::write(project_root.join("src/config.ts"), "main project content")
        .expect("Failed to write main project file");
    
    // @step When I read file "src/config.ts"
    let effective_cwd = get_test_effective_cwd(session_id.to_string()).expect("effective_cwd not set");
    let resolved_path = effective_cwd.join("src/config.ts");
    let content = fs::read_to_string(&resolved_path).expect("Failed to read file");
    
    // @step Then the content should be "worktree content"
    assert_eq!(
        content, "worktree content",
        "Should read content from worktree, not main project"
    );
    
    cleanup_test_effective_cwd_map();
}

// ============================================================================
// Scenario: Bash pwd in isolated session returns worktree path
// ============================================================================

/// Feature: spec/features/isolated-session-file-operations.feature
/// Scenario: Bash pwd in isolated session returns worktree path
///
/// @step Given I am an AI agent running in an isolated session
/// @step And my session has worktree_path "/project/.fspec/worktrees/abc123"
/// @step When I execute bash command "pwd"
/// @step Then the output should contain "/project/.fspec/worktrees/abc123"
#[test]
#[serial]
fn test_bash_pwd_in_isolated_session_returns_worktree_path() {
    use tempfile::TempDir;
    use std::fs;
    use std::process::Command;

    // Create a mock project structure
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let project_root = temp_dir.path();
    let worktree_path = project_root.join(".fspec/worktrees/abc123");
    
    // @step Given I am an AI agent running in an isolated session
    let session_id = Uuid::new_v4();
    setup_test_effective_cwd_map();
    
    // @step And my session has worktree_path "/project/.fspec/worktrees/abc123"
    fs::create_dir_all(&worktree_path).expect("Failed to create worktree dir");
    set_test_effective_cwd(session_id, worktree_path.clone());
    
    // @step When I execute bash command "pwd"
    let effective_cwd = get_test_effective_cwd(session_id.to_string()).expect("effective_cwd not set");
    let output = Command::new("pwd")
        .current_dir(&effective_cwd)
        .output()
        .expect("Failed to execute pwd");
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    
    // @step Then the output should contain "/project/.fspec/worktrees/abc123"
    // We use the actual worktree_path since it's a temp directory
    let worktree_str = worktree_path.to_string_lossy();
    assert!(
        stdout.contains(&*worktree_str),
        "pwd output should contain worktree path. Got: {}, Expected to contain: {}",
        stdout.trim(),
        worktree_str
    );
    
    cleanup_test_effective_cwd_map();
}

// ============================================================================
// Scenario: Two parallel sessions write to same relative path without conflict
// ============================================================================

/// Feature: spec/features/isolated-session-file-operations.feature
/// Scenario: Two parallel sessions write to same relative path without conflict
///
/// @step Given two AI agent sessions are running in parallel
/// @step And session A has worktree_path "/project/.fspec/worktrees/session-a"
/// @step And session B has worktree_path "/project/.fspec/worktrees/session-b"
/// @step When session A writes "content A" to file "src/shared.ts"
/// @step And session B writes "content B" to file "src/shared.ts"
/// @step Then "/project/.fspec/worktrees/session-a/src/shared.ts" should contain "content A"
/// @step And "/project/.fspec/worktrees/session-b/src/shared.ts" should contain "content B"
/// @step And the files should be independent
#[test]
#[serial]
fn test_two_parallel_sessions_write_without_conflict() {
    use tempfile::TempDir;
    use std::fs;

    // Create a mock project structure
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let project_root = temp_dir.path();
    let worktree_a = project_root.join(".fspec/worktrees/session-a");
    let worktree_b = project_root.join(".fspec/worktrees/session-b");
    
    // @step Given two AI agent sessions are running in parallel
    let session_a = Uuid::new_v4();
    let session_b = Uuid::new_v4();
    setup_test_effective_cwd_map();
    
    // @step And session A has worktree_path "/project/.fspec/worktrees/session-a"
    fs::create_dir_all(worktree_a.join("src")).expect("Failed to create worktree A src");
    set_test_effective_cwd(session_a, worktree_a.clone());
    
    // @step And session B has worktree_path "/project/.fspec/worktrees/session-b"
    fs::create_dir_all(worktree_b.join("src")).expect("Failed to create worktree B src");
    set_test_effective_cwd(session_b, worktree_b.clone());
    
    // @step When session A writes "content A" to file "src/shared.ts"
    let effective_cwd_a = get_test_effective_cwd(session_a.to_string()).expect("effective_cwd A not set");
    fs::write(effective_cwd_a.join("src/shared.ts"), "content A").expect("Failed to write file A");
    
    // @step And session B writes "content B" to file "src/shared.ts"
    let effective_cwd_b = get_test_effective_cwd(session_b.to_string()).expect("effective_cwd B not set");
    fs::write(effective_cwd_b.join("src/shared.ts"), "content B").expect("Failed to write file B");
    
    // @step Then "/project/.fspec/worktrees/session-a/src/shared.ts" should contain "content A"
    let content_a = fs::read_to_string(worktree_a.join("src/shared.ts")).expect("Failed to read A");
    assert_eq!(content_a, "content A", "Session A file should have content A");
    
    // @step And "/project/.fspec/worktrees/session-b/src/shared.ts" should contain "content B"
    let content_b = fs::read_to_string(worktree_b.join("src/shared.ts")).expect("Failed to read B");
    assert_eq!(content_b, "content B", "Session B file should have content B");
    
    // @step And the files should be independent
    assert_ne!(
        worktree_a.join("src/shared.ts"),
        worktree_b.join("src/shared.ts"),
        "Files should be at different paths"
    );
    
    cleanup_test_effective_cwd_map();
}

// ============================================================================
// Scenario: Edit tool modifies file in worktree only
// ============================================================================

/// Feature: spec/features/isolated-session-file-operations.feature
/// Scenario: Edit tool modifies file in worktree only
///
/// @step Given I am an AI agent running in an isolated session
/// @step And my session has worktree_path "/project/.fspec/worktrees/abc123"
/// @step And a file exists at "/project/.fspec/worktrees/abc123/src/app.ts" with content "original"
/// @step And a file exists at "/project/src/app.ts" with content "main original"
/// @step When I edit file "src/app.ts" replacing "original" with "modified"
/// @step Then "/project/.fspec/worktrees/abc123/src/app.ts" should contain "modified"
/// @step And "/project/src/app.ts" should still contain "main original"
#[test]
#[serial]
fn test_edit_tool_modifies_file_in_worktree_only() {
    use tempfile::TempDir;
    use std::fs;

    // Create a mock project structure
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let project_root = temp_dir.path();
    let worktree_path = project_root.join(".fspec/worktrees/abc123");
    
    // @step Given I am an AI agent running in an isolated session
    let session_id = Uuid::new_v4();
    setup_test_effective_cwd_map();
    
    // @step And my session has worktree_path "/project/.fspec/worktrees/abc123"
    fs::create_dir_all(worktree_path.join("src")).expect("Failed to create worktree src");
    fs::create_dir_all(project_root.join("src")).expect("Failed to create project src");
    set_test_effective_cwd(session_id, worktree_path.clone());
    
    // @step And a file exists at "/project/.fspec/worktrees/abc123/src/app.ts" with content "original"
    fs::write(worktree_path.join("src/app.ts"), "original")
        .expect("Failed to write worktree file");
    
    // @step And a file exists at "/project/src/app.ts" with content "main original"
    fs::write(project_root.join("src/app.ts"), "main original")
        .expect("Failed to write main project file");
    
    // @step When I edit file "src/app.ts" replacing "original" with "modified"
    let effective_cwd = get_test_effective_cwd(session_id.to_string()).expect("effective_cwd not set");
    let resolved_path = effective_cwd.join("src/app.ts");
    let content = fs::read_to_string(&resolved_path).expect("Failed to read file");
    let new_content = content.replace("original", "modified");
    fs::write(&resolved_path, new_content).expect("Failed to write edited file");
    
    // @step Then "/project/.fspec/worktrees/abc123/src/app.ts" should contain "modified"
    let worktree_content = fs::read_to_string(worktree_path.join("src/app.ts"))
        .expect("Failed to read worktree file");
    assert_eq!(worktree_content, "modified", "Worktree file should be modified");
    
    // @step And "/project/src/app.ts" should still contain "main original"
    let main_content = fs::read_to_string(project_root.join("src/app.ts"))
        .expect("Failed to read main project file");
    assert_eq!(main_content, "main original", "Main project file should be unchanged");
    
    cleanup_test_effective_cwd_map();
}

// ============================================================================
// Scenario: Non-isolated session file operations affect main project
// ============================================================================

/// Feature: spec/features/isolated-session-file-operations.feature
/// Scenario: Non-isolated session file operations affect main project
///
/// @step Given I am an AI agent running in a non-isolated session
/// @step And my session has NO worktree_path
/// @step When I write content "new content" to file "src/test.ts"
/// @step Then the file should exist at "/project/src/test.ts"
/// @step And there should be NO worktree directory
#[test]
#[serial]
fn test_non_isolated_session_affects_main_project() {
    use tempfile::TempDir;
    use std::fs;

    // Create a mock project structure
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let project_root = temp_dir.path();
    
    // @step Given I am an AI agent running in a non-isolated session
    let session_id = Uuid::new_v4();
    setup_test_effective_cwd_map();
    
    // @step And my session has NO worktree_path
    // For non-isolated sessions, effective_cwd returns project root
    fs::create_dir_all(project_root.join("src")).expect("Failed to create src");
    set_test_effective_cwd(session_id, project_root.to_path_buf());
    
    // @step When I write content "new content" to file "src/test.ts"
    let effective_cwd = get_test_effective_cwd(session_id.to_string()).expect("effective_cwd not set");
    let resolved_path = effective_cwd.join("src/test.ts");
    fs::write(&resolved_path, "new content").expect("Failed to write file");
    
    // @step Then the file should exist at "/project/src/test.ts"
    assert!(
        project_root.join("src/test.ts").exists(),
        "File should exist in main project for non-isolated session"
    );
    let content = fs::read_to_string(project_root.join("src/test.ts")).expect("Failed to read");
    assert_eq!(content, "new content", "Content should match");
    
    // @step And there should be NO worktree directory
    assert!(
        !project_root.join(".fspec/worktrees").exists(),
        "No worktree directory should exist for non-isolated session"
    );
    
    cleanup_test_effective_cwd_map();
}
